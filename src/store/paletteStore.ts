import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import {
  DEFAULT_PALETTE,
  PRESETS,
  derivedTokens,
  randomPalette,
  type Palette,
  type RoleKey,
} from "../palette/generator";

/** CSS variables we set/unset when previewing a palette live on the app. */
const TOKEN_KEYS = Object.keys(derivedTokens(DEFAULT_PALETTE));

function writeTokens(p: Palette) {
  const root = document.documentElement;
  for (const [k, v] of Object.entries(derivedTokens(p))) root.style.setProperty(k, v);
}

function clearTokens() {
  const root = document.documentElement;
  for (const k of TOKEN_KEYS) root.style.removeProperty(k);
}

function nextSeed(): number {
  return Math.floor(Math.random() * 0xffffffff);
}

interface PaletteState {
  palette: Palette;
  seed: number; // drives the generative icon composition
  applied: boolean; // is this palette currently previewed live on the app?
  setRole: (role: RoleKey, hex: string) => void;
  setPalette: (p: Palette) => void;
  loadPreset: (name: string) => void;
  randomize: () => void;
  regenerate: () => void;
  applyToApp: () => void;
  resetApp: () => void;
}

/**
 * Palette-planner state. The chosen palette + generative seed persist across
 * launches; the live "applied to app" preview does not (it resets on relaunch
 * so the app never starts up wearing a half-finished palette). Whenever the
 * palette changes while applied, the live tokens re-render immediately.
 */
export const usePaletteStore = create<PaletteState>()(
  persist(
    (set, get) => {
      // Re-apply live tokens after a palette change if a preview is active.
      const refresh = (palette: Palette) => {
        if (get().applied) writeTokens(palette);
      };
      return {
        palette: DEFAULT_PALETTE,
        seed: 1,
        applied: false,

        setRole: (role, hex) =>
          set((s) => {
            const palette = { ...s.palette, [role]: hex };
            refresh(palette);
            return { palette };
          }),

        setPalette: (palette) => {
          refresh(palette);
          set({ palette });
        },

        loadPreset: (name) => {
          const found = PRESETS.find((p) => p.name === name);
          if (!found) return;
          refresh(found.palette);
          set({ palette: found.palette, seed: nextSeed() });
        },

        randomize: () => {
          const seed = nextSeed();
          const palette = randomPalette(seed);
          refresh(palette);
          set({ palette, seed });
        },

        regenerate: () => set({ seed: nextSeed() }),

        applyToApp: () => {
          writeTokens(get().palette);
          set({ applied: true });
        },

        resetApp: () => {
          clearTokens();
          set({ applied: false });
        },
      };
    },
    {
      name: "review-helper.palette",
      storage: createJSONStorage(() => localStorage),
      // Persist the design, not the transient live-preview flag.
      partialize: (s) => ({ palette: s.palette, seed: s.seed }),
    },
  ),
);
