import { invoke } from "@tauri-apps/api/core";

/** A subject the user is studying. Stage advances intake → proposed → ready. */
export interface Subject {
  id: number;
  title: string;
  source_kind: "describe" | "upload";
  stage: "intake" | "proposed" | "ready";
  created_at: string;
  updated_at: string;
}

export interface SubjectDetail {
  id: number;
  title: string;
  source_kind: "describe" | "upload";
  source_text: string | null;
  stage: "intake" | "proposed" | "ready";
}

/** Create a subject from a described goal or extracted upload text. */
export function subjectCreate(
  title: string,
  sourceKind: "describe" | "upload",
  sourceText: string,
): Promise<number> {
  return invoke<number>("subject_create", { title, sourceKind, sourceText });
}

export function subjectsList(): Promise<Subject[]> {
  return invoke<Subject[]>("subjects_list");
}

export function subjectGet(subjectId: number): Promise<SubjectDetail> {
  return invoke<SubjectDetail>("subject_get", { subjectId });
}

export function subjectDelete(subjectId: number): Promise<void> {
  return invoke<void>("subject_delete", { subjectId });
}

// ---- L1: intake grill (scope the subject before building materials) ----

export interface IntakeItem {
  id: number;
  idx: number;
  question: string;
  answer: string | null;
}

/** The subject's scoping questions, generated + cached on first call. */
export function learningIntake(subjectId: number): Promise<IntakeItem[]> {
  return invoke<IntakeItem[]>("learning_intake", { subjectId });
}

export function learningIntakeAnswer(intakeId: number, answer: string): Promise<void> {
  return invoke<void>("learning_intake_answer", { intakeId, answer });
}

// ---- L2: generative module proposal (the editable study plan) ----

export type ModuleKind = "notes" | "flashcards" | "quiz" | "tutor";

export interface ProposedModule {
  id: number;
  idx: number;
  kind: ModuleKind;
  title: string;
  summary: string | null;
  skill: string | null;
  included: boolean;
  status: "proposed" | "generating" | "ready" | "failed";
}

/** Propose a study plan from the scoping answers (cached; advances to proposed). */
export function learningPropose(subjectId: number): Promise<ProposedModule[]> {
  return invoke<ProposedModule[]>("learning_propose", { subjectId });
}

export function learningModules(subjectId: number): Promise<ProposedModule[]> {
  return invoke<ProposedModule[]>("learning_modules", { subjectId });
}

export function learningModuleSetIncluded(moduleId: number, included: boolean): Promise<void> {
  return invoke<void>("learning_module_set_included", { moduleId, included });
}

/** Lock in the edited plan and move to studying (needs ≥1 included module). */
export function learningConfirmPlan(subjectId: number): Promise<void> {
  return invoke<void>("learning_confirm_plan", { subjectId });
}
