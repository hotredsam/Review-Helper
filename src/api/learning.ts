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

// ---- L3/L4: study materials + the adaptive engine ----

export interface Flashcard {
  id: number;
  front: string;
  back: string;
  due: string | null;
  reps: number;
}

export interface QuizQuestion {
  id: number;
  question: string;
  options: string[];
  answer_idx: number;
  explanation: string | null;
}

export interface QuizResult {
  correct: boolean;
  answer_idx: number;
  explanation: string | null;
  p_known: number;
}

export interface SkillMastery {
  skill: string;
  p_known: number;
  n_obs: number;
}

export interface ProfileSnapshot {
  attempts: number;
  correct: number;
  accuracy: number;
  flashcard_reviews: number;
  avg_latency_ms: number;
  skills: SkillMastery[];
}

/** A module's notes (markdown), generated + cached on first open. */
export function learningNotes(moduleId: number): Promise<string> {
  return invoke<string>("learning_notes", { moduleId });
}

export function learningFlashcards(moduleId: number): Promise<Flashcard[]> {
  return invoke<Flashcard[]>("learning_flashcards", { moduleId });
}

export function learningQuiz(moduleId: number): Promise<QuizQuestion[]> {
  return invoke<QuizQuestion[]>("learning_quiz", { moduleId });
}

/** Grade a flashcard (1=Again, 2=Hard, 3=Good, 4=Easy); returns next due date. */
export function learningFlashcardGrade(flashcardId: number, rating: 1 | 2 | 3 | 4): Promise<string> {
  return invoke<string>("learning_flashcard_grade", { flashcardId, rating });
}

/** Submit a quiz answer; returns correctness + the right answer + explanation. */
export function learningQuizAnswer(
  questionId: number,
  choiceIdx: number,
  latencyMs?: number,
): Promise<QuizResult> {
  return invoke<QuizResult>("learning_quiz_answer", { questionId, choiceIdx, latencyMs: latencyMs ?? null });
}

/** The learner profile (pace + per-skill mastery) for the progress view. */
export function learningProgress(subjectId: number): Promise<ProfileSnapshot> {
  return invoke<ProfileSnapshot>("learning_progress", { subjectId });
}
