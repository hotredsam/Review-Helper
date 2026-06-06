import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const ctrl = vi.hoisted(() => ({ status: null as any }));

vi.mock("../api/model", () => ({
  getModelStatus: vi.fn(async () => ctrl.status),
  runModel: vi.fn(async () => {}),
  onModelEvent: vi.fn(async () => () => {}),
}));

import { ModelBanner } from "../components/ModelBanner";
import { useStatusStore } from "../store/statusStore";

const claudeDown = {
  provider: "claude",
  available: false,
  version: null,
  reason: "not_installed",
  command: "claude --version",
  exit_code: null,
  stderr: "claude not found",
};
const claudeUp = { ...claudeDown, available: true, version: "2.1.167", reason: null, exit_code: 0, stderr: "" };
const local = {
  provider: "local",
  available: true,
  version: null,
  reason: null,
  command: "(local stub)",
  exit_code: 0,
  stderr: "",
};

beforeEach(() => {
  useStatusStore.setState({ status: null, loading: false });
});

describe("ModelBanner", () => {
  it("shows when Claude is unavailable and clears after Retry restores it", async () => {
    const user = userEvent.setup();
    ctrl.status = claudeDown;
    await useStatusStore.getState().refresh();
    render(<ModelBanner />);

    await screen.findByText(/Claude not available/i);

    ctrl.status = claudeUp; // claude comes back
    await user.click(screen.getByRole("button", { name: /Retry/i }));
    await waitFor(() => expect(screen.queryByText(/Claude not available/i)).toBeNull());
  });

  it("does not show when the provider is the local stub", async () => {
    ctrl.status = local;
    await useStatusStore.getState().refresh();
    const { container } = render(<ModelBanner />);
    expect(container.firstChild).toBeNull();
  });
});
