import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const ctrl = vi.hoisted(() => ({ subjects: [] as any[], queue: { cards: [] as any[], total: 0, next_due: null as string | null } }));

vi.mock("../api/learning", () => ({
  subjectsList: vi.fn(async () => ctrl.subjects),
  subjectCreate: vi.fn(async () => 7),
  subjectDelete: vi.fn(async () => {}),
  subjectGet: vi.fn(async (id: number) => ({
    id,
    title: "Spanish A1",
    source_kind: "describe",
    source_text: "Conversational basics for a trip.",
    stage: "intake",
  })),
  learningIntake: vi.fn(async () => [
    { id: 1, idx: 0, question: "What's your current level?", answer: null },
    { id: 2, idx: 1, question: "What's your goal?", answer: "A trip" },
  ]),
  learningIntakeAnswer: vi.fn(async () => {}),
  learningFlashcards: vi.fn(async () => []),
  learningFlashcardsQueue: vi.fn(async () => ctrl.queue),
  learningFlashcardGrade: vi.fn(async () => "2099-01-01T00:00:00+00:00"),
}));

import { LearningShell } from "../components/learning/LearningShell";
import { useLearningStore } from "../store/learningStore";

beforeEach(() => {
  ctrl.subjects = [];
  useLearningStore.setState({ subjects: [], selectedSubjectId: null, status: "idle", error: null });
});
afterEach(() => vi.clearAllMocks());

describe("LearningShell", () => {
  it("shows an inviting empty state when there are no subjects", async () => {
    render(<LearningShell />);
    expect(await screen.findByText(/No subjects yet/i)).toBeTruthy();
  });

  it("lists subjects and opens one into its detail view", async () => {
    ctrl.subjects = [
      { id: 1, title: "Spanish A1", source_kind: "describe", stage: "intake", created_at: "", updated_at: "" },
    ];
    const user = userEvent.setup();
    render(<LearningShell />);
    await user.click(await screen.findByRole("button", { name: /Spanish A1/i }));
    // Detail view loads the subject's goal + the generated scoping questions.
    expect(await screen.findByText(/Conversational basics for a trip\./i)).toBeTruthy();
    expect(await screen.findByText(/What's your current level\?/i)).toBeTruthy();
  });

  it("surfaces an error when loading subjects fails — never a blank screen", async () => {
    const { subjectsList } = await import("../api/learning");
    vi.mocked(subjectsList).mockRejectedValueOnce(new Error("DB locked"));
    render(<LearningShell />);
    expect(await screen.findByRole("alert")).toHaveTextContent("DB locked");
  });
});

describe("SubjectDetail delete (Phase 15)", () => {
  it("confirms through the Modal — never window.confirm, which is dead under wry", async () => {
    const { SubjectDetail } = await import("../components/learning/SubjectDetail");
    const { subjectDelete } = await import("../api/learning");
    const { waitFor } = await import("@testing-library/react");
    const onBack = vi.fn();
    const user = userEvent.setup();
    render(<SubjectDetail subjectId={7} onBack={onBack} />);
    await screen.findByText("Spanish A1");

    await user.click(screen.getByRole("button", { name: "Delete subject" }));
    expect(vi.mocked(subjectDelete)).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(vi.mocked(subjectDelete)).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Delete subject" }));
    const btns = screen.getAllByRole("button", { name: "Delete subject" });
    await user.click(btns[btns.length - 1]);
    await waitFor(() => expect(vi.mocked(subjectDelete)).toHaveBeenCalledWith(7));
    await waitFor(() => expect(onBack).toHaveBeenCalled());
  });
});

describe("FlashcardsPane FSRS queue (Phase 17)", () => {
  it("shows the nothing-due empty state with the next due date", async () => {
    ctrl.queue = { cards: [], total: 5, next_due: "2099-06-01T09:00:00+00:00" };
    const { FlashcardsPane } = await import("../components/learning/FlashcardsPane");
    render(<FlashcardsPane moduleId={3} />);
    expect(await screen.findByText(/Nothing due right now/i)).toBeTruthy();
    expect(screen.getByText(/Next card due/i)).toBeTruthy();
  });

  it("serves the queue and finishes with a session-complete state", async () => {
    ctrl.queue = {
      cards: [
        { id: 1, front: "F1", back: "B1", due: null, reps: 0 },
        { id: 2, front: "F2", back: "B2", due: null, reps: 0 },
      ],
      total: 2,
      next_due: null,
    };
    const { FlashcardsPane } = await import("../components/learning/FlashcardsPane");
    const { learningFlashcardGrade } = await import("../api/learning");
    const user = userEvent.setup();
    render(<FlashcardsPane moduleId={4} />);

    // Card 1: flip, grade Good.
    await user.click(await screen.findByText("F1"));
    await user.click(screen.getByRole("button", { name: "Good" }));
    expect(vi.mocked(learningFlashcardGrade)).toHaveBeenCalledWith(1, 3);

    // Card 2: flip, grade Easy → session complete.
    await user.click(await screen.findByText("F2"));
    await user.click(screen.getByRole("button", { name: "Easy" }));
    expect(await screen.findByText(/Session complete/i)).toBeTruthy();
    expect(screen.getByRole("button", { name: /Check for due cards/i })).toBeTruthy();
  });
});
