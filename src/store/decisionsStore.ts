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

  // Reloads run only on success — a failure surfaces an error instead of being
  // erased by a subsequent successful reload that clears it (loadPending resets
  // error to null on success).
  approve: async (id, suggestionId) => {
    try {
      await suggestionApprove(id, suggestionId);
      await get().loadPending(id);
      await get().loadDecisions(id);
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
  },

  dismiss: async (id, suggestionId) => {
    try {
      await suggestionDismiss(id, suggestionId);
      await get().loadPending(id);
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
  },

  approveAll: async (id) => {
    try {
      await suggestionsApproveAll(id);
      await get().loadPending(id);
      await get().loadDecisions(id);
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
  },

  supersede: async (id, decisionId) => {
    try {
      await decisionSupersede(id, decisionId);
      await get().loadDecisions(id);
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
  },
}));
