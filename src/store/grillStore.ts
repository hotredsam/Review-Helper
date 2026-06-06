import { create } from "zustand";
import {
  grillGenerate,
  grillList,
  grillAnswer,
  grillChatResolve,
  grillSetStatus,
  grillDelete,
  onGrillEvent,
  type Question,
  type GrillEvent,
} from "../api/grill";

type Status = "idle" | "running" | "error";

interface GrillStore {
  questions: Record<number, Question[] | undefined>;
  status: Record<number, Status>;
  progress: Record<number, string[]>;
  error: Record<number, string | null>;
  depth: Record<number, number>;
  load: (id: number) => Promise<void>;
  generate: (id: number, depth: number) => Promise<void>;
  setDepth: (id: number, depth: number) => void;
  // Per-card actions: mutate then reload so the card moves to addressed. They
  // throw on failure so the card can show a local error (no pane-wide error).
  answer: (id: number, questionId: number, body: string) => Promise<void>;
  chatResolve: (id: number, questionId: number, resolution: string) => Promise<void>;
  setStatus: (id: number, questionId: number, status: string) => Promise<void>;
  remove: (id: number, questionId: number) => Promise<void>;
}

export const useGrillStore = create<GrillStore>((set, get) => ({
  questions: {},
  status: {},
  progress: {},
  error: {},
  depth: {},

  load: async (id) => {
    try {
      const qs = await grillList(id);
      set((s) => ({ questions: { ...s.questions, [id]: qs } }));
    } catch (e) {
      // Surface an error — never leave a silent spinner.
      set((s) => ({ status: { ...s.status, [id]: "error" }, error: { ...s.error, [id]: String(e) } }));
    }
  },

  generate: async (id, depth) => {
    if (get().status[id] === "running") return; // don't double-spend a run
    set((s) => ({
      status: { ...s.status, [id]: "running" },
      progress: { ...s.progress, [id]: [] },
      error: { ...s.error, [id]: null },
    }));
    try {
      await grillGenerate(id, depth);
    } catch (e) {
      set((s) => ({ status: { ...s.status, [id]: "error" }, error: { ...s.error, [id]: String(e) } }));
    }
  },

  setDepth: (id, depth) => set((s) => ({ depth: { ...s.depth, [id]: depth } })),

  answer: async (id, questionId, body) => {
    await grillAnswer(id, questionId, body);
    await get().load(id);
  },
  chatResolve: async (id, questionId, resolution) => {
    await grillChatResolve(id, questionId, resolution);
    await get().load(id);
  },
  setStatus: async (id, questionId, status) => {
    await grillSetStatus(id, questionId, status);
    await get().load(id);
  },
  remove: async (id, questionId) => {
    await grillDelete(id, questionId);
    await get().load(id);
  },
}));

function handle(e: GrillEvent) {
  const id = e.project_id;
  switch (e.type) {
    case "started":
      useGrillStore.setState((s) => ({
        status: { ...s.status, [id]: "running" },
        progress: { ...s.progress, [id]: [] },
      }));
      break;
    case "tool":
      useGrillStore.setState((s) => ({
        progress: { ...s.progress, [id]: [...(s.progress[id] ?? []), e.name] },
      }));
      break;
    case "done":
      useGrillStore.setState((s) => ({ status: { ...s.status, [id]: "idle" } }));
      void useGrillStore.getState().load(id);
      break;
    case "failed":
      useGrillStore.setState((s) => ({
        status: { ...s.status, [id]: "error" },
        error: { ...s.error, [id]: e.detail },
      }));
      break;
  }
}

let wired = false;
export function ensureGrillListener() {
  if (wired) return;
  wired = true;
  onGrillEvent(handle).catch(() => {
    wired = false;
  });
}
