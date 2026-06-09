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
