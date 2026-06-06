import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { RadarChart, Gauge, ProgressBar, Donut } from "../components/charts";

describe("charts (theme-token, data-driven, no hardcoded colors)", () => {
  it("RadarChart renders one polygon per grid ring + the data polygon, from real axes", () => {
    const { container } = render(
      <RadarChart axes={[
        { label: "architecture", value: 70 },
        { label: "security", value: 40 },
        { label: "workflow", value: 90 },
      ]} />,
    );
    // 4 grid rings + 1 data polygon.
    expect(container.querySelectorAll("polygon").length).toBe(5);
    // data polygon uses currentColor (theme accent), not a hardcoded hex.
    expect(container.innerHTML).not.toMatch(/#[0-9a-fA-F]{6}/);
    expect(container.querySelector('[aria-label="Dimension scores"]')).toBeTruthy();
  });

  it("Gauge shows the clamped value and tints by score", () => {
    const { container } = render(<Gauge value={150} label="Overall" />);
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("aria-label")).toBe("Overall: 100 of 100"); // clamped
    expect(svg.classList.contains("text-success")).toBe(true); // high score -> success token
  });

  it("ProgressBar exposes an accessible progressbar with the right value", () => {
    const { getByRole } = render(<ProgressBar value={3} max={12} label="Coverage" />);
    const bar = getByRole("progressbar");
    expect(bar.getAttribute("aria-valuenow")).toBe("25");
    expect(bar.getAttribute("aria-label")).toBe("Coverage");
  });

  it("Donut renders a percentage from value/max", () => {
    const { container } = render(<Donut value={1} max={4} label="Done" />);
    expect(container.textContent).toContain("25%");
    expect(container.querySelector('[aria-label="Done: 1 of 4"]')).toBeTruthy();
  });
});
