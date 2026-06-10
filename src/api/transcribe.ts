import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type TranscribeEvent =
  | { type: "state"; state: "loading_model" | "downloading" | "recording" | "transcribing" }
  | { type: "model_download"; done: number; total: number }
  | { type: "partial"; text: string }
  | { type: "final"; text: string }
  | { type: "error"; detail: string };

/** Start recording (downloads/loads the local Whisper model on first use). */
export function transcribeStart(): Promise<void> {
  return invoke("transcribe_start");
}

/** Stop recording; resolves with the final, clean transcript. */
export function transcribeStop(): Promise<string> {
  return invoke<string>("transcribe_stop");
}

/** Discard the recording without transcribing. */
export function transcribeCancel(): Promise<void> {
  return invoke("transcribe_cancel");
}

export function onTranscribeEvent(cb: (e: TranscribeEvent) => void): Promise<UnlistenFn> {
  return listen<TranscribeEvent>("transcribe-event", (e) => cb(e.payload));
}
