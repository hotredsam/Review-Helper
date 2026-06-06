import { describe, it, expect, beforeEach, vi } from "vitest";

const ctrl = vi.hoisted(() => ({
  status: { connected: false, login: null } as any,
  repos: [] as any[],
  connectError: null as string | null,
}));

vi.mock("../api/github", () => ({
  githubStatus: vi.fn(async () => ctrl.status),
  githubConnectGh: vi.fn(async () => {
    if (ctrl.connectError) throw ctrl.connectError;
    ctrl.status = { connected: true, login: "hotredsam" };
    return ctrl.status;
  }),
  githubSignOut: vi.fn(async () => {
    ctrl.status = { connected: false, login: null };
  }),
  githubListRepos: vi.fn(async () => ctrl.repos),
}));

import { useGithubStore } from "../store/githubStore";

beforeEach(() => {
  ctrl.status = { connected: false, login: null };
  ctrl.repos = [];
  ctrl.connectError = null;
  useGithubStore.setState({
    status: null,
    repos: [],
    connecting: false,
    loadingRepos: false,
    error: null,
  });
});

describe("githubStore", () => {
  it("connects, lists repos, and signs out", async () => {
    await useGithubStore.getState().refreshStatus();
    expect(useGithubStore.getState().status).toEqual({ connected: false, login: null });

    await useGithubStore.getState().connect();
    expect(useGithubStore.getState().status).toEqual({ connected: true, login: "hotredsam" });

    ctrl.repos = [
      { full_name: "hotredsam/Review-Helper", name: "Review-Helper", clone_url: "u", private: false, default_branch: "main", description: null },
    ];
    await useGithubStore.getState().loadRepos();
    expect(useGithubStore.getState().repos).toHaveLength(1);

    await useGithubStore.getState().signOut();
    expect(useGithubStore.getState().status).toEqual({ connected: false, login: null });
    expect(useGithubStore.getState().repos).toHaveLength(0);
  });

  it("surfaces a connect error and stays disconnected", async () => {
    ctrl.connectError = "`gh` is not signed in. Run `gh auth login` first.";
    await useGithubStore.getState().connect();
    const s = useGithubStore.getState();
    expect(s.connecting).toBe(false);
    expect(s.error).toMatch(/gh auth login/);
    expect(s.status?.connected ?? false).toBe(false);
  });
});
