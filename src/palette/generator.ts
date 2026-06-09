/**
 * Palette model + the generative icon composer. A `Palette` is five color roles;
 * `composeIcon` turns a palette + seed into a simplistic, app-icon-style SVG
 * composition (which template, and how the colors are distributed across its
 * shapes) — the "generative UI box". `derivedTokens` expands the five roles into
 * the full app theme-token set so a palette can be previewed live on the app.
 */
import { hslToHex, isLight, mixHex, readableTextHex } from "./color";

export type RoleKey = "bg" | "surface" | "text" | "primary" | "accent";

export interface Palette {
  bg: string;
  surface: string;
  text: string;
  primary: string;
  accent: string;
}

export const ROLES: { key: RoleKey; label: string; hint: string }[] = [
  { key: "bg", label: "Background", hint: "The base canvas" },
  { key: "surface", label: "Surface", hint: "Cards & panels" },
  { key: "text", label: "Text", hint: "Primary foreground" },
  { key: "primary", label: "Primary", hint: "Brand / main action" },
  { key: "accent", label: "Accent", hint: "Secondary highlight" },
];

export const DEFAULT_PALETTE: Palette = {
  bg: "#0f1424",
  surface: "#1b2440",
  text: "#e8ecff",
  primary: "#6ea8fe",
  accent: "#f4a259",
};

export const PRESETS: { name: string; palette: Palette }[] = [
  { name: "Nocturne", palette: DEFAULT_PALETTE },
  { name: "Sunset", palette: { bg: "#1c1014", surface: "#2a181d", text: "#ffe9e3", primary: "#ff6b6b", accent: "#ffd166" } },
  { name: "Forest", palette: { bg: "#0e1a14", surface: "#16271e", text: "#e7f3ec", primary: "#34d399", accent: "#a3e635" } },
  { name: "Ocean", palette: { bg: "#08171f", surface: "#0f2630", text: "#e2f3fa", primary: "#22d3ee", accent: "#38bdf8" } },
  { name: "Candy", palette: { bg: "#fef6fb", surface: "#ffffff", text: "#2a1a24", primary: "#ec4899", accent: "#8b5cf6" } },
  { name: "Mono", palette: { bg: "#111315", surface: "#1c1f22", text: "#f2f4f6", primary: "#9aa3ad", accent: "#cbd3da" } },
];

/** mulberry32 — a tiny, fast, deterministic PRNG so the same seed always yields
 *  the same composition (stable rendering + testable). */
