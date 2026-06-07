import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { ComingSoon } from "../components/ComingSoon";
import { SECTIONS } from "../nav/sections";

describe("Learning mode stub (Phase 14)", () => {
  it("renders a clearly-marked coming-soon placeholder", () => {
    render(<ComingSoon />);
    expect(screen.getByText(/Learning mode — coming soon/i)).toBeTruthy();
    expect(screen.getByText(/^Coming soon$/)).toBeTruthy();
  });

  it("is a navigable section", () => {
    const learn = SECTIONS.find((s) => s.id === "learn");
    expect(learn).toBeTruthy();
    expect(learn!.label).toMatch(/learn/i);
  });
});
