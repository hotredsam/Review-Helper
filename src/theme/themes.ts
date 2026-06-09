/**
 * Theme registry — the config the UI reads to render the switcher. The color
 * values live in themes.css; this file is the externalized list of themes plus
 * the shared persistence key. Add a theme by adding a row here AND a matching
 * [data-theme="id"] block in themes.css.
 */
export type ThemeId =
  | "light"
  | "dark"
  | "midnight"
  | "sand"
  | "nord"
  | "forest"
  | "rose"
  | "grape";

export interface ThemeMeta {
  id: ThemeId;
  label: string;
}

export const THEMES: ThemeMeta[] = [
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
  { id: "midnight", label: "Midnight" },
  { id: "sand", label: "Sand" },
  { id: "nord", label: "Nord" },
  { id: "forest", label: "Forest" },
  { id: "rose", label: "Rose" },
  { id: "grape", label: "Grape" },
];

export const DEFAULT_THEME: ThemeId = "light";

/**
 * localStorage key shared by the Zustand persist store (themeStore.ts) and the
 * anti-flash inline script in index.html. Keep all three in sync if it changes.
 */
export const THEME_STORAGE_KEY = "review-helper.theme";

export function isThemeId(value: unknown): value is ThemeId {
  return THEMES.some((t) => t.id === value);
}
