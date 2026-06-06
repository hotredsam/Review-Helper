import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

vi.mock("../api/sync", () => ({
  syncPushPlanning: vi.fn(async () => 4),
  syncIssuePreview: vi.fn(async () => [
    { kind: "create", marker: "phase-setup", title: "Setup", body: "", state: "open", label: "status:todo" },
    { kind: "close", number: 9, title: "Old phase" },
  ]),
  syncIssueApply: vi.fn(async () => 2),
  syncPushMain: vi.fn(async () => 4),
}));

import { SyncPanel } from "../components/SyncPanel";
import { syncPushPlanning, syncIssuePreview, syncIssueApply, syncPushMain } from "../api/sync";

const project = (over: Partial<any> = {}) => ({ id: 1, github_repo_url: "https://github.com/o/r.git", ...over }) as any;

beforeEach(() => vi.clearAllMocks());

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

  it("previews issue changes then applies + pushes to main on confirm", async () => {
    const user = userEvent.setup();
    render(<SyncPanel project={project()} />);
    // Preview first (the confirm gate) — nothing is written yet.
    await user.click(screen.getByRole("button", { name: /Preview push to main/i }));
    expect(vi.mocked(syncIssuePreview)).toHaveBeenCalledWith(1);
    expect(await screen.findByText(/Issue changes \(2\)/i)).toBeTruthy();
    expect(vi.mocked(syncIssueApply)).not.toHaveBeenCalled();

    // Confirm applies issues + pushes docs.
    await user.click(screen.getByRole("button", { name: /Confirm: sync issues/i }));
    expect(vi.mocked(syncIssueApply)).toHaveBeenCalledWith(1);
    expect(vi.mocked(syncPushMain)).toHaveBeenCalledWith(1);
    expect(await screen.findByText(/Synced 2 issue\(s\) and pushed 4 files to main/i)).toBeTruthy();
  });
});
