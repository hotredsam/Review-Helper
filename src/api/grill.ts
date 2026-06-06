import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Question {
  id: number;
  dimension: string | null;
  bank_topic: string | null;
  text: string;
  recommended_answer: string | null;
  status: string;
}

export type GrillEvent =
  | { type: "started"; project_id: number }
  | { type: "tool"; project_id: number; name: string }
  | { type: "done"; project_id: number; added: number }
  | { type: "failed"; project_id: number; detail: string };

/** Generate a batch of repo-specific questions, scaled by depth (1–5). */
export function grillGenerate(id: number, depth: number): Promise<void> {
  return invoke("grill_generate", { projectId: id, depth });
}

export function grillList(id: number): Promise<Question[]> {
  return invoke<Question[]>("grill_list", { projectId: id });
}

export function onGrillEvent(cb: (e: GrillEvent) => void): Promise<UnlistenFn> {
  return listen<GrillEvent>("grill-event", (e) => cb(e.payload));
}
