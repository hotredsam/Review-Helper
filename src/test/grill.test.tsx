import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";

const ctrl = vi.hoisted(() => ({ questions: [] as any[] }));

vi.mock("../api/grill", () => ({
  grillGenerate: vi.fn(async () => {}),
  grillList: vi.fn(async () => ctrl.questions),
  onGrillEvent: vi.fn(async () => () => {}),
}));

import { GrillPane } from "../components/GrillPane";

const project = (id: number) => ({ id }) as any;

describe("GrillPane", () => {
  it("invites grilling when there are no questions yet", async () => {
    ctrl.questions = [];
    render(<GrillPane project={project(101)} />);
    expect(await screen.findByText("Not grilled yet")).toBeTruthy();
    expect(screen.getByRole("button", { name: /Start grilling/i })).toBeTruthy();
  });

  it("lists generated questions with dimension + recommended answer", async () => {
    ctrl.questions = [
      {
        id: 1,
        dimension: "vision",
        bank_topic: "Core problem",
        text: "What problem does Brisket Helpline solve?",
        recommended_answer: "Tracking BBQ cooks and routing help.",
        status: "open",
      },
    ];
    render(<GrillPane project={project(102)} />);
    expect(await screen.findByText("What problem does Brisket Helpline solve?")).toBeTruthy();
    expect(screen.getByText("vision")).toBeTruthy();
    expect(screen.getByText("Tracking BBQ cooks and routing help.", { exact: false })).toBeTruthy();
  });
});
