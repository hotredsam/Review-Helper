import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import { type SectionId, DEFAULT_SECTION } from "../nav/sections";

interface UiState {
  sidebarCollapsed: boolean;
  activeSection: SectionId;
  toggleSidebar: () => void;
  setSection: (id: SectionId) => void;
}

/**
 * Shell UI state (sidebar collapse + active section), persisted so the layout
 * is restored on the next launch.
 */
export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      sidebarCollapsed: false,
      activeSection: DEFAULT_SECTION,
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
      setSection: (id) => set({ activeSection: id }),
    }),
    {
      name: "review-helper.ui",
      storage: createJSONStorage(() => localStorage),
    },
  ),
);
