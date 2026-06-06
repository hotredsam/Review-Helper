import { create } from "zustand";
import { featuresList, featureAdd, featureSetStatus, type Feature } from "../api/features";

interface FeaturesStore {
  features: Record<number, Feature[] | undefined>;
  error: Record<number, string | null>;
  load: (id: number) => Promise<void>;
  add: (id: number, title: string, detail: string, source?: string) => Promise<void>;
  setStatus: (id: number, featureId: number, status: string) => Promise<void>;
}

export const useFeaturesStore = create<FeaturesStore>((set, get) => ({
  features: {},
  error: {},

  load: async (id) => {
    try {
      const fs = await featuresList(id);
      set((s) => ({ features: { ...s.features, [id]: fs }, error: { ...s.error, [id]: null } }));
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
  },

  add: async (id, title, detail, source) => {
    try {
      await featureAdd(id, title, detail, source);
      await get().load(id);
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
  },

  setStatus: async (id, featureId, status) => {
    try {
      await featureSetStatus(id, featureId, status);
      await get().load(id);
    } catch (e) {
      set((s) => ({ error: { ...s.error, [id]: String(e) } }));
    }
  },
}));
