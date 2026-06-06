import { create } from "zustand";
import {
  assessProject,
  getAssessment,
  onAssessmentEvent,
  type AssessmentView,
  type AssessmentEvent,
} from "../api/assessment";

type Status = "idle" | "running" | "error";

interface AssessStore {
  assessments: Record<number, AssessmentView | null | undefined>;
  status: Record<number, Status>;
  progress: Record<number, string[]>;
  error: Record<number, string | null>;
  load: (id: number) => Promise<void>;
  assess: (id: number) => Promise<void>;
}

export const useAssessStore = create<AssessStore>((set, get) => ({
  assessments: {},
  status: {},
  progress: {},
  error: {},

  load: async (id) => {
    try {
      const a = await getAssessment(id);
      set((s) => ({ assessments: { ...s.assessments, [id]: a } }));
    } catch (e) {
      // Surface an error state — never leave a silent spinner.
      set((s) => ({ status: { ...s.status, [id]: "error" }, error: { ...s.error, [id]: String(e) } }));
    }
  },

  assess: async (id) => {
    if (get().status[id] === "running") return; // don't double-spend a run
    set((s) => ({
      status: { ...s.status, [id]: "running" },
      progress: { ...s.progress, [id]: [] },
      error: { ...s.error, [id]: null },
    }));
    try {
      await assessProject(id);
    } catch (e) {
      set((s) => ({ status: { ...s.status, [id]: "error" }, error: { ...s.error, [id]: String(e) } }));
    }
  },
}));

function handle(e: AssessmentEvent) {
  const id = e.project_id;
  switch (e.type) {
    case "started":
      useAssessStore.setState((s) => ({
        status: { ...s.status, [id]: "running" },
        progress: { ...s.progress, [id]: [] },
      }));
      break;
    case "tool":
      useAssessStore.setState((s) => ({
        progress: { ...s.progress, [id]: [...(s.progress[id] ?? []), e.name] },
      }));
      break;
    case "done":
      useAssessStore.setState((s) => ({ status: { ...s.status, [id]: "idle" } }));
      void useAssessStore.getState().load(id);
      break;
    case "failed":
      useAssessStore.setState((s) => ({
        status: { ...s.status, [id]: "error" },
        error: { ...s.error, [id]: e.detail },
      }));
      break;
  }
}

let wired = false;
export function ensureAssessListener() {
  if (wired) return;
  wired = true;
  onAssessmentEvent(handle).catch(() => {
    wired = false;
  });
}
