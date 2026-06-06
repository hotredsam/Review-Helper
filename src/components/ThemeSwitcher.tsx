import { THEMES } from "../theme/themes";
import { useThemeStore } from "../theme/themeStore";

/**
 * Compact theme picker. Lives in the shell for now; it will move into the nav /
 * Settings in a later phase. Fully token-styled, so it renders correctly in
 * every theme it switches between.
 */
export function ThemeSwitcher() {
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);

  return (
    <div
      role="radiogroup"
      aria-label="Theme"
      className="inline-flex gap-1 rounded-lg border border-border bg-surface p-1"
    >
      {THEMES.map((t) => {
        const active = t.id === theme;
        return (
          <button
            key={t.id}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => setTheme(t.id)}
            className={
              "rounded-md px-3 py-1.5 text-sm font-medium transition-colors " +
              (active
                ? "bg-accent text-accent-fg"
                : "text-fg-muted hover:bg-surface-2 hover:text-fg")
            }
          >
            {t.label}
          </button>
        );
      })}
    </div>
  );
}
