import { invoke } from "@tauri-apps/api/core";

export interface Decision {
  id: number;
  topic: string;
  choice: string;
  rationale: string | null;
  alternatives: string | null;
  consequences: string | null;
  source_ref: string | null;
  status: string;
  created_at: string;
}

export function decisionsList(id: number): Promise<Decision[]> {
  return invoke<Decision[]>("decisions_list", { projectId: id });
}

export function decisionSupersede(id: number, decisionId: number): Promise<void> {
  return invoke("decision_supersede", { projectId: id, decisionId });
}
