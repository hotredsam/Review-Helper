import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ProjectSwitcher } from "../components/ProjectSwitcher";
import { useProjectStore } from "../store/projectStore";
import type { Project } from "../api/projects";

const mkProject = (id: number, name: string): Project =>
  ({ id, name, kind: "new" }) as unknown as Project;

describe("ProjectSwitcher keyboard + ARIA (council round 3)", () => {
  beforeEach(() => {
    useProjectStore.setState({
      projects: [mkProject(1, "Alpha"), mkProject(2, "Beta")],
      activeProjectId: 1,
    });
  });

  it("opens via keyboard and exposes listbox semantics", () => {
    render(<ProjectSwitcher collapsed={false} onNewProject={() => {}} />);
    const trigger = screen.getByRole("button", { name: /switch project/i });
    expect(trigger.getAttribute("aria-haspopup")).toBe("listbox");
    expect(trigger.getAttribute("aria-expanded")).toBe("false");

    fireEvent.keyDown(trigger, { key: "ArrowDown" });
    expect(trigger.getAttribute("aria-expanded")).toBe("true");

    expect(screen.getByRole("listbox", { name: /projects/i })).toBeTruthy();
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(2);
    // Alpha is the active project, so its option is marked selected.
    expect(options[0].getAttribute("aria-selected")).toBe("true");
    expect(options[1].getAttribute("aria-selected")).toBe("false");
  });

  it("closes on Escape", () => {
    render(<ProjectSwitcher collapsed={false} onNewProject={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /switch project/i }));
    const listbox = screen.getByRole("listbox");
    expect(listbox).toBeTruthy();
    fireEvent.keyDown(listbox, { key: "Escape" });
    expect(screen.queryByRole("listbox")).toBeNull();
  });
});
