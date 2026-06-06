import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const ctrl = vi.hoisted(() => ({ pending: [] as any[] }));

vi.mock("../api/suggestions", () => ({
  suggestionsList: vi.fn(async () => ctrl.pending),
  suggestionApprove: vi.fn(async () => {}),
  suggestionDismiss: vi.fn(async () => {}),
  suggestionsApproveAll: vi.fn(async () => 0),
  summarizeSuggestion: (s: any) => `${s.kind}: ${s.payload?.choice ?? s.payload?.title ?? ""}`,
}));

import { DecisionsPane } from "../components/DecisionsPane";
import { useDecisionsStore } from "../store/decisionsStore";
import { suggestionApprove, suggestionDismiss, suggestionsApproveAll } from "../api/suggestions";

const project = (id: number) => ({ id }) as any;

beforeEach(() => {
  useDecisionsStore.setState({ pending: {}, error: {} });
  ctrl.pending = [];
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
