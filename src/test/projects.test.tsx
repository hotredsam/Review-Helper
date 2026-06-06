import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// In-memory stand-in for the Rust command backend. Declared via vi.hoisted so
// it exists before the hoisted vi.mock factory runs.
const backend = vi.hoisted(() => ({ rows: [] as any[], nextId: 1 }));

// SettingsView mounts the ModelConsole, which subscribes to model events.
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string, args: any = {}) => {
    switch (cmd) {
      case "list_projects":
        return backend.rows.map((r) => ({ ...r }));
      case "create_project": {
        const name = String(args.name ?? "").trim();
        if (!name) throw "Project name cannot be empty.";
        if (args.kind !== "new" && args.kind !== "imported") throw "Invalid kind.";
        const row = {
          id: backend.nextId++,
          name,
          kind: args.kind,
          app_type: args.appType ?? null,
          github_repo_url: null,
          clone_path: null,
          default_branch: "main",
          created_at: "t",
          updated_at: "t",
        };
        backend.rows.push(row);
        return { ...row };
      }
      case "get_project":
        return backend.rows.find((r) => r.id === args.id) ?? null;
      case "rename_project": {
        const r = backend.rows.find((row) => row.id === args.id);
        if (!r) throw "No project.";
        r.name = String(args.name).trim();
        return { ...r };
      }
      case "delete_project": {
        const i = backend.rows.findIndex((r) => r.id === args.id);
        if (i >= 0) {
          backend.rows.splice(i, 1);
          return true;
        }
        return false;
      }
      case "github_status":
        return { connected: false, login: null };
      case "model_status":
        return {
          provider: "claude",
          available: true,
          version: "test",
          reason: null,
          command: "claude --version",
          exit_code: 0,
          stderr: "",
        };
      case "get_model_config":
        return { provider: "claude", local_endpoint: null, api_credit_overflow: false };
      case "set_model_config":
        return undefined;
      case "app_info":
        return { name: "Review Helper", version: "0.1.0" };
      default:
        throw new Error("unknown command: " + cmd);
    }
  }),
}));

import App from "../App";
import { useProjectStore } from "../store/projectStore";
import { useUiStore } from "../store/uiStore";
import { useThemeStore } from "../theme/themeStore";

beforeEach(() => {
  backend.rows = [];
  backend.nextId = 1;
  localStorage.clear();
  useProjectStore.setState({ projects: [], activeProjectId: null, status: "idle", error: null });
  useUiStore.setState({ sidebarCollapsed: false, activeSection: "overview" });
});

describe("projectStore data flow", () => {
  it("loads empty, then create persists and becomes active", async () => {
    await useProjectStore.getState().load();
    expect(useProjectStore.getState().projects).toHaveLength(0);

    await useProjectStore.getState().create("Alpha", "new");
    const beta = await useProjectStore.getState().create("Beta", "imported");

    const s = useProjectStore.getState();
    expect(s.projects.map((p) => p.name)).toEqual(["Alpha", "Beta"]);
    expect(s.activeProjectId).toBe(beta.id);
    // The rows really went through the (mocked) command, not just local state.
    expect(backend.rows).toHaveLength(2);

    // The active selection is written to localStorage, so it survives a restart.
    const persisted = JSON.parse(localStorage.getItem("review-helper.projects")!);
    expect(persisted.state.activeProjectId).toBe(beta.id);

    useProjectStore.getState().select(s.projects[0].id);
    expect(useProjectStore.getState().activeProjectId).toBe(s.projects[0].id);
  });

  it("surfaces a backend validation error and writes nothing", async () => {
    await expect(useProjectStore.getState().create("   ", "new")).rejects.toBeTruthy();
    expect(useProjectStore.getState().projects).toHaveLength(0);
    expect(backend.rows).toHaveLength(0);
  });
});

describe("App shell", () => {
  it("first-run -> create -> section empty states -> settings", async () => {
    const user = userEvent.setup();
    render(<App />);

    // No projects yet: the first-run prompt is shown.
    await screen.findByText(/Create your first project/i);

    // Create a project through the dialog.
    await user.click(screen.getByRole("button", { name: /New project/i }));
    const dialog = await screen.findByRole("dialog", { name: /New project/i });
    await user.type(within(dialog).getByPlaceholderText(/My app/i), "Alpha");
    await user.click(within(dialog).getByRole("button", { name: /^Create$/i }));

    // First-run is replaced by the active project's default (Overview) pane.
    await screen.findByText(/No assessment yet/i);
    expect(screen.queryByText(/Create your first project/i)).toBeNull();
    expect(screen.getAllByText("Alpha").length).toBeGreaterThan(0);

    // Nav switches to another pane's empty state.
    await user.click(screen.getByRole("button", { name: /Decisions/i }));
    await screen.findByText(/No decisions recorded/i);

    // Settings is reachable and hosts the (functional) theme switcher.
    await user.click(screen.getByRole("button", { name: /Settings/i }));
    await screen.findByText(/Choose how Review Helper looks/i);
  });
});

describe("theme persistence", () => {
  it("reflects the choice onto <html> and saves it", () => {
    useThemeStore.getState().setTheme("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    const persisted = JSON.parse(localStorage.getItem("review-helper.theme")!);
    expect(persisted.state.theme).toBe("dark");
  });
});
