//! Local Whisper transcription for the Inbox mic (Phase 19): cpal microphone
//! capture → 16 kHz mono ring buffer → streaming partial transcripts every
//! ~1.5 s over a sliding window → one clean full pass on stop. All inference is
//! whisper.cpp (Metal) via whisper-rs; nothing leaves the machine.
//!
//! Model: ggml-large-v3-turbo-q5_0 (~547 MB download, ~1.3 GB inference RAM —
//! better accuracy than small.en, still coexists with Ollama on an 11.8 GB
//! machine). Auto-downloaded to app-data on first use, sha256-verified; the
//! context is dropped after each session so the RAM is only held while the mic
//! is actually in use.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use sha2::Digest;
use tauri::{AppHandle, Emitter, Manager};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub mod commands;

const MODEL_FILE: &str = "ggml-large-v3-turbo-q5_0.bin";
const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin";
const MODEL_SHA256: &str = "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2";
const MODEL_BYTES: u64 = 574_041_195;

const SAMPLE_RATE: usize = 16_000;
/// Sliding window fed to each partial pass — enough context for coherent text,
/// small enough to stay well under the ~1.5 s cadence on an M-series GPU.
const PARTIAL_WINDOW_SECS: usize = 12;
// Measured on the M4 (probe test): one pass ≈ 1.7 s regardless of window
// size (whisper pads to a fixed 30 s encoder frame), so an 800 ms wait puts a
// fresh partial on screen roughly every 2.5 s.
const PARTIAL_CADENCE: Duration = Duration::from_millis(800);
/// Hard cap on one dictation (10 minutes at 16 kHz mono f32 ≈ 38 MB).
const MAX_SECS: usize = 600;

#[derive(Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TranscribeEvent {
    /// loading_model | downloading | recording | transcribing
    State { state: String },
    ModelDownload { done: u64, total: u64 },
    Partial { text: String },
    Final { text: String },
    Error { detail: String },
}

fn emit(app: &AppHandle, ev: TranscribeEvent) {
    let _ = app.emit("transcribe-event", &ev);
}

// ---- model management ----

/// Where the model may already live: app-data first, then the dev cache (a
/// pre-downloaded copy on the dev machine skips the 547 MB download).
fn model_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(dir) = app.path().app_data_dir() {
        out.push(dir.join("models").join(MODEL_FILE));
    }
    if let Some(home) = std::env::var_os("HOME") {
        out.push(PathBuf::from(home).join("Library/Caches/review-helper-models").join(MODEL_FILE));
    }
    out
}

fn sha256_of(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Find or download the model, verifying integrity. Emits download progress.
pub fn ensure_model(app: &AppHandle) -> Result<PathBuf, String> {
    for cand in model_candidates(app) {
        if cand.is_file() {
            let size = std::fs::metadata(&cand).map(|m| m.len()).unwrap_or(0);
            if size == MODEL_BYTES {
                return Ok(cand);
            }
            // Wrong size = a truncated download; fall through and re-fetch.
        }
    }

    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("models");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(MODEL_FILE);
    let part = dir.join(format!("{MODEL_FILE}.part"));

    emit(app, TranscribeEvent::State { state: "downloading".into() });
    let resp = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(1800))
        .build()
        .map_err(|e| e.to_string())?
        .get(MODEL_URL)
        .send()
        .map_err(|e| format!("Couldn't download the speech model: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Couldn't download the speech model (HTTP {}).", resp.status()));
    }

    let mut reader = resp;
    let mut out = std::fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut done: u64 = 0;
    let mut last_emit = Instant::now();
    let mut buf = [0u8; 1 << 16];
    loop {
        let n = std::io::Read::read(&mut reader, &mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut out, &buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if last_emit.elapsed() > Duration::from_millis(300) {
            emit(app, TranscribeEvent::ModelDownload { done, total: MODEL_BYTES });
            last_emit = Instant::now();
        }
    }
    drop(out);

    if sha256_of(&part)? != MODEL_SHA256 {
        let _ = std::fs::remove_file(&part);
        return Err("The downloaded speech model failed verification. Try again.".into());
    }
    std::fs::rename(&part, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

// ---- session plumbing ----

enum Control {
    /// Stop recording and run the final pass; reply on the channel.
    Stop(Sender<Result<String, String>>),
    /// Discard everything.
    Cancel,
}

struct Session {
    control: Sender<Control>,
}

static SESSION: OnceLock<Mutex<Option<Session>>> = OnceLock::new();

fn session_slot() -> &'static Mutex<Option<Session>> {
    SESSION.get_or_init(|| Mutex::new(None))
}

pub fn is_recording() -> bool {
    session_slot().lock().map(|s| s.is_some()).unwrap_or(false)
}

/// Linear resampler to 16 kHz mono — accurate enough for speech, no extra dep.
fn resample_to_16k(input: &[f32], in_rate: u32, channels: usize) -> Vec<f32> {
    if input.is_empty() || channels == 0 {
        return Vec::new();
    }
    let mono: Vec<f32> = input
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect();
    if in_rate as usize == SAMPLE_RATE {
        return mono;
    }
    let ratio = in_rate as f64 / SAMPLE_RATE as f64;
    let out_len = (mono.len() as f64 / ratio) as usize;
    (0..out_len)
        .map(|i| {
            let pos = i as f64 * ratio;
            let i0 = pos as usize;
            let frac = (pos - i0 as f64) as f32;
            let a = mono.get(i0).copied().unwrap_or(0.0);
            let b = mono.get(i0 + 1).copied().unwrap_or(a);
            a + (b - a) * frac
        })
        .collect()
}

fn run_inference(ctx: &WhisperContext, samples: &[f32]) -> Result<String, String> {
    // Whisper needs at least ~1s of audio to say anything useful.
    if samples.len() < SAMPLE_RATE {
        return Ok(String::new());
    }
    let mut state = ctx.create_state().map_err(|e| e.to_string())?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);
    params.set_no_context(true);
    state.full(params, samples).map_err(|e| e.to_string())?;
    let mut text = String::new();
    for i in 0..state.full_n_segments() {
        if let Some(segment) = state.get_segment(i) {
            if let Ok(seg) = segment.to_str() {
                text.push_str(seg);
            }
        }
    }
    Ok(text.trim().to_string())
}

