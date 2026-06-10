import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const ctrl = vi.hoisted(() => ({ features: [] as any[] }));

vi.mock("../api/features", () => ({
  featuresList: vi.fn(async () => ctrl.features),
  featureAdd: vi.fn(async () => {}),
  featureSetStatus: vi.fn(async () => {}),
  featuresPendingCount: vi.fn(async () => 0),
}));
vi.mock("../api/transcribe", () => ({
  transcribeStart: vi.fn(async () => {}),
  transcribeStop: vi.fn(async () => "ship a brisket timer"),
  transcribeCancel: vi.fn(async () => {}),
  onTranscribeEvent: vi.fn(async () => () => {}),
}));

import { InboxPane } from "../components/InboxPane";
import { useFeaturesStore } from "../store/featuresStore";
import { featureAdd, featureSetStatus } from "../api/features";
import { transcribeStart, transcribeStop } from "../api/transcribe";

const project = (id: number) => ({ id }) as any;

beforeEach(() => {
  useFeaturesStore.setState({ features: {}, error: {} });
  ctrl.features = [];
  vi.clearAllMocks();
});

describe("InboxPane", () => {
  it("captures a text feature", async () => {
    const user = userEvent.setup();
    render(<InboxPane project={project(1)} />);
    await user.type(screen.getByLabelText("Feature idea"), "CSV export");
    await user.click(screen.getByRole("button", { name: /^Add$/i }));
    expect(vi.mocked(featureAdd)).toHaveBeenCalledWith(1, "CSV export", "", undefined);
  });

  it("the mic records, stops, and lands the transcript in the capture box", async () => {
    const user = userEvent.setup();
    render(<InboxPane project={project(2)} />);

    await user.click(screen.getByRole("button", { name: /Capture by voice/i }));
    expect(vi.mocked(transcribeStart)).toHaveBeenCalled();
    expect(await screen.findByRole("button", { name: /Stop recording/i })).toBeTruthy();

    await user.click(screen.getByRole("button", { name: /Stop recording/i }));
    expect(vi.mocked(transcribeStop)).toHaveBeenCalled();
    expect(await screen.findByLabelText("Feature idea")).toHaveValue("ship a brisket timer");
  });

  it("shows the queue with a reject action and a soft nudge at 10", async () => {
    ctrl.features = Array.from({ length: 10 }, (_, i) => ({
      id: i + 1,
      title: `Idea ${i + 1}`,
      detail: null,
      source: "text",
      status: "inbox",
      created_at: "",
    }));
    const user = userEvent.setup();
    render(<InboxPane project={project(3)} />);
    expect(await screen.findByText("Idea 1")).toBeTruthy();
    expect(screen.getByText(/ideas waiting/i)).toBeTruthy();
    // Reject is one-way: it must confirm through the Modal first.
    await user.click(screen.getByRole("button", { name: "Reject Idea 1" }));
    expect(vi.mocked(featureSetStatus)).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(vi.mocked(featureSetStatus)).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Reject Idea 1" }));
    await user.click(screen.getByRole("button", { name: "Reject" }));
    expect(vi.mocked(featureSetStatus)).toHaveBeenCalledWith(3, 1, "rejected");
  });
});
