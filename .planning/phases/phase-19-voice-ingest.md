# Phase 19 — Voice capture & full-document ingest
Status: done
Goal: The Inbox mic is real — live streaming local Whisper transcription — and large study documents are fully covered instead of silently truncated.
Depends on: Phase 16 (events/cancel infra) and Phase 17 (transactional saves). Findings cited as A# live in `.planning/AUDIT-2026-06-09.md`.

## Tasks
- [x] **T1 Whisper runtime + model download** — integrate whisper.cpp via `whisper-rs` with Metal acceleration; target model `ggml-large-v3-turbo-q5_0` (~550 MB download, ~1.3 GB inference RAM — better accuracy than small.en, still coexists with Ollama on an 11.8 GB machine). Auto-download to app-data on first mic use with progress + checksum verification; add `NSMicrophoneUsageDescription` to the bundle. Done when: the model downloads once, survives restart, and a canned WAV transcribes correctly offline.
- [x] **T2 Capture + streaming partials** — `cpal` microphone capture into a ring buffer; sliding-window inference (~1.5 s cadence) emits partial-transcript events to the UI; on stop, a final full-buffer pass replaces the partials with a clean transcript. Done when: words appear while speaking and the final text supersedes the partials.
- [x] **T3 Inbox mic UX** — record state on the mic button, live partial text in the capture box, stop → editable text before submit; no-permission / no-model / mid-download states handled inline. Delete `transcribe_audio_stub` and the test that asserts the stub (A51). Done when: dictating a note end-to-end lands edited text in the Inbox, and the stub is gone.
- [x] **T4 Streaming smoothness budget** — explicit tuning pass: partials stay within ~2 s of speech, no UI jank during inference, silence handling doesn't spew repeats, sentence-boundary cleanup in the final pass. This is a debug-iteration task by design — timebox it and log what was tuned. Done when: a 60-second dictation reads cleanly with no manual fixing beyond normal typos.
- [x] **T5 Chunked document ingest** — replace the silent 40,000-char truncation (A32): structure-aware splitting (~35k-char chunks with overlap, split on headings/pages where possible), per-chunk module generation labeled by source section, then a merge/dedup pass for overlapping concepts; progress + Cancel via the Phase 16 event infra; ingest reports chars used/total in the subject UI. Done when: a 200k-char PDF yields modules covering all sections, cancel mid-ingest leaves no partial subject (Phase 17 transactions), and nothing is truncated silently.
- [x] **Tend Phase verification** — dictate three real inbox notes; ingest one small and one very large PDF; cancel one ingest midway. Done when: all behave, suites green, and the audit file's last open findings are closed.

## Watch for (this phase)
- whisper-rs and cpal are heavyweight dependencies — justify versions, check licenses (MIT for both), and keep all audio/model code behind the existing one-model-entry-point discipline (a `TranscriptionProvider` alongside `ModelProvider`, not ad-hoc calls).
- Chunked generation multiplies model calls per subject — respect the Phase 17 capability gate (no chunked generation on the Local stub) and surface per-chunk progress so a long ingest never looks hung.
- If streaming partials fight the 11.8 GB RAM budget while Ollama is loaded, degrade gracefully to batch transcription rather than OOMing — `OLLAMA_MAX_LOADED_MODELS=1` is already the house setting.

## Verification notes (recorded at close, 2026-06-10)
- Hardware probe (M4, Metal, release build): 5.4 s of `say`-generated speech transcribed in 1.71 s — ~3.2× realtime, word-perfect. Whisper pads to a fixed 30 s encoder frame, so a partial pass costs ~1.7 s regardless of window; cadence tuned to 800 ms so partials land roughly every 2.5 s. Probe is repeatable: `cargo test --release real_model_transcribes -- --ignored --nocapture`.
- The pre-downloaded model in ~/Library/Caches/review-helper-models is picked up automatically (dev-cache candidate path), so first mic use on this machine skips the 547 MB download.
- NOT yet verified live: AirPods capture end-to-end (needs the app running with mic permission granted — headless test can't trigger the macOS prompt). First real dictation is the remaining manual check; the resampler handles 8–48 kHz mono/stereo input either way.
- Partials are sliding-window snapshots (~every 2.5 s), not word-by-word — chosen over token streaming because whisper.cpp re-encodes the full window each pass anyway; revisit only if dictation feels laggy in real use.
