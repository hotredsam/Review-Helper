import { create } from "zustand";
import {
  githubStatus,
  githubConnectGh,
  githubSignOut,
  githubListRepos,
  type GithubStatus,
  type RepoSummary,
} from "../api/github";

interface GithubState {
  status: GithubStatus | null;
  repos: RepoSummary[];
  connecting: boolean;
  loadingRepos: boolean;
  error: string | null;
  refreshStatus: () => Promise<void>;
  connect: () => Promise<void>;
  signOut: () => Promise<void>;
  loadRepos: () => Promise<void>;
}

const disconnected: GithubStatus = { connected: false, login: null };

/** GitHub connection state, shared by the connect UI and the add-project paths. */
export const useGithubStore = create<GithubState>((set) => ({
  status: null,
  repos: [],
  connecting: false,
  loadingRepos: false,
  error: null,

  refreshStatus: async () => {
    try {
      set({ status: await githubStatus() });
    } catch (e) {
      set({ status: disconnected, error: String(e) });
    }
  },

  connect: async () => {
    set({ connecting: true, error: null });
    try {
      set({ status: await githubConnectGh(), connecting: false });
    } catch (e) {
      set({ connecting: false, error: String(e) });
    }
  },

  signOut: async () => {
    set({ error: null });
    try {
      await githubSignOut();
      set({ status: disconnected, repos: [] });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  loadRepos: async () => {
    set({ loadingRepos: true, error: null });
    try {
      set({ repos: await githubListRepos(), loadingRepos: false });
    } catch (e) {
      set({ loadingRepos: false, error: String(e) });
    }
  },
}));