/// Start a recording session. The session owns its thread: cpal stream (audio
/// callback pushes into a shared buffer), a partial-inference loop at a fixed
/// cadence, and the final pass on Stop. Returns once recording has begun.
pub fn start(app: AppHandle) -> Result<(), String> {
    {
        let slot = session_slot().lock().unwrap_or_else(|p| p.into_inner());
        if slot.is_some() {
            return Err("Already recording.".into());
        }
    }

    emit(&app, TranscribeEvent::State { state: "loading_model".into() });
    let model_path = ensure_model(&app)?;

    let (control_tx, control_rx) = std::sync::mpsc::channel::<Control>();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();

    let thread_app = app.clone();
    std::thread::spawn(move || session_thread(thread_app, model_path, control_rx, ready_tx));

    // Surface setup failures (no mic permission, no input device, bad model)
    // to the caller instead of a silent dead button.
    match ready_rx.recv_timeout(Duration::from_secs(60)) {
        Ok(Ok(())) => {
            let mut slot = session_slot().lock().unwrap_or_else(|p| p.into_inner());
            *slot = Some(Session { control: control_tx });
            Ok(())
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err("The recorder didn't start in time.".into()),
    }
}

pub fn stop() -> Result<String, String> {
    let control = {
        let mut slot = session_slot().lock().unwrap_or_else(|p| p.into_inner());
        slot.take().ok_or("Not recording.")?.control
    };
    let (tx, rx): (Sender<Result<String, String>>, Receiver<Result<String, String>>) =
        std::sync::mpsc::channel();
    control.send(Control::Stop(tx)).map_err(|_| "The recorder already exited.")?;
    rx.recv_timeout(Duration::from_secs(120))
        .map_err(|_| "Transcription timed out.".to_string())?
}

pub fn cancel() -> Result<(), String> {
    let mut slot = session_slot().lock().unwrap_or_else(|p| p.into_inner());
    if let Some(s) = slot.take() {
        let _ = s.control.send(Control::Cancel);
    }
    Ok(())
}

fn session_thread(
    app: AppHandle,
    model_path: PathBuf,
    control: Receiver<Control>,
    ready: Sender<Result<(), String>>,
) {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    // Load the model first (a few seconds; ~1.3 GB until the session ends).
    let ctx = match WhisperContext::new_with_params(
        model_path.to_string_lossy().as_ref(),
        WhisperContextParameters::default(),
    ) {
        Ok(c) => c,
        Err(e) => {
            let _ = ready.send(Err(format!("Couldn't load the speech model: {e}")));
            return;
        }
    };

    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => {
            let _ = ready.send(Err(
                "No microphone found. Check System Settings → Privacy & Security → Microphone.".into(),
            ));
            return;
        }
    };
    let config = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
            let _ = ready.send(Err(format!("Couldn't read the microphone's format: {e}")));
            return;
        }
    };
    let in_rate = config.sample_rate();
    let channels = config.channels() as usize;
    let stream_config: cpal::StreamConfig = config.into();

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let writer = samples.clone();
    let overflow = Arc::new(AtomicBool::new(false));
    let overflow_w = overflow.clone();

    let err_app = app.clone();
    let stream = match device.build_input_stream(
        stream_config,
        move |data: &[f32], _| {
            let chunk = resample_to_16k(data, in_rate, channels);
            if let Ok(mut buf) = writer.lock() {
                if buf.len() < SAMPLE_RATE * MAX_SECS {
                    buf.extend_from_slice(&chunk);
                } else {
                    overflow_w.store(true, Ordering::SeqCst);
                }
            }
        },
        move |e| {
            emit(&err_app, TranscribeEvent::Error { detail: format!("Microphone error: {e}") });
        },
        None,
    ) {
        Ok(s) => s,
        Err(e) => {
            let _ = ready.send(Err(format!(
                "Couldn't open the microphone ({e}). If this is the first use, grant mic access and try again."
            )));
            return;
        }
    };
    if let Err(e) = stream.play() {
        let _ = ready.send(Err(format!("Couldn't start the microphone: {e}")));
        return;
    }
    let _ = ready.send(Ok(()));
    emit(&app, TranscribeEvent::State { state: "recording".into() });

    // Partial loop: transcribe the last PARTIAL_WINDOW_SECS at a fixed cadence.
    let mut last_partial = String::new();
    loop {
        match control.recv_timeout(PARTIAL_CADENCE) {
            Ok(Control::Stop(reply)) => {
                drop(stream); // stop capture before the (possibly long) final pass
                emit(&app, TranscribeEvent::State { state: "transcribing".into() });
                let all = samples.lock().map(|b| b.clone()).unwrap_or_default();
                let mut result = run_inference(&ctx, &all);
                if overflow.load(Ordering::SeqCst) {
                    if let Ok(t) = &mut result {
                        t.push_str("\n…(recording capped at 10 minutes)");
                    }
                }
                if let Ok(t) = &result {
                    emit(&app, TranscribeEvent::Final { text: t.clone() });
                }
                let _ = reply.send(result);
                return;
            }
            Ok(Control::Cancel) => {
                drop(stream);
                return;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                let window = {
                    let buf = samples.lock().map(|b| b.clone()).unwrap_or_default();
                    let max = SAMPLE_RATE * PARTIAL_WINDOW_SECS;
                    if buf.len() > max { buf[buf.len() - max..].to_vec() } else { buf }
                };
                match run_inference(&ctx, &window) {
                    Ok(text) => {
                        if !text.is_empty() && text != last_partial {
                            last_partial = text.clone();
                            emit(&app, TranscribeEvent::Partial { text });
                        }
                    }
                    Err(e) => emit(&app, TranscribeEvent::Error { detail: e }),
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                drop(stream);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_halves_a_32k_signal_and_downmixes_stereo() {
        // 2 channels @ 32 kHz → mono @ 16 kHz: half the frames, averaged channels.
        let frames = 32_000; // one second
        let mut input = Vec::with_capacity(frames * 2);
        for _ in 0..frames {
            input.push(0.5);
            input.push(-0.5); // averages to 0.0
        }
        let out = resample_to_16k(&input, 32_000, 2);
        assert!((out.len() as i64 - 16_000).abs() < 4, "got {}", out.len());
        assert!(out.iter().all(|s| s.abs() < 1e-6));
    }

    #[test]
    fn resample_passthrough_at_16k_mono() {
        let input = vec![0.1f32, 0.2, 0.3];
        assert_eq!(resample_to_16k(&input, 16_000, 1), input);
    }

    #[test]
    #[ignore = "loads the 547 MB local model and runs real inference; run: cargo test -- --ignored"]
    fn real_model_transcribes_synthetic_speech() {
        // Hardware fit/speed check (Phase 19 T4): transcribe a `say`-generated
        // WAV and report wall time. Run on the dev machine, not CI.
        let wav = std::env::temp_dir().join("rh-whisper-probe.wav");
        let aiff = std::env::temp_dir().join("rh-whisper-probe.aiff");
        std::process::Command::new("say")
            .args(["-o", aiff.to_str().unwrap(), "The quick brown fox jumps over the lazy dog. This is a hardware speed test for Review Helper."])
            .status()
            .expect("say");
        std::process::Command::new("afconvert")
            .args(["-f", "WAVE", "-d", "LEI16@16000", "-c", "1", aiff.to_str().unwrap(), wav.to_str().unwrap()])
            .status()
            .expect("afconvert");
        let mut reader = hound_lite_read(&wav);
        let home = std::env::var("HOME").unwrap();
        let model = PathBuf::from(home).join("Library/Caches/review-helper-models").join(MODEL_FILE);
        let ctx = WhisperContext::new_with_params(model.to_string_lossy().as_ref(), WhisperContextParameters::default()).unwrap();
        let started = Instant::now();
        let text = run_inference(&ctx, &reader).unwrap();
        let secs = started.elapsed().as_secs_f64();
        let audio_secs = reader.len() as f64 / SAMPLE_RATE as f64;
        println!("whisper-probe: {audio_secs:.1}s audio in {secs:.2}s ({:.1}x realtime): {text}", audio_secs / secs);
        assert!(text.to_lowercase().contains("quick brown fox"), "got: {text}");
        reader.clear();
    }

    /// Minimal 16-bit PCM WAV reader for the probe (avoids a hound dependency).
    #[cfg(test)]
    fn hound_lite_read(path: &Path) -> Vec<f32> {
        let bytes = std::fs::read(path).unwrap();
        let data_pos = bytes.windows(4).position(|w| w == b"data").unwrap() + 8;
        bytes[data_pos..]
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
            .collect()
    }
}
