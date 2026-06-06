import { invoke } from "@tauri-apps/api/core";

export interface Suggestion {
  id: number;
  kind: string;
  payload: Record<string, unknown> | null;
  status: string;
  created_at: string;
}

/** List a project's suggestions, optionally filtered by status (e.g. "pending"). */
export function suggestionsList(id: number, status?: string): Promise<Suggestion[]> {
  return invoke<Suggestion[]>("suggestions_list", { projectId: id, status: status ?? null });
}

export function suggestionApprove(id: number, suggestionId: number): Promise<void> {
  return invoke("suggestion_approve", { projectId: id, suggestionId });
}

export function suggestionDismiss(id: number, suggestionId: number): Promise<void> {
  return invoke("suggestion_dismiss", { projectId: id, suggestionId });
}

export function suggestionsApproveAll(id: number): Promise<number> {
  return invoke<number>("suggestions_approve_all", { projectId: id });
}

/** A short human label for a suggestion, by kind. */
export function summarizeSuggestion(s: Suggestion): string {
  const p = (s.payload ?? {}) as Record<string, string>;
  switch (s.kind) {
    case "decision":
      return `${p.topic ?? "Decision"}: ${p.choice ?? ""}`;
    case "feature":
      return p.title ?? "Feature";
    case "stack":
      return `${p.pane ?? "Stack"}: ${p.choice ?? ""}`;
    case "answer":
      return p.question ?? "Answer";
    default:
      return s.kind;
  }
}
