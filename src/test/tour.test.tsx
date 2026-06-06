import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { Tour, TOUR_STEPS, tourSeen } from "../components/Tour";
import { InfoDot } from "../components/InfoDot";

beforeEach(() => localStorage.clear());

describe("Tour", () => {
  it("walks the steps and marks itself seen on finish", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<Tour onClose={onClose} />);

    expect(screen.getByText(TOUR_STEPS[0].title)).toBeTruthy();
    expect(tourSeen()).toBe(false);

    // Step through to the last step.
    for (let i = 0; i < TOUR_STEPS.length - 1; i++) {
      await user.click(screen.getByRole("button", { name: /^Next/i }));
    }
    expect(screen.getByText(TOUR_STEPS[TOUR_STEPS.length - 1].title)).toBeTruthy();

    await user.click(screen.getByRole("button", { name: /Get started/i }));
    expect(onClose).toHaveBeenCalled();
    expect(tourSeen()).toBe(true); // won't auto-show again
  });

  it("can be skipped, which also marks it seen", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<Tour onClose={onClose} />);
    await user.click(screen.getByRole("button", { name: /Skip tour/i }));
    expect(onClose).toHaveBeenCalled();
    expect(tourSeen()).toBe(true);
  });
});

describe("InfoDot", () => {
  it("explains a jargon term on click", async () => {
    const user = userEvent.setup();
    render(<InfoDot term="MVP" explanation="The smallest version that's still useful." />);
    expect(screen.queryByRole("tooltip")).toBeNull();
    await user.click(screen.getByRole("button", { name: /What is MVP/i }));
    expect(screen.getByRole("tooltip")).toHaveTextContent("smallest version");
  });
});
