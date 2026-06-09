import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type TextMode = "easy" | "technical";

export interface DimScore {
  score: number;
  reason: string;
  reason_technical?: string;
}

/** A fix / hygiene item: a {easy, technical} pair, or a legacy plain string. */
export type DualText = string | { easy?: string; technical?: string };

export interface AssessmentView {
  overall: number;
  dimensions: Record<string, DimScore>;
  production: { scores: Record<string, DimScore>; overall: number };
  top_fixes: DualText[];
  hygiene: DualText[];
  created_at: string;
}

/** A dimension's reason in the chosen register (falls back to the plain one). */
export function pickReason(d: DimScore | undefined, mode: TextMode): string {
  if (!d) return "";
  return mode === "technical" ? d.reason_technical ?? d.reason : d.reason;
}

/** A fix / hygiene item in the chosen register; tolerates legacy plain strings. */
export function pickText(item: DualText, mode: TextMode): string {
  if (typeof item === "string") return item;
  return (mode === "technical" ? item.technical : item.easy) ?? item.easy ?? item.technical ?? "";
}

export type AssessmentEvent =
  | { type: "started"; project_id: number }
  | { type: "tool"; project_id: number; name: string }
  | { type: "done"; project_id: number; overall: number }
  | { type: "failed"; project_id: number; detail: string };

export function assessProject(id: number): Promise<void> {
  return invoke("assess_project", { projectId: id });
}

export function getAssessment(id: number): Promise<AssessmentView | null> {
  return invoke<AssessmentView | null>("get_assessment", { projectId: id });
}

export function onAssessmentEvent(cb: (e: AssessmentEvent) => void): Promise<UnlistenFn> {
  return listen<AssessmentEvent>("assessment-event", (e) => cb(e.payload));
}
