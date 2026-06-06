import { describe, it, expect, beforeEach, vi } from "vitest";

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
