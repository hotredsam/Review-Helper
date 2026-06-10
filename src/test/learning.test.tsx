import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const ctrl = vi.hoisted(() => ({ subjects: [] as any[] }));

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