export function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function shuffle<T>(arr: readonly T[], r: () => number): T[] {
  const a = arr.slice();
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(r() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

/** A harmonious, random-but-coherent palette (seeded). */
export function randomPalette(seed: number): Palette {
  const r = rng(seed);
  const dark = r() > 0.5;
  const baseHue = Math.floor(r() * 360);
  const primHue = (baseHue + 180 + Math.floor(r() * 80)) % 360;
  const accHue = (primHue + 120 + Math.floor(r() * 80)) % 360;
  return dark
    ? {
        bg: hslToHex(baseHue, 0.22, 0.09),
        surface: hslToHex(baseHue, 0.2, 0.16),
        text: hslToHex(baseHue, 0.15, 0.95),
        primary: hslToHex(primHue, 0.7, 0.62),
        accent: hslToHex(accHue, 0.75, 0.6),
      }
    : {
        bg: hslToHex(baseHue, 0.32, 0.97),
        surface: hslToHex(baseHue, 0.4, 0.995),
        text: hslToHex(baseHue, 0.28, 0.12),
        primary: hslToHex(primHue, 0.68, 0.46),
        accent: hslToHex(accHue, 0.7, 0.5),
      };
}

// ---- Generative icon composition ------------------------------------------

export interface LinearGradient {
  id: string;
  angle: number; // degrees
  stops: { offset: number; color: string }[];
}

export type Shape =
  | { kind: "rect"; x: number; y: number; w: number; h: number; rx?: number; fill: string }
  | { kind: "circle"; cx: number; cy: number; r: number; fill: string }
  | { kind: "poly"; points: [number, number][]; fill: string };

export interface IconComposition {
  template: string;
  defs: LinearGradient[];
  shapes: Shape[]; // rendered in order; shapes[0] is the full-bleed base
}

type Template = (p: Palette, r: () => number, uid: string) => IconComposition;

const base = (fill: string): Shape => ({ kind: "rect", x: 0, y: 0, w: 100, h: 100, fill });

const gradientOrb: Template = (p, r, uid) => {
  const [c1, c2, c3] = shuffle([p.primary, p.accent, p.surface, p.text], r);
  const id = `${uid}-grad`;
  return {
    template: "gradient-orb",
    defs: [{ id, angle: 135, stops: [{ offset: 0, color: c1 }, { offset: 1, color: mixHex(c1, c2, 0.6) }] }],
    shapes: [
      base(`url(#${id})`),
      { kind: "circle", cx: 50, cy: 54, r: 27, fill: c3 },
      { kind: "circle", cx: 68, cy: 34, r: 9, fill: c2 },
    ],
  };
};

const diagonalSplit: Template = (p, r) => {
  const [c1, c2, c3] = shuffle([p.primary, p.accent, p.surface, p.text], r);
  return {
    template: "diagonal-split",
    defs: [],
    shapes: [
      base(c1),
      { kind: "poly", points: [[0, 100], [100, 0], [100, 100]], fill: c2 },
      { kind: "circle", cx: 50, cy: 50, r: 18, fill: c3 },
    ],
  };
};

const concentric: Template = (p, r) => {
  const [c1, c2, c3, c4] = shuffle([p.primary, p.accent, p.surface, p.text], r);
  return {
    template: "concentric",
    defs: [],
    shapes: [
      base(c1),
      { kind: "rect", x: 16, y: 16, w: 68, h: 68, rx: 18, fill: c2 },
      { kind: "rect", x: 30, y: 30, w: 40, h: 40, rx: 12, fill: c3 },
      { kind: "circle", cx: 50, cy: 50, r: 9, fill: c4 },
    ],
  };
};

const bars: Template = (p, r) => {
  const [c1, ...rest] = shuffle([p.bg, p.surface, p.primary, p.accent, p.text], r);
  const cols = rest.slice(0, 3);
  const heights = [54, 70, 40];
  return {
    template: "bars",
    defs: [],
    shapes: [
      base(c1),
      ...cols.map((fill, i): Shape => ({
        kind: "rect",
        x: 22 + i * 20,
        y: 80 - heights[i],
        w: 13,
        h: heights[i],
        rx: 6,
        fill,
      })),
    ],
  };
};

const cornerArc: Template = (p, r, uid) => {
  const [c1, c2, c3] = shuffle([p.primary, p.accent, p.surface, p.text], r);
  const id = `${uid}-arc`;
  return {
    template: "corner-arc",
    defs: [{ id, angle: 160, stops: [{ offset: 0, color: c1 }, { offset: 1, color: mixHex(c1, c3, 0.5) }] }],
    shapes: [
      base(`url(#${id})`),
      { kind: "circle", cx: 96, cy: 96, r: 46, fill: c2 },
      { kind: "circle", cx: 34, cy: 34, r: 11, fill: c3 },
    ],
  };
};

const stack: Template = (p, r) => {
  const [c1, c2, c3] = shuffle([p.bg, p.surface, p.primary, p.accent], r);
  return {
    template: "stack",
    defs: [],
    shapes: [
      base(c1),
      { kind: "rect", x: 24, y: 34, w: 46, h: 46, rx: 12, fill: c2 },
      { kind: "rect", x: 38, y: 22, w: 46, h: 46, rx: 12, fill: c3 },
    ],
  };
};

const TEMPLATES: Template[] = [gradientOrb, diagonalSplit, concentric, bars, cornerArc, stack];

/** Pick a template by seed and distribute the palette colors across it. `uid`
 *  scopes any gradient ids so multiple icons on the page never collide. */
export function composeIcon(palette: Palette, seed: number, uid: string): IconComposition {
  const r = rng(seed);
  const idx = Math.floor(r() * TEMPLATES.length);
  return TEMPLATES[idx](palette, r, uid);
}

// ---- App theme-token derivation (for live "apply to app" preview) ----------

/** The full app token set derived from the five palette roles. Keys match the
 *  CSS variables defined in theme/themes.css. */
export function derivedTokens(p: Palette): Record<string, string> {
  const light = isLight(p.bg);
  return {
    "--bg": p.bg,
    "--surface": p.surface,
    "--surface-2": mixHex(p.surface, p.text, 0.07),
    "--overlay": p.surface,
    "--fg": p.text,
    "--fg-muted": mixHex(p.text, p.bg, 0.32),
    "--fg-subtle": mixHex(p.text, p.bg, 0.5),
    "--border": mixHex(p.surface, p.text, 0.14),
    "--border-strong": mixHex(p.surface, p.text, 0.26),
    "--accent": p.primary,
    "--accent-fg": readableTextHex(p.primary),
    "--accent-hover": mixHex(p.primary, p.text, 0.18),
    "--success": "#22c55e",
    "--success-fg": readableTextHex("#22c55e"),
    "--warning": "#f59e0b",
    "--warning-fg": readableTextHex("#f59e0b"),
    "--danger": "#ef4444",
    "--danger-fg": readableTextHex("#ef4444"),
    "--ring": p.primary,
    "--scrim": light ? "rgba(20, 18, 28, 0.40)" : "rgba(0, 0, 0, 0.55)",
  };
}
