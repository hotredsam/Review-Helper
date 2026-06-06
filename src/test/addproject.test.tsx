import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const proj = (over: any = {}) => ({
  id: 1,
  name: "Repo",
  kind: "imported",
  app_type: null,
  github_repo_url: "u",
  clone_path: null,
  default_branch: "main",
  created_at: "t",
  updated_at: "t",
  ...over,
});

vi.mock("../api/projects", () => ({
  listProjects: vi.fn(async () => []),
  createProject: vi.fn(async (name: string, kind: string) => proj({ id: 2, name, kind })),
  renameProject: vi.fn(),
  deleteProject: vi.fn(),
  importRepo: vi.fn(async () => proj({ id: 3, name: "Imported" })),
  linkRepoByUrl: vi.fn(async () => proj({ id: 4, name: "Linked" })),
  createRepoProject: vi.fn(async (name: string) => proj({ id: 5, name, kind: "new" })),
}));

import { useProjectStore } from "../store/projectStore";
import { useGithubStore } from "../store/githubStore";
import { NewProjectDialog } from "../components/NewProjectDialog";

const repo = {
  full_name: "hotredsam/Review-Helper",
  name: "Review-Helper",
  clone_url: "u",
  private: true,
  default_branch: "main",
  description: null,
};

beforeEach(() => {
  useProjectStore.setState({ projects: [], activeProjectId: null, status: "ready", error: null });
  useGithubStore.setState({
    status: { connected: true, login: "hotredsam" },
    repos: [repo],
    connecting: false,
    loadingRepos: false,
    error: null,
  });
});

describe("projectStore GitHub paths", () => {
  it("importRepo / linkUrl / createRepo each add and activate a project", async () => {
    const a = await useProjectStore.getState().importRepo(repo);
    expect(a.id).toBe(3);
    expect(useProjectStore.getState().activeProjectId).toBe(3);

    const b = await useProjectStore.getState().linkUrl("https://github.com/o/r");
    expect(b.id).toBe(4);

    const c = await useProjectStore.getState().createRepo("new-repo", true);
    expect(c.kind).toBe("new");
    expect(useProjectStore.getState().projects).toHaveLength(3);
  });
});

describe("NewProjectDialog — four paths", () => {
  it("imports a repo from the list", async () => {
    const user = userEvent.setup();
    render(<NewProjectDialog open onClose={() => {}} />);
    await user.click(screen.getByRole("button", { name: "Import" }));
    await user.click(await screen.findByRole("button", { name: /hotredsam\/Review-Helper/ }));
    expect(useProjectStore.getState().activeProjectId).toBe(3);
  });

  it("creates a new GitHub repo with a private toggle", async () => {
    const user = userEvent.setup();
    render(<NewProjectDialog open onClose={() => {}} />);
    await user.click(screen.getByRole("button", { name: "GitHub" }));
    await user.type(screen.getByPlaceholderText(/my-new-repo/i), "fresh");
    expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(true);
    await user.click(screen.getByRole("button", { name: "Create on GitHub" }));
    expect(useProjectStore.getState().projects.some((p) => p.name === "fresh")).toBe(true);
  });
});
