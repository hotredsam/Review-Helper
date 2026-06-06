import { describe, it, expect, beforeEach, vi } from "vitest";
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
}));

import { UnderstandHub } from "../components/UnderstandHub";
import { WhyExplain } from "../components/WhyExplain";

beforeEach(() => {
  ctrl.cards = [
    { id: 1, term: "MVP", domain: "business", what_md: "min viable", when_md: "start", why_md: "learn fast", source: "seed" },
  ];
});

describe("UnderstandHub", () => {
  it("browses seeded cards and opens one", async () => {
    const user = userEvent.setup();
    render(<UnderstandHub />);
    await user.click(await screen.findByRole("button", { name: "MVP" }));
    expect(await screen.findByText("min viable")).toBeTruthy();
  });

  it("explains a cold term and shows the new card", async () => {
    const user = userEvent.setup();
    render(<UnderstandHub />);
    await screen.findByRole("button", { name: "MVP" });
    await user.type(screen.getByPlaceholderText(/Explain anything/i), "Bloom filter");
    await user.click(screen.getByRole("button", { name: /Explain/i }));
    expect(await screen.findByText("It is a generated explanation.")).toBeTruthy();
  });
});

describe("WhyExplain", () => {
  it("surfaces a rationale card on demand and links it", async () => {
    const user = userEvent.setup();
    render(<WhyExplain term="SQLite" />);
    await user.click(screen.getByRole("button", { name: /Why\?/i }));
    expect(await screen.findByText("SQLite:")).toBeTruthy();
    expect(screen.getByText("It is a generated explanation.")).toBeTruthy();
  });
});
