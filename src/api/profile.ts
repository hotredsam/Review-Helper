import { invoke } from "@tauri-apps/api/core";

export interface ProfileFile {
  name: string;
  content: string;
}

export interface ProfileStatus {
  enabled: boolean;
  unreflected_events: number;
  files: ProfileFile[];
}

export function profileGet(): Promise<ProfileStatus> {
  return invoke<ProfileStatus>("profile_get");
}

export function profileSetEnabled(enabled: boolean): Promise<void> {
  return invoke("profile_set_enabled", { enabled });
}

export function profileSaveNotes(name: string, notes: string): Promise<void> {
  return invoke("profile_save_notes", { name, notes });
}

export function profileReset(name: string): Promise<void> {
  return invoke("profile_reset", { name });
}

/** Fire at session boundaries; the backend gates on ≥15 new events. */
export function profileReflect(): Promise<string> {
  return invoke<string>("profile_reflect");
}
