import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import { DEFAULT_THEME, THEME_STORAGE_KEY, type ThemeId } from "./themes";

interface ThemeState {
  theme: ThemeId;
  setTheme: (theme: ThemeId) => void;
}

/**
 * Persisted theme store. Zustand's persist middleware writes the choice to
 * localStorage, which the Tauri webview keeps on disk across app restarts —
 * so the selected theme survives a relaunch.
 */
export const useThemeStore = create<ThemeState>()(
  persist(
    (set) => ({
      theme: DEFAULT_THEME,
      setTheme: (theme) => set({ theme }),
    }),
    {
      name: THEME_STORAGE_KEY,
      storage: createJSONStorage(() => localStorage),
      partialize: (s) => ({ theme: s.theme }),
    },
  ),
);

/**
 * Reflect the active theme onto <html data-theme> so the token blocks in
 * themes.css take effect across the entire window (not just React-rendered DOM).
 */
function applyTheme(theme: ThemeId) {
  document.documentElement.setAttribute("data-theme", theme);
}

// localStorage is synchronous, so the persisted value is already hydrated here.
// Apply once on load (covers the case where the inline anti-flash script in
// index.html didn't run) and then on every change.
applyTheme(useThemeStore.getState().theme);
useThemeStore.subscribe((state) => applyTheme(state.theme));
