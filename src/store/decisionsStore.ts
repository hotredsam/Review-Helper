import { create } from "zustand";
import {
  suggestionsList,
  suggestionApprove,
  suggestionDismiss,
  suggestionsApproveAll,
  type Suggestion,
} from "../api/suggestions";
import { decisionsList, decisionSupersede, type Decision } from "../api/decisions";

interface DecisionsStore {
  pending: Record<number, Suggestion[] | undefined>;
  decisions: Record<number, Decision[] | undefined>;
  error: Record<number, string | null>;
  loadPending: (id: number) => Promise<void>;
  loadDecisions: (id: number) => Promise<void>;
  approve: (id: number, suggestionId: number) => Promise<void>;
  dismiss: (id: number, suggestionId: number) => Promise<void>;
  approveAll: (id: number) => Promise<void>;
  supersede: (id: number, decisionId: number) => Promise<void>;
}

export const useDecisionsStore = create<DecisionsStore>((set, get) => ({
  pending: {},
  decisions: {},
  error: {},

  loadPending: async (id) => {
    try {
      const ps = await suggestionsList(id, "pending");
      set((s) => ({ pending: { ...s.pending, [id]: ps }, error: { ...s.error, [id]: null } }));
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
  },

  loadDecisions: async (id) => {
    try {
      const ds = await decisionsList(id);
      set((s) => ({ decisions: { ...s.decisions, [id]: ds } }));
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
  },

  // Approving may add a decision/feature/stack/answer; refresh both lists.
  approve: async (id, suggestionId) => {
    try {
      await suggestionApprove(id, suggestionId);
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
    await get().loadPending(id);
    await get().loadDecisions(id);
  },

  dismiss: async (id, suggestionId) => {
    try {
      await suggestionDismiss(id, suggestionId);
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
    await get().loadPending(id);
  },

  approveAll: async (id) => {
    try {
      await suggestionsApproveAll(id);
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
    await get().loadPending(id);
    await get().loadDecisions(id);
  },

  supersede: async (id, decisionId) => {
    try {
      await decisionSupersede(id, decisionId);
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
    await get().loadDecisions(id);
  },
}));
