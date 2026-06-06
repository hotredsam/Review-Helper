import { describe, it, expect, beforeEach, vi } from "vitest";

const ctrl = vi.hoisted(() => ({
  plan: null as any,
  handler: null as null | ((e: any) => void),
}));

vi.mock("../api/analysis", () => ({
  getPlan: vi.fn(async () => ctrl.plan),
  analyzeProject: vi.fn(async () => {}),
  kickoffProject: vi.fn(async () => {}),
  onAnalysisEvent: vi.fn(async (h: (e: any) => void) => {
    ctrl.handler = h;
    return () => {};
  }),
}));

import { usePlanStore, ensureAnalysisListener } from "../store/planStore";

const aPlan = {
  version: 1,
  current_state: "y",
  body_md: null,
  phases: [],
  decisions: [],
  stack: [],
};

beforeEach(() => {
  ctrl.plan = null;
  usePlanStore.setState({ plans: {}, analysis: {}, progress: {}, error: {} });
});

describe("planStore", () => {
  it("loads a plan for a project", async () => {
    ctrl.plan = aPlan;
    await usePlanStore.getState().loadPlan(1);
    expect(usePlanStore.getState().plans[1]?.version).toBe(1);
  });

  it("analyze marks running; a done event flips to idle and reloads the plan", async () => {
    ensureAnalysisListener();
    await usePlanStore.getState().analyze(2);
    expect(usePlanStore.getState().analysis[2]).toBe("running");

    ctrl.plan = aPlan;
    ctrl.handler!({ type: "done", project_id: 2, version: 1, confidence: "low", phases: 0 });
    expect(usePlanStore.getState().analysis[2]).toBe("idle");
    await vi.waitFor(() => expect(usePlanStore.getState().plans[2]?.version).toBe(1));
  });

  it("a failed event surfaces the error", () => {
    ensureAnalysisListener();
    ctrl.handler!({ type: "failed", project_id: 3, detail: "invalid plan json" });
    expect(usePlanStore.getState().analysis[3]).toBe("error");
    expect(usePlanStore.getState().error[3]).toBe("invalid plan json");
  });

  it("accumulates tool progress while analyzing", () => {
    ensureAnalysisListener();
    ctrl.handler!({ type: "started", project_id: 4 });
    ctrl.handler!({ type: "tool", project_id: 4, name: "Read package.json" });
    ctrl.handler!({ type: "tool", project_id: 4, name: "Glob" });
    expect(usePlanStore.getState().progress[4]).toEqual(["Read package.json", "Glob"]);
  });
});
