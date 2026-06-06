import { create } from "zustand";
import {
  analyzeProject,
  kickoffProject,
  getPlan,
  onAnalysisEvent,
  type PlanView,
  type AnalysisEvent,
} from "../api/analysis";

type AnalysisState = "idle" | "running" | "error";

interface PlanStore {
  // undefined = not loaded; null = loaded but no plan; PlanView = a plan
  plans: Record<number, PlanView | null | undefined>;
  analysis: Record<number, AnalysisState>;
  progress: Record<number, string[]>;
  error: Record<number, string | null>;
  loadPlan: (id: number) => Promise<void>;
  analyze: (id: number) => Promise<void>;
  kickoff: (id: number, description: string) => Promise<void>;
}

export const usePlanStore = create<PlanStore>((set, get) => ({
  plans: {},
  analysis: {},
  progress: {},
  error: {},

  loadPlan: async (id) => {
    try {
      const plan = await getPlan(id);
      set((s) => ({ plans: { ...s.plans, [id]: plan } }));
    } catch (e) {
      // Surface an error state — never leave a silent spinner.
      set((s) => ({ analysis: { ...s.analysis, [id]: "error" }, error: { ...s.error, [id]: String(e) } }));
    }
  },

  analyze: async (id) => {
    if (get().analysis[id] === "running") return; // don't double-spend a run
    set((s) => ({
      analysis: { ...s.analysis, [id]: "running" },
      progress: { ...s.progress, [id]: [] },
      error: { ...s.error, [id]: null },
    }));
    try {
      await analyzeProject(id);
    } catch (e) {
      set((s) => ({
        analysis: { ...s.analysis, [id]: "error" },
        error: { ...s.error, [id]: String(e) },
      }));
    }
  },

  kickoff: async (id, description) => {
    if (get().analysis[id] === "running") return; // don't double-spend a run
    set((s) => ({
      analysis: { ...s.analysis, [id]: "running" },
      progress: { ...s.progress, [id]: [] },
      error: { ...s.error, [id]: null },
    }));
    try {
      await kickoffProject(id, description);
    } catch (e) {
      set((s) => ({
        analysis: { ...s.analysis, [id]: "error" },
        error: { ...s.error, [id]: String(e) },
      }));
    }
  },
}));

function handle(e: AnalysisEvent) {
  const id = e.project_id;
  switch (e.type) {
    case "started":
      usePlanStore.setState((s) => ({
        analysis: { ...s.analysis, [id]: "running" },
        progress: { ...s.progress, [id]: [] },
      }));
      break;
    case "tool":
      usePlanStore.setState((s) => ({
        progress: { ...s.progress, [id]: [...(s.progress[id] ?? []), e.name] },
      }));
      break;
    case "done":
      usePlanStore.setState((s) => ({ analysis: { ...s.analysis, [id]: "idle" } }));
      void usePlanStore.getState().loadPlan(id);
      break;
    case "failed":
      usePlanStore.setState((s) => ({
        analysis: { ...s.analysis, [id]: "error" },
        error: { ...s.error, [id]: e.detail },
      }));
      break;
  }
}

let wired = false;
export function ensureAnalysisListener() {
  if (wired) return;
  wired = true;
  onAnalysisEvent(handle).catch(() => {
    wired = false;
  });
}
