import { invoke } from "@tauri-apps/api/core";

export interface GithubStatus {
  connected: boolean;
  login: string | null;
}

/** Mirrors the Rust `RepoSummary`. */
export interface RepoSummary {
  full_name: string;
  name: string;
  clone_url: string;
  private: boolean;
  default_branch: string;
  description: string | null;
}

export function githubStatus(): Promise<GithubStatus> {
  return invoke<GithubStatus>("github_status");
}

/** Connect by importing the token from the authenticated gh CLI. */
export function githubConnectGh(): Promise<GithubStatus> {
  return invoke<GithubStatus>("github_connect_gh");
}

export function githubSignOut(): Promise<void> {
  return invoke("github_sign_out");
}

export function githubListRepos(): Promise<RepoSummary[]> {
  return invoke<RepoSummary[]>("github_list_repos");
}
