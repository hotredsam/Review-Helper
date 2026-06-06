import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, screen } from "@testing-library/react";

const ctrl = vi.hoisted(() => ({
  a: null as any,
  handler: null as null | ((e: any) => void),
}));

vi.mock("../api/assessment", () => ({
  getAssessment: vi.fn(async () => ctrl.a),
  assessProject: vi.fn(async () => {}),
  onAssessmentEvent: vi.fn(async (h: (e: any) => void) => {
    ctrl.handler = h;
    return () => {};
  }),
}));

import { useAssessStore, ensureAssessListener } from "../store/assessStore";
import { assessProject, getAssessment } from "../api/assessment";
import { StatePane } from "../components/StatePane";

const project = (over: any = {}) => ({
  id: 9,
  name: "P",
  kind: "imported",
  app_type: null,
  github_repo_url: "u",
  clone_path: "/c",
  default_branch: "main",
  created_at: "t",
  updated_at: "t",
  ...over,
});

const fullSample = {
  overall: 78,
  dimensions: {
    architecture: { score: 80, reason: "clean" },
    modularity: { score: 90, reason: "small files" },
    context_hygiene: { score: 85, reason: "CLAUDE.md" },
    security: { score: 40, reason: "no auth" },
    git_discipline: { score: 70, reason: "commits" },
    workflow: { score: 88, reason: "plan matches" },
  },
  production: {
    scores: {
      tests: { score: 80, reason: "" },
      error_handling: { score: 60, reason: "" },
      secrets: { score: 90, reason: "" },
      build_ci: { score: 30, reason: "" },
      dependencies: { score: 70, reason: "" },
      docs: { score: 85, reason: "" },
    },
    overall: 69,
  },
  top_fixes: ["Add CI", "Cover error paths"],
  hygiene: ["No obvious cruft"],
  created_at: "t",
};

const sample = {
  overall: 70,
  dimensions: {},
  production: { scores: {}, overall: 60 },
  top_fixes: [],
  hygiene: [],
  created_at: "t",
};

beforeEach(() => {
  ctrl.a = null;
  useAssessStore.setState({ assessments: {}, status: {}, progress: {}, error: {} });
});

describe("assessStore", () => {
  it("loads an assessment", async () => {
    ctrl.a = sample;
    await useAssessStore.getState().load(1);
    expect(useAssessStore.getState().assessments[1]?.overall).toBe(70);
  });

  it("assess marks running; done reloads", async () => {
    ensureAssessListener();
    await useAssessStore.getState().assess(2);
    expect(useAssessStore.getState().status[2]).toBe("running");

    ctrl.a = sample;
    ctrl.handler!({ type: "done", project_id: 2, overall: 70 });
    expect(useAssessStore.getState().status[2]).toBe("idle");
    await vi.waitFor(() => expect(useAssessStore.getState().assessments[2]?.overall).toBe(70));
  });

  it("a failed event surfaces the error", () => {
    ensureAssessListener();
    ctrl.handler!({ type: "failed", project_id: 3, detail: "scan failed" });
    expect(useAssessStore.getState().status[3]).toBe("error");
    expect(useAssessStore.getState().error[3]).toBe("scan failed");
  });
});

describe("assessStore error paths (review fixes)", () => {
  it("assess() surfaces a rejected command as an error", async () => {
    (assessProject as any).mockRejectedValueOnce("Clone the repo first, then assess.");
    await useAssessStore.getState().assess(5);
    expect(useAssessStore.getState().status[5]).toBe("error");
    expect(useAssessStore.getState().error[5]).toMatch(/Clone the repo/);
  });

  it("load() surfaces a rejected getAssessment as an error (no silent spinner)", async () => {
    (getAssessment as any).mockRejectedValueOnce("db locked");
    await useAssessStore.getState().load(6);
    expect(useAssessStore.getState().status[6]).toBe("error");
    expect(useAssessStore.getState().error[6]).toMatch(/db locked/);
  });

  it("in-flight guard: a second assess() while running is a no-op", async () => {
    useAssessStore.setState({ status: { 7: "running" } });
    await useAssessStore.getState().assess(7);
    expect(assessProject).not.toHaveBeenCalledWith(7);
  });
});

describe("StatePane render", () => {
  it("renders the assessment with numbers, fixes, and cleanup", () => {
    useAssessStore.setState({ assessments: { 9: fullSample }, status: {}, progress: {}, error: {} });
    render(<StatePane project={project()} />);
    expect(screen.getByText("78")).toBeTruthy(); // overall
    expect(screen.getByText("Architecture")).toBeTruthy();
    expect(screen.getByText("Security")).toBeTruthy();
    expect(screen.getByText("Add CI")).toBeTruthy();
    expect(screen.getByText("No obvious cruft")).toBeTruthy();
  });

  it("shows the empty state when there is no assessment", () => {
    useAssessStore.setState({ assessments: { 9: null }, status: {}, progress: {}, error: {} });
    render(<StatePane project={project()} />);
    expect(screen.getByText(/No assessment yet/i)).toBeTruthy();
  });

  it("shows an alert error view on error status", () => {
    useAssessStore.setState({
      assessments: { 9: null },
      status: { 9: "error" },
      progress: {},
      error: { 9: "Claude not available" },
    });
    render(<StatePane project={project()} />);
    expect(screen.getByRole("alert").textContent).toMatch(/Claude not available/);
  });
});
