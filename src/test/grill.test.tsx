import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
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
import { computeCoverage } from "../store/grillStore";
import { grillAnswer, grillChatResolve, grillSetStatus, grillGenerate } from "../api/grill";

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

describe("computeCoverage", () => {
  const q = (id: number, status: string, dim = "vision") =>
    ({ id, dimension: dim, bank_topic: "t", text: "q", recommended_answer: "r", status }) as any;

  it("is done only when at least one question exists and none are open", () => {
    expect(computeCoverage([]).done).toBe(false);

    const mixed = computeCoverage([q(1, "open"), q(2, "answered")]);
    expect(mixed.done).toBe(false);
    expect(mixed).toMatchObject({ total: 2, addressed: 1, open: 1 });

    // answered, not_relevant and unknown all count as addressed -> done.
    const all = computeCoverage([q(1, "answered"), q(2, "not_relevant"), q(3, "unknown")]);
    expect(all.done).toBe(true);
    expect(all.open).toBe(0);

    // adding a new open question re-opens it (e.g. a feature in a later phase).
    expect(computeCoverage([q(1, "answered"), q(4, "open")]).done).toBe(false);

    // deleted questions are excluded from the totals.
    const withDeleted = computeCoverage([q(1, "answered"), q(2, "deleted")]);
    expect(withDeleted.total).toBe(1);
    expect(withDeleted.done).toBe(true);
  });

  it("breaks coverage down by dimension", () => {
    const cov = computeCoverage([q(1, "answered", "vision"), q(2, "open", "vision"), q(3, "answered", "ux")]);
    const vision = cov.byDimension.find((d) => d.dimension === "vision")!;
    expect(vision).toMatchObject({ total: 2, addressed: 1 });
    const ux = cov.byDimension.find((d) => d.dimension === "ux")!;
    expect(ux.addressed).toBe(ux.total);
  });
});

describe("depth slider", () => {
  it("raising depth then asking generates with the higher depth", async () => {
    ctrl.questions = [question({ id: 1, status: "answered" })]; // populated + done
    const user = userEvent.setup();
    render(<GrillPane project={project(202)} />);
    await screen.findByText(/Detail coverage/i);
    fireEvent.change(screen.getByLabelText("Grill depth"), { target: { value: "5" } });
    await user.click(screen.getByRole("button", { name: /Ask more|Go deeper/i }));
    expect(vi.mocked(grillGenerate)).toHaveBeenCalledWith(202, 5);
  });
});
