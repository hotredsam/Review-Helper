import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface DimScore {
  score: number;
  reason: string;
}

export interface AssessmentView {
  overall: number;
  dimensions: Record<string, DimScore>;
  production: { scores: Record<string, DimScore>; overall: number };
  top_fixes: string[];
  hygiene: string[];
  created_at: string;
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
