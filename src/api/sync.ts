import { invoke } from "@tauri-apps/api/core";

export interface PackageFile {
  path: string;
  content: string;
}

export type IssueAction =
  | { kind: "create"; marker: string; title: string; body: string; state: string; label: string }
  | { kind: "update"; number: number; marker: string; title: string; body: string; state: string; label: string }
  | { kind: "close"; number: number; title: string };

export function syncPackage(id: number): Promise<PackageFile[]> {
  return invoke<PackageFile[]>("sync_package", { projectId: id });
}
export function syncPushPlanning(id: number): Promise<number> {
  return invoke<number>("sync_push_planning", { projectId: id });
}
export function syncIssuePreview(id: number): Promise<IssueAction[]> {
  return invoke<IssueAction[]>("sync_issue_preview", { projectId: id });
}
export function syncIssueApply(id: number): Promise<number> {
  return invoke<number>("sync_issue_apply", { projectId: id });
}
export function syncPushMain(id: number): Promise<number> {
  return invoke<number>("sync_push_main", { projectId: id });
}
