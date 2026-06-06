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
