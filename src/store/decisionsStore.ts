import { create } from "zustand";
import {
  suggestionsList,
  suggestionApprove,
  suggestionDismiss,
  suggestionsApproveAll,
  type Suggestion,
} from "../api/suggestions";

interface DecisionsStore {
  pending: Record<number, Suggestion[] | undefined>;
  error: Record<number, string | null>;
  loadPending: (id: number) => Promise<void>;
  approve: (id: number, suggestionId: number) => Promise<void>;
  dismiss: (id: number, suggestionId: number) => Promise<void>;
  approveAll: (id: number) => Promise<void>;
}

export const useDecisionsStore = create<DecisionsStore>((set, get) => ({
  pending: {},
  error: {},

  loadPending: async (id) => {
    try {
      const ps = await suggestionsList(id, "pending");
      set((s) => ({ pending: { ...s.pending, [id]: ps }, error: { ...s.error, [id]: null } }));
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
  },

  approve: async (id, suggestionId) => {
    try {
      await suggestionApprove(id, suggestionId);
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
    await get().loadPending(id);
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
  },
}));
