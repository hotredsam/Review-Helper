import { create } from "zustand";
import {
  analyzeProject,
  kickoffProject,
  updateProject,
  rebuildProject,
  getPlan,
  auditList,
  onAnalysisEvent,
  type PlanView,
  type AnalysisEvent,
  type AuditEntry,
} from "../api/analysis";
import { useFeaturesStore } from "./featuresStore";

type AnalysisState = "idle" | "running" | "error";

interface PlanStore {
  // undefined = not loaded; null = loaded but no plan; PlanView = a plan
  plans: Record<number, PlanView | null | undefined>;
  analysis: Record<number, AnalysisState>;
  progress: Record<number, string[]>;
  error: Record<number, string | null>;
  audit: Record<number, AuditEntry[]>;
  loadPlan: (id: number) => Promise<void>;
  loadAudit: (id: number) => Promise<void>;
  analyze: (id: number) => Promise<void>;
  kickoff: (id: number, description: string) => Promise<void>;
  update: (id: number) => Promise<void>;
  rebuild: (id: number) => Promise<void>;
}

export const usePlanStore = create<PlanStore>((set, get) => ({
  plans: {},
  analysis: {},
  progress: {},
  error: {},
  audit: {},

  loadAudit: async (id) => {
    try {
      const entries = await auditList(id);
      set((s) => ({ audit: { ...s.audit, [id]: entries } }));
    } catch {
      // non-fatal: the history panel just stays empty
    }
  },

  rebuild: async (id) => {
    if (get().analysis[id] === "running") return;
    set((s) => ({
      analysis: { ...s.analysis, [id]: "running" },
      progress: { ...s.progress, [id]: [] },
      error: { ...s.error, [id]: null },
    }));
    try {
      await rebuildProject(id);
    } catch (e) {
      set((s) => ({ analysis: { ...s.analysis, [id]: "error" }, error: { ...s.error, [id]: String(e) } }));
    }
  },

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

  update: async (id) => {
    if (get().analysis[id] === "running") return; // don't double-spend a run
    set((s) => ({
      analysis: { ...s.analysis, [id]: "running" },
      progress: { ...s.progress, [id]: [] },
      error: { ...s.error, [id]: null },
    }));
    try {
      await updateProject(id);
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
      void usePlanStore.getState().loadAudit(id);
      // A merge may have marked inbox features in_plan — refresh the inbox too.
      void useFeaturesStore.getState().load(id);
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
