import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const ctrl = vi.hoisted(() => ({
  catalog: {
    frontend: [
      { choice: "React + Vite", rationale: "r1" },
      { choice: "Svelte", rationale: "r2" },
    ],
    backend: [{ choice: "Tauri (Rust)", rationale: "rb" }],
    database: [{ choice: "SQLite", rationale: "rd" }],
    deployment: [{ choice: "Direct download", rationale: "rdep" }],
    pipes: [{ choice: "None", rationale: "rp" }],
  },
  premade: [
    {
      name: "Local-first desktop",
      summary: "Native, no servers.",
      panes: { frontend: "React + Vite", backend: "Tauri (Rust)", database: "SQLite", deployment: "Direct download", pipes: "None" },
    },
  ],
  selections: [
    { pane: "frontend", choice: "React + Vite", alternatives: "Svelte", rationale: "r1" },
    { pane: "backend", choice: null, alternatives: null, rationale: null },
    { pane: "database", choice: null, alternatives: null, rationale: null },
    { pane: "deployment", choice: null, alternatives: null, rationale: null },
    { pane: "pipes", choice: null, alternatives: null, rationale: null },
  ],
}));

vi.mock("../api/stack", () => ({
  stackCatalog: vi.fn(async () => ctrl.catalog),
  stackPremade: vi.fn(async () => ctrl.premade),
  stackList: vi.fn(async () => ctrl.selections),
  stackSet: vi.fn(async () => {}),
  stackApplyPremade: vi.fn(async () => {}),
}));
vi.mock("../api/cards", () => ({ cardExplain: vi.fn(async () => ({})) }));

import { StackPane } from "../components/StackPane";
import { useStackStore } from "../store/stackStore";
import { stackSet, stackApplyPremade } from "../api/stack";

const project = (id: number) => ({ id }) as any;

beforeEach(() => {
  useStackStore.setState({ catalog: {}, premade: [], selections: {}, error: {} });
  vi.clearAllMocks();
});

describe("StackPane", () => {
  it("applies a pre-made stack to all five panes", async () => {
    const user = userEvent.setup();
    render(<StackPane project={project(1)} />);
    await user.click(await screen.findByRole("button", { name: /Local-first desktop/i }));
    expect(vi.mocked(stackApplyPremade)).toHaveBeenCalledWith(1, "Local-first desktop");
  });

  it("overrides a single pane and flags the recommendation", async () => {
    const user = userEvent.setup();
    render(<StackPane project={project(2)} />);
    // The first option of each pane is marked recommended.
    expect((await screen.findAllByText(/recommended/i)).length).toBeGreaterThan(0);
    // Pick the alternative for frontend.
    await user.click(screen.getByRole("button", { name: /^Svelte$/i }));
    expect(vi.mocked(stackSet)).toHaveBeenCalledWith(2, "frontend", "Svelte");
  });
});
