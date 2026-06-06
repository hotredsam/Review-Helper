import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface TaskView {
  id: number;
  idx: number;
  title: string;
  body_md: string | null;
  verification: string | null;
  status: string;
}
export interface PhaseView {
  id: number;
  idx: number;
  title: string;
  goal: string | null;
  status: string;
  tasks: TaskView[];
}
export interface DecisionView {
  topic: string;
  choice: string;
  rationale: string | null;
}
export interface StackView {
  pane: string;
  choice: string | null;
}
export interface PlanView {
  version: number;
  current_state: string | null;
  body_md: string | null;
  phases: PhaseView[];
  decisions: DecisionView[];
  stack: StackView[];
}

export type AnalysisEvent =
  | { type: "started"; project_id: number }
  | { type: "tool"; project_id: number; name: string }
  | { type: "done"; project_id: number; version: number; confidence: string; phases: number; source: string }
  | { type: "failed"; project_id: number; detail: string };

/** Kick off read-only analysis of the project's clone into a first plan. */
export function analyzeProject(id: number): Promise<void> {
  return invoke("analyze_project", { projectId: id });
}

/** Seed a blank project's plan from a free-text description (T3). */
export function kickoffProject(id: number, description: string): Promise<void> {
  return invoke("kickoff_project", { projectId: id, description });
}

/** Incrementally update the plan: weave answers + inbox features into a new
 *  version, preserving completed phases (emits analysis-event). */
export function updateProject(id: number): Promise<void> {
  return invoke("update_plan", { projectId: id });
}

export interface AuditEntry {
  version: number;
  source: string;
  at: string;
}

/** Rebuild the plan from scratch (warned in the UI; no status carry-over). */
export function rebuildProject(id: number): Promise<void> {
  return invoke("rebuild_plan", { projectId: id });
}

/** The plan audit trail: source → version mapping. */
export function auditList(id: number): Promise<AuditEntry[]> {
  return invoke<AuditEntry[]>("audit_list", { projectId: id });
}

export function getPlan(id: number): Promise<PlanView | null> {
  return invoke<PlanView | null>("get_plan", { projectId: id });
}

export function onAnalysisEvent(cb: (e: AnalysisEvent) => void): Promise<UnlistenFn> {
  return listen<AnalysisEvent>("analysis-event", (e) => cb(e.payload));
}
