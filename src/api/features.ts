import { invoke } from "@tauri-apps/api/core";

export interface Feature {
  id: number;
  title: string;
  detail: string | null;
  source: string | null;
  status: string;
  created_at: string;
}

export function featuresList(id: number): Promise<Feature[]> {
  return invoke<Feature[]>("features_list", { projectId: id });
}
export function featureAdd(id: number, title: string, detail?: string, source?: string): Promise<Feature> {
  return invoke<Feature>("feature_add", { projectId: id, title, detail: detail ?? null, source: source ?? null });
}
export function featureSetStatus(id: number, featureId: number, status: string): Promise<void> {
  return invoke("feature_set_status", { projectId: id, featureId, status });
}
export function featuresPendingCount(id: number): Promise<number> {
  return invoke<number>("features_pending_count", { projectId: id });
}
