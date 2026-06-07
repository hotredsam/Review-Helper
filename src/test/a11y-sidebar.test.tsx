import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { Sidebar } from "../components/Sidebar";
import { useUiStore } from "../store/uiStore";
import { useProjectStore } from "../store/projectStore";

describe("Sidebar accessibility (council round 4)", () => {
  beforeEach(() => {
    useProjectStore.setState({ projects: [], activeProjectId: null });
  });

  it("section buttons keep an accessible name even when the rail is collapsed", () => {
    useUiStore.setState({ sidebarCollapsed: true });
    render(<Sidebar onNewProject={() => {}} hasProject={true} />);
    // Icon-only when collapsed, but still reachable by name for AT users.
    expect(screen.getByRole("button", { name: "Plan" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Understand" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Toggle sidebar" })).toBeTruthy();
  });
});
