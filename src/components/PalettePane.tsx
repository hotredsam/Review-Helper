import { Shuffle, RefreshCw, Wand2, Check, RotateCcw, Sparkles } from "lucide-react";
import { usePaletteStore } from "../store/paletteStore";
import { ROLES, PRESETS, type RoleKey } from "../palette/generator";
import { isHex, parseHex, toHex } from "../palette/color";
import { PaletteIcon } from "./PaletteIcon";
import { PaletteMiniUI } from "./PaletteMiniUI";

/**
 * Palette planner: design a five-role color palette for a project's frontend and
 * see it rendered live — a generative app-icon composition (plus variants) and a
 * simplistic UI mock — then optionally preview it on the whole app. All chrome
 * uses theme tokens; only the preview swatches/icons use the raw palette colors.
 */
export function PalettePane() {
  const palette = usePaletteStore((s) => s.palette);
  const seed = usePaletteStore((s) => s.seed);
  const applied = usePaletteStore((s) => s.applied);
  const setRole = usePaletteStore((s) => s.setRole);
  const loadPreset = usePaletteStore((s) => s.loadPreset);
  const randomize = usePaletteStore((s) => s.randomize);
  const regenerate = usePaletteStore((s) => s.regenerate);
  const applyToApp = usePaletteStore((s) => s.applyToApp);
  const resetApp = usePaletteStore((s) => s.resetApp);

  // Variant seeds: a few alternative distributions of the same palette.
  const variants = [seed + 1, seed + 2, seed + 3];

  return (
    <div className="mx-auto max-w-4xl space-y-6 p-8">
      <div>
        <h2 className="flex items-center gap-2 text-sm font-semibold text-fg">
          <Sparkles className="h-4 w-4 text-accent" /> Color palette planner
        </h2>
        <p className="mt-1 max-w-2xl text-sm text-fg-muted">
          Plan a frontend color theme and see how the colors sit together — a generative
          app-icon composition and a simplistic UI mock, both rendered in your palette. Regenerate
          to redistribute the colors, or preview the palette on the whole app.
        </p>
      </div>

      <div className="grid gap-6 md:grid-cols-[minmax(0,1fr)_auto]">
        {/* ---- Editor ---- */}
        <section className="space-y-4">
          <div className="space-y-2">
            {ROLES.map((role) => (
              <RoleRow
                key={role.key}
                roleKey={role.key}
                label={role.label}
                hint={role.hint}
                value={palette[role.key]}
                onChange={(hex) => setRole(role.key, hex)}
              />
            ))}
          </div>

          <div className="space-y-2">
            <h3 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Presets</h3>
            <div className="flex flex-wrap gap-2">
              {PRESETS.map((p) => (
                <button
                  key={p.name}
                  type="button"
                  onClick={() => loadPreset(p.name)}
                  className="flex items-center gap-1.5 rounded-md border border-border px-2 py-1 text-xs text-fg-muted hover:bg-surface-2"
                >
                  <span className="flex -space-x-1">
                    {[p.palette.bg, p.palette.primary, p.palette.accent].map((c, i) => (
                      <span
                        key={i}
                        className="h-3 w-3 rounded-full border border-border"
                        style={{ background: c }}
                      />
                    ))}
                  </span>
                  {p.name}
                </button>
              ))}
            </div>
          </div>

          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              onClick={randomize}
              className="flex items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-accent-fg hover:bg-accent-hover"
            >
              <Wand2 className="h-3.5 w-3.5" /> Randomize palette
            </button>
            {applied ? (
              <button
                type="button"
                onClick={resetApp}
                className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-xs text-fg-muted hover:bg-surface-2"
              >
                <RotateCcw className="h-3.5 w-3.5" /> Reset app theme
              </button>
            ) : (
              <button
                type="button"
                onClick={applyToApp}
                className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-xs text-fg-muted hover:bg-surface-2"
              >
                <Check className="h-3.5 w-3.5" /> Preview on app
              </button>
            )}
          </div>
          {applied && (
            <p className="text-xs text-warning" role="status">
              Previewing this palette live on the app — “Reset app theme” to restore your theme.
            </p>
          )}
        </section>

        {/* ---- Generative preview ---- */}
        <section className="flex flex-col items-center gap-4">
          <PaletteIcon palette={palette} seed={seed} size={200} label="Generated app icon from palette" />
          <button
            type="button"
            onClick={regenerate}
            className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-xs text-fg-muted hover:bg-surface-2"
          >
            <RefreshCw className="h-3.5 w-3.5" /> Regenerate
          </button>
          <div className="flex items-center gap-2" aria-label="Alternative color distributions">
            {variants.map((v) => (
              <PaletteIcon key={v} palette={palette} seed={v} size={52} label="Alternative distribution" />
            ))}
            <Shuffle className="h-4 w-4 text-fg-subtle" />
          </div>
          <PaletteMiniUI palette={palette} />
        </section>
      </div>
    </div>
  );
}

function RoleRow({
  roleKey,
  label,
  hint,
  value,
  onChange,
}: {
  roleKey: RoleKey;
  label: string;
  hint: string;
  value: string;
  onChange: (hex: string) => void;
}) {
  const inputId = `palette-${roleKey}`;
  const commit = (raw: string) => {
    const parsed = parseHex(raw);
    if (isHex(raw) && parsed) onChange(toHex(parsed)); // normalize to #rrggbb
  };
  return (
    <div className="flex items-center gap-3 rounded-lg border border-border bg-surface p-2.5">
      <input
        id={inputId}
        type="color"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        aria-label={`${label} color`}
        className="h-9 w-9 shrink-0 cursor-pointer rounded-md border border-border bg-transparent"
      />
      <div className="min-w-0 flex-1">
        <label htmlFor={inputId} className="block text-sm font-medium text-fg">
          {label}
        </label>
        <span className="text-xs text-fg-subtle">{hint}</span>
      </div>
      <input
        type="text"
        value={value}
        spellCheck={false}
        onChange={(e) => commit(e.target.value)}
        aria-label={`${label} hex value`}
        className="w-24 rounded-md border border-border bg-surface-2 px-2 py-1 text-right font-mono text-xs uppercase text-fg focus:border-accent focus:outline-none"
      />
    </div>
  );
}
