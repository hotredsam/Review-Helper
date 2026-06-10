import { invoke } from "@tauri-apps/api/core";

export interface PackageFile {
  path: string;
  content: string;
}

export type IssueAction =
  | { kind: "create"; marker: string; title: string; body: string; state: string; labels: string[] }
  | { kind: "update"; number: number; marker: string; title: string; body: string; state: string; labels: string[] }
  | { kind: "close"; number: number; title: string };

export interface SyncPreview {
  /** The project this preview was computed for; the backend refuses a mismatch. */
  project_id: number;
  issue_actions: IssueAction[];
  file_deletions: string[];
}

export interface SyncResult {
  files_pushed: number;
  issues_applied: number;
  files_deleted: number;
  failures: string[];
}

export function syncPackage(id: number): Promise<PackageFile[]> {
  return invoke<PackageFile[]>("sync_package", { projectId: id });
}
export function syncPushPlanning(id: number): Promise<number> {
  return invoke<number>("sync_push_planning", { projectId: id });
}
export function syncMainPreview(id: number): Promise<SyncPreview> {
  return invoke<SyncPreview>("sync_main_preview", { projectId: id });
}
/** Apply the CONFIRMED preview (passed back verbatim). */
export function syncMainApply(id: number, preview: SyncPreview): Promise<SyncResult> {
  return invoke<SyncResult>("sync_main_apply", { projectId: id, preview });
}
