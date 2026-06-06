import { create } from "zustand";
import { getModelStatus, type ModelStatus } from "../api/model";

interface StatusState {
  status: ModelStatus | null;
  loading: boolean;
  refresh: () => Promise<void>;
}

/** Tracks model-provider availability. `refresh` re-probes (used on startup and
 *  by the banner's Retry / the debug panel's Recheck). */
export const useStatusStore = create<StatusState>((set) => ({
  status: null,
  loading: false,
  refresh: async () => {
    set({ loading: true });
    try {
      const status = await getModelStatus();
      set({ status, loading: false });
    } catch (e) {
      set({
        loading: false,
        status: {
          provider: "claude",
          available: false,
          version: null,
          reason: "unknown",
          command: "model_status",
          exit_code: null,
          stderr: String(e),
        },
      });
    }
  },
}));
