import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import { type SectionId, DEFAULT_SECTION } from "../nav/sections";

interface UiState {
  sidebarCollapsed: boolean;
  activeSection: SectionId;
  tourOpen: boolean; // ephemeral (not persisted)
  notice: string | null; // ephemeral transient confirmation
  toggleSidebar: () => void;
  setSection: (id: SectionId) => void;
  setTourOpen: (open: boolean) => void;
  setNotice: (msg: string | null) => void;
}

/**
 * Shell UI state (sidebar collapse + active section), persisted so the layout
 * is restored on the next launch. `tourOpen` is ephemeral.
 */
export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      sidebarCollapsed: false,
      activeSection: DEFAULT_SECTION,
      tourOpen: false,
      notice: null,
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
      setSection: (id) => set({ activeSection: id }),
      setTourOpen: (open) => set({ tourOpen: open }),
      setNotice: (msg) => set({ notice: msg }),
    }),
    {
      name: "review-helper.ui",
      storage: createJSONStorage(() => localStorage),
      // Persist only layout — not the transient tour flag.
      partialize: (s) => ({ sidebarCollapsed: s.sidebarCollapsed, activeSection: s.activeSection }),
    },
  ),
);
