import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { ProviderKind } from "./settings";

export type UnavailableReason =
  | "not_installed"
  | "not_authenticated"
  | "credit_exhausted"
  | "unknown";

/** Mirrors the Rust `ModelEvent` enum (serde tag = "type", snake_case). */
export type ModelEvent =
  | { type: "started"; session_id: string | null; model: string | null }
  | { type: "assistant_text"; text: string }
  | { type: "tool_use"; name: string }
  | { type: "notice"; message: string }
  | { type: "completed"; session_id: string | null; text: string }
  | { type: "unavailable"; reason: UnavailableReason; detail: string }
  | { type: "failed"; detail: string };

/** Start a model run. Events arrive asynchronously via `onModelEvent`. */
export function runModel(prompt: string, sessionId?: string | null): Promise<void> {
  return invoke("model_run", { prompt, sessionId: sessionId ?? null });
}

/** Subscribe to streamed model events. Returns an unlisten function. */
export function onModelEvent(handler: (e: ModelEvent) => void): Promise<UnlistenFn> {
  return listen<ModelEvent>("model-event", (evt) => handler(evt.payload));
}

/** Mirrors the Rust `ModelStatus` — whether the active provider is usable, with
 *  debug detail (probe command, exit code, stderr). */
export interface ModelStatus {
  provider: ProviderKind;
  available: boolean;
  version: string | null;
  reason: UnavailableReason | null;
  command: string;
  exit_code: number | null;
  stderr: string;
}

export function getModelStatus(): Promise<ModelStatus> {
  return invoke<ModelStatus>("model_status");
}
