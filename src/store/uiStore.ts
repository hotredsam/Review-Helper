import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import { type SectionId, DEFAULT_SECTION } from "../nav/sections";

interface UiState {
  sidebarCollapsed: boolean;
  activeSection: SectionId;
  tourOpen: boolean; // ephemeral (not persisted)
  toggleSidebar: () => void;
  setSection: (id: SectionId) => void;
  setTourOpen: (open: boolean) => void;
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
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
      setSection: (id) => set({ activeSection: id }),
      setTourOpen: (open) => set({ tourOpen: open }),
    }),
    {
      name: "review-helper.ui",
      storage: createJSONStorage(() => localStorage),
      // Persist only layout — not the transient tour flag.
      partialize: (s) => ({ sidebarCollapsed: s.sidebarCollapsed, activeSection: s.activeSection }),
    },
  ),
);
