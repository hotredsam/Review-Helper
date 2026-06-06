import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const ctrl = vi.hoisted(() => ({ questions: [] as any[] }));

vi.mock("../api/grill", () => ({
  grillGenerate: vi.fn(async () => {}),
  grillList: vi.fn(async () => ctrl.questions),
  grillAnswer: vi.fn(async () => {}),
  grillChatResolve: vi.fn(async () => {}),
  grillSetStatus: vi.fn(async () => {}),
  grillDelete: vi.fn(async () => {}),
  onGrillEvent: vi.fn(async () => () => {}),
}));

import { GrillPane } from "../components/GrillPane";
import { QuestionCard } from "../components/QuestionCard";
import { grillAnswer, grillChatResolve, grillSetStatus } from "../api/grill";

const project = (id: number) => ({ id }) as any;
const question = (over: Partial<any> = {}) => ({
  id: 7,
  dimension: "vision",
  bank_topic: "Core problem",
  text: "What problem does Brisket Helpline solve?",
  recommended_answer: "Tracking BBQ cooks and routing help.",
  status: "open",
  ...over,
});

describe("GrillPane", () => {
  it("invites grilling when there are no questions yet", async () => {
    ctrl.questions = [];
    render(<GrillPane project={project(101)} />);
    expect(await screen.findByText("Not grilled yet")).toBeTruthy();
    expect(screen.getByRole("button", { name: /Start grilling/i })).toBeTruthy();
  });

  it("lists generated questions with dimension + recommended answer", async () => {
    ctrl.questions = [question()];
    render(<GrillPane project={project(102)} />);
    expect(await screen.findByText("What problem does Brisket Helpline solve?")).toBeTruthy();
    expect(screen.getByText("vision")).toBeTruthy();
    expect(screen.getByText("Tracking BBQ cooks and routing help.", { exact: false })).toBeTruthy();
  });
});

describe("QuestionCard actions", () => {
  it("submits a typed answer", async () => {
    const user = userEvent.setup();
    render(
      <ul>
        <QuestionCard projectId={5} question={question({ id: 7 })} />
      </ul>,
    );
    await user.type(screen.getByPlaceholderText("Your answer…"), "Solo pitmasters");
    await user.click(screen.getByRole("button", { name: /Submit/i }));
    expect(vi.mocked(grillAnswer)).toHaveBeenCalledWith(5, 7, "Solo pitmasters");
  });

  it("dismisses as not relevant", async () => {
    const user = userEvent.setup();
    render(
      <ul>
        <QuestionCard projectId={5} question={question({ id: 9 })} />
      </ul>,
    );
    await user.click(screen.getByRole("button", { name: /Not relevant/i }));
    expect(vi.mocked(grillSetStatus)).toHaveBeenCalledWith(5, 9, "not_relevant");
  });

  it("'Let's chat' writes the resolution back into the card", async () => {
    const user = userEvent.setup();
    render(
      <ul>
        <QuestionCard projectId={5} question={question({ id: 8 })} />
      </ul>,
    );
    await user.click(screen.getByRole("button", { name: /Let's chat/i }));
    await user.type(screen.getByPlaceholderText("What did you decide?"), "Read-only v1");
    await user.click(screen.getByRole("button", { name: /Save resolution/i }));
    expect(vi.mocked(grillChatResolve)).toHaveBeenCalledWith(5, 8, "Read-only v1");
  });
});
