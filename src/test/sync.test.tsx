import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const ctrl = vi.hoisted(() => ({
  preview: {
    issue_actions: [
      { kind: "create", marker: "phase-setup", title: "Setup", body: "", state: "open", labels: ["status:todo"] },
      { kind: "close", number: 9, title: "Old phase" },
    ],
    file_deletions: [".planning/phases/phase-03-old.md"],
  } as any,
  result: { files_pushed: 4, issues_applied: 2, files_deleted: 1, failures: [] as string[] },
}));

vi.mock("../api/sync", () => ({
  syncPushPlanning: vi.fn(async () => 4),
  syncMainPreview: vi.fn(async () => ctrl.preview),
  syncMainApply: vi.fn(async () => ctrl.result),
}));

import { SyncPanel } from "../components/SyncPanel";
import { syncPushPlanning, syncMainPreview, syncMainApply } from "../api/sync";

const project = (over: Partial<any> = {}) => ({ id: 1, github_repo_url: "https://github.com/o/r.git", ...over }) as any;

beforeEach(() => {
  vi.clearAllMocks();
  ctrl.result = { files_pushed: 4, issues_applied: 2, files_deleted: 1, failures: [] };
});

describe("SyncPanel", () => {
  it("hides for a project with no GitHub repo", () => {
    const { container } = render(<SyncPanel project={project({ github_repo_url: null })} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("pushes to the planning branch", async () => {
    const user = userEvent.setup();
    render(<SyncPanel project={project()} />);
    await user.click(screen.getByRole("button", { name: /Push to planning branch/i }));
    expect(vi.mocked(syncPushPlanning)).toHaveBeenCalledWith(1);
    expect(await screen.findByText(/Pushed 4 files to the planning branch/i)).toBeTruthy();
  });

  it("previews issues + file deletions, then applies the confirmed preview", async () => {
    const user = userEvent.setup();
    render(<SyncPanel project={project()} />);
    await user.click(screen.getByRole("button", { name: /Preview push to main/i }));
    expect(vi.mocked(syncMainPreview)).toHaveBeenCalledWith(1);
    // Both destructive surfaces shown before confirm.
    expect(await screen.findByText(/Issue changes \(2\)/i)).toBeTruthy();
    expect(screen.getByText(/Files to delete \(1\)/i)).toBeTruthy();
    expect(screen.getByText(".planning/phases/phase-03-old.md")).toBeTruthy();
    expect(vi.mocked(syncMainApply)).not.toHaveBeenCalled();

    // Confirm replays the exact previewed actions.
    await user.click(screen.getByRole("button", { name: /Confirm: push to main/i }));
    expect(vi.mocked(syncMainApply)).toHaveBeenCalledWith(1, ctrl.preview);
    expect(await screen.findByText(/removed 1 stale file/i)).toBeTruthy();
  });

  it("drops a loaded preview when the project changes (no cross-project apply)", async () => {
    const user = userEvent.setup();
    const { rerender } = render(<SyncPanel project={project()} />);
    await user.click(screen.getByRole("button", { name: /Preview push to main/i }));
    expect(await screen.findByText(/Issue changes \(2\)/i)).toBeTruthy();

    // Same component instance, different project: A's preview must not survive.
    rerender(<SyncPanel project={project({ id: 2 })} />);
    expect(screen.queryByText(/Issue changes/i)).toBeNull();
    expect(screen.queryByRole("button", { name: /Confirm: push to main/i })).toBeNull();
  });

  it("keeps the preview visible and reports partial failures", async () => {
    ctrl.result = { files_pushed: 4, issues_applied: 1, files_deleted: 0, failures: ["close #9: 403"] };
    const user = userEvent.setup();
    render(<SyncPanel project={project()} />);
    await user.click(screen.getByRole("button", { name: /Preview push to main/i }));
    await user.click(await screen.findByRole("button", { name: /Confirm: push to main/i }));
    expect(await screen.findByText(/Some steps failed: close #9: 403/i)).toBeTruthy();
    // Preview not cleared on failure.
    expect(screen.getByText(/Issue changes \(2\)/i)).toBeTruthy();
  });
});
