import { invoke } from "@tauri-apps/api/core";

/** Mirrors the Rust `Project` struct (the `projects` table row). */
export interface Project {
  id: number;
  name: string;
  kind: "imported" | "new";
  app_type: string | null;
  github_repo_url: string | null;
  clone_path: string | null;
  default_branch: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * Typed wrappers over the Rust project commands. All disk/DB access happens in
 * Rust behind these named commands — the frontend only ever calls invoke.
 * Tauri maps camelCase JS args (appType) to snake_case Rust params (app_type).
 */
export function createProject(
  name: string,
  kind: Project["kind"],
  appType?: string,
): Promise<Project> {
  return invoke<Project>("create_project", {
    name,
    kind,
    appType: appType ?? null,
  });
}

export function listProjects(): Promise<Project[]> {
  return invoke<Project[]>("list_projects");
}

export function getProject(id: number): Promise<Project | null> {
  return invoke<Project | null>("get_project", { id });
}

export function renameProject(id: number, name: string): Promise<Project> {
  return invoke<Project>("rename_project", { id, name });
}

export function deleteProject(id: number): Promise<boolean> {
  return invoke<boolean>("delete_project", { id });
}

// ---- GitHub-attached add-project paths ----

export function importRepo(
  fullName: string,
  cloneUrl: string,
  defaultBranch: string,
): Promise<Project> {
  return invoke<Project>("project_import_repo", { fullName, cloneUrl, defaultBranch });
}

export function linkRepoByUrl(url: string): Promise<Project> {
  return invoke<Project>("project_link_url", { url });
}

export function createRepoProject(name: string, isPrivate: boolean): Promise<Project> {
  return invoke<Project>("project_create_repo", { name, private: isPrivate });
}
