import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface UiSpec {
  field: "single_choice" | "multi_choice" | "scale" | "short_text" | "long_text";
  options?: string[];
  min?: number;
  max?: number;
  min_label?: string;
  max_label?: string;
}

export interface Question {
  id: number;
  dimension: string | null;
  bank_topic: string | null;
  text: string;
  recommended_answer: string | null;
  ui_spec?: UiSpec | null;
  status: string;
  /** Docs-grounded mode: the official-doc URL this question cites. */
  doc_ref?: string | null;
}

export type GrillEvent =
  | { type: "started"; project_id: number }
  | { type: "tool"; project_id: number; name: string }
  | { type: "done"; project_id: number; added: number }
  | { type: "failed"; project_id: number; detail: string };

/** Generate a batch of repo-specific questions, scaled by depth (1–5). */
export function grillGenerate(id: number, depth: number, withDocs = false): Promise<void> {
  return invoke("grill_generate", { projectId: id, depth, withDocs });
}

export function grillList(id: number): Promise<Question[]> {
  return invoke<Question[]>("grill_list", { projectId: id });
}

export function grillAnswer(id: number, questionId: number, body: string): Promise<void> {
  return invoke("grill_answer", { projectId: id, questionId, body });
}

/** Write a "Let's chat about this" resolution back into the card. */
export function grillChatResolve(id: number, questionId: number, resolution: string): Promise<void> {
  return invoke("grill_chat_resolve", { projectId: id, questionId, resolution });
}

export function grillSetStatus(id: number, questionId: number, status: string): Promise<void> {
  return invoke("grill_set_status", { projectId: id, questionId, status });
}

export function grillDelete(id: number, questionId: number): Promise<void> {
  return invoke("grill_delete", { projectId: id, questionId });
}

export function onGrillEvent(cb: (e: GrillEvent) => void): Promise<UnlistenFn> {
  return listen<GrillEvent>("grill-event", (e) => cb(e.payload));
}
