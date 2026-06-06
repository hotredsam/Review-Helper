import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const ctrl = vi.hoisted(() => ({ pending: [] as any[], decisions: [] as any[] }));

vi.mock("../api/suggestions", () => ({
  suggestionsList: vi.fn(async () => ctrl.pending),
  suggestionApprove: vi.fn(async () => {}),
  suggestionDismiss: vi.fn(async () => {}),
  suggestionsApproveAll: vi.fn(async () => 0),
  summarizeSuggestion: (s: any) => `${s.kind}: ${s.payload?.choice ?? s.payload?.title ?? ""}`,
}));
vi.mock("../api/decisions", () => ({
  decisionsList: vi.fn(async () => ctrl.decisions),
  decisionSupersede: vi.fn(async () => {}),
}));

import { DecisionsPane } from "../components/DecisionsPane";
import { useDecisionsStore } from "../store/decisionsStore";
import { suggestionApprove, suggestionDismiss, suggestionsApproveAll } from "../api/suggestions";
import { decisionSupersede } from "../api/decisions";

const project = (id: number) => ({ id }) as any;

beforeEach(() => {
  useDecisionsStore.setState({ pending: {}, decisions: {}, error: {} });
  ctrl.pending = [];
  ctrl.decisions = [];
  vi.clearAllMocks();
});

describe("DecisionsPane — pending approval", () => {
  it("shows an empty state when there are no pending suggestions", async () => {
    render(<DecisionsPane project={project(1)} />);
    expect(await screen.findByText(/No pending suggestions/i)).toBeTruthy();
  });

  it("approves and dismisses individual suggestions", async () => {
    ctrl.pending = [
      { id: 10, kind: "decision", payload: { topic: "DB", choice: "SQLite" }, status: "pending", created_at: "" },
      { id: 11, kind: "feature", payload: { title: "CSV" }, status: "pending", created_at: "" },
    ];
    const user = userEvent.setup();
    render(<DecisionsPane project={project(2)} />);
    await user.click(await screen.findByRole("button", { name: /Approve decision/i }));
    expect(vi.mocked(suggestionApprove)).toHaveBeenCalledWith(2, 10);
    await user.click(screen.getByRole("button", { name: /Dismiss feature/i }));
    expect(vi.mocked(suggestionDismiss)).toHaveBeenCalledWith(2, 11);
  });

  it("approves all at once", async () => {
    ctrl.pending = [
      { id: 10, kind: "decision", payload: { topic: "DB", choice: "SQLite" }, status: "pending", created_at: "" },
    ];
    const user = userEvent.setup();
    render(<DecisionsPane project={project(3)} />);
    await user.click(await screen.findByRole("button", { name: /Approve all/i }));
    expect(vi.mocked(suggestionsApproveAll)).toHaveBeenCalledWith(3);
  });
});

describe("DecisionsPane — decisions record", () => {
  it("shows all decision fields and supersedes an active one", async () => {
    ctrl.decisions = [
      {
        id: 50,
        topic: "Database",
        choice: "SQLite",
        rationale: "local + simple",
        alternatives: "Postgres; files",
        consequences: "no server to run",
        source_ref: "chat",
        status: "active",
        created_at: "",
      },
    ];
    const user = userEvent.setup();
    render(<DecisionsPane project={project(4)} />);
    expect(await screen.findByText("Database")).toBeTruthy();
    expect(screen.getByText(/local \+ simple/)).toBeTruthy();
    expect(screen.getByText(/Postgres; files/)).toBeTruthy();
    expect(screen.getByText(/no server to run/)).toBeTruthy();
    expect(screen.getByText("Active")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: /Supersede Database/i }));
    expect(vi.mocked(decisionSupersede)).toHaveBeenCalledWith(4, 50);
  });

  it("hides the supersede control on an already-superseded decision", async () => {
    ctrl.decisions = [
      { id: 51, topic: "Old", choice: "X", rationale: null, alternatives: null, consequences: null, source_ref: null, status: "superseded", created_at: "" },
    ];
    render(<DecisionsPane project={project(5)} />);
    expect(await screen.findByText("Superseded")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Supersede Old/i })).toBeNull();
  });
});
