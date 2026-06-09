import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const ctrl = vi.hoisted(() => ({ cards: [] as any[] }));

vi.mock("../api/cards", () => ({
  cardsList: vi.fn(async () => ctrl.cards),
  cardGet: vi.fn(async () => null),
  cardExplain: vi.fn(async (term: string) => ({
    id: 99,
    term,
    domain: "other",
    what_md: "It is a generated explanation.",
    when_md: "Use when Y.",
    why_md: "Matters because Z.",
    source: "generated",
  })),
  cardCapture: vi.fn(async () => ({})),
  cardProjectTerms: vi.fn(async () => []),
  cardCleanTerm: vi.fn(async (t: string) => t),
  cardPremadeQuestions: vi.fn(async () => []),
  cardChatHistory: vi.fn(async () => []),
  cardChatSend: vi.fn(async () => "reply"),
}));

import { cardsList, cardExplain } from "../api/cards";
import { UnderstandHub } from "../components/UnderstandHub";
import { WhyExplain } from "../components/WhyExplain";

const proj: any = { id: 1, name: "P", kind: "imported", clone_path: "/c", github_repo_url: "u", default_branch: "main", app_type: null, created_at: "", updated_at: "" };

beforeEach(() => {
  ctrl.cards = [
    { id: 1, term: "MVP", domain: "business", what_md: "min viable", when_md: "start", why_md: "learn fast", source: "seed" },
  ];
});
afterEach(() => vi.clearAllMocks());

describe("UnderstandHub", () => {
  it("browses seeded cards and opens one", async () => {
    const user = userEvent.setup();
    render(<UnderstandHub project={proj} />);
    await user.click(await screen.findByRole("button", { name: "MVP" }));
    expect(await screen.findByText("min viable")).toBeTruthy();
  });

  it("explains a cold term and shows the new card", async () => {
    const user = userEvent.setup();
    render(<UnderstandHub project={proj} />);
    await screen.findByRole("button", { name: "MVP" });
    await user.type(screen.getByPlaceholderText(/Explain anything/i), "Bloom filter");
    await user.click(screen.getByRole("button", { name: /Explain/i }));
    expect(await screen.findByText("It is a generated explanation.")).toBeTruthy();
  });

  it("surfaces an error (offline) when listing cards fails", async () => {
    vi.mocked(cardsList).mockRejectedValueOnce(new Error("Claude not available"));
    render(<UnderstandHub project={proj} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Claude not available");
  });

  it("surfaces an error when generation fails — never silently dead-ends", async () => {
    const user = userEvent.setup();
    render(<UnderstandHub project={proj} />);
    await screen.findByRole("button", { name: "MVP" });
    vi.mocked(cardExplain).mockRejectedValueOnce(new Error("Model unavailable"));
    await user.type(screen.getByPlaceholderText(/Explain anything/i), "Bloom filter");
    await user.click(screen.getByRole("button", { name: /Explain/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Model unavailable");
  });

  it("renders only the populated sections of a partial card", async () => {
    const user = userEvent.setup();
    render(<UnderstandHub project={proj} />);
    await screen.findByRole("button", { name: "MVP" });
    vi.mocked(cardExplain).mockResolvedValueOnce({
      id: 5,
      term: "Partial",
      domain: "other",
      what_md: "Only the what.",
      when_md: "",
      why_md: "",
      source: "generated",
    });
    await user.type(screen.getByPlaceholderText(/Explain anything/i), "Partial");
    await user.click(screen.getByRole("button", { name: /Explain/i }));
    expect(await screen.findByText("Only the what.")).toBeTruthy();
    expect(screen.queryByText("When to use it")).toBeNull();
    expect(screen.queryByText("Why it matters")).toBeNull();
  });
});

describe("WhyExplain", () => {
  it("surfaces a rationale card on demand and links it", async () => {
    const user = userEvent.setup();
    render(<WhyExplain term="SQLite" />);
    await user.click(screen.getByRole("button", { name: /why/i }));
    expect(await screen.findByText("SQLite:")).toBeTruthy();
    expect(screen.getByText("It is a generated explanation.")).toBeTruthy();
  });

  it("surfaces an error when the explanation fails", async () => {
    const user = userEvent.setup();
    vi.mocked(cardExplain).mockRejectedValueOnce(new Error("Out of credits"));
    render(<WhyExplain term="SQLite" />);
    await user.click(screen.getByRole("button", { name: /why/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Out of credits");
  });
});
