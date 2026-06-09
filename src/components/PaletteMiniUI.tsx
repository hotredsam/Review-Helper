import { mixHex, readableTextHex } from "../palette/color";
import type { Palette } from "../palette/generator";

/**
 * A simplistic phone-screen mock rendered in the chosen palette — header, a card
 * with text rows, a primary button, and an accent chip — so you can see how the
 * colors distribute in an actual UI (not just an abstract icon). Uses the raw
 * palette hexes on purpose (it previews the user's palette, not the app theme).
 */
export function PaletteMiniUI({ palette }: { palette: Palette }) {
  const muted = mixHex(palette.text, palette.bg, 0.42);
  const border = mixHex(palette.surface, palette.text, 0.16);
  const onPrimary = readableTextHex(palette.primary);
  const onAccent = readableTextHex(palette.accent);

  return (
    <div
      aria-label="Palette UI preview"
      role="img"
      className="w-[180px] shrink-0 overflow-hidden rounded-[26px] border shadow-lg"
      style={{ background: palette.bg, borderColor: border }}
    >
      {/* status + header */}
      <div className="px-4 pt-3" style={{ background: palette.bg }}>
        <div className="mb-2 flex items-center justify-between text-[9px]" style={{ color: muted }}>
          <span>9:41</span>
          <span>●●●</span>
        </div>
        <div className="flex items-center justify-between">
          <span className="text-[13px] font-semibold" style={{ color: palette.text }}>
            Dashboard
          </span>
          <span
            className="rounded-full px-2 py-0.5 text-[8px] font-medium"
            style={{ background: palette.accent, color: onAccent }}
          >
            New
          </span>
        </div>
      </div>

      <div className="space-y-2.5 p-4">
        {/* a card */}
        <div className="rounded-xl p-3" style={{ background: palette.surface, border: `1px solid ${border}` }}>
          <div className="mb-2 h-2 w-1/2 rounded-full" style={{ background: palette.primary }} />
          <div className="mb-1.5 h-1.5 w-full rounded-full" style={{ background: muted, opacity: 0.5 }} />
          <div className="h-1.5 w-3/4 rounded-full" style={{ background: muted, opacity: 0.5 }} />
        </div>

        {/* a stat row */}
        <div className="flex gap-2">
          <div className="flex-1 rounded-xl p-2 text-center" style={{ background: palette.surface, border: `1px solid ${border}` }}>
            <div className="text-[13px] font-bold" style={{ color: palette.primary }}>
              24
            </div>
            <div className="text-[7px]" style={{ color: muted }}>
              done
            </div>
          </div>
          <div className="flex-1 rounded-xl p-2 text-center" style={{ background: palette.surface, border: `1px solid ${border}` }}>
            <div className="text-[13px] font-bold" style={{ color: palette.accent }}>
              7
            </div>
            <div className="text-[7px]" style={{ color: muted }}>
              left
            </div>
          </div>
        </div>

        {/* primary action */}
        <div
          className="rounded-xl py-2 text-center text-[11px] font-semibold"
          style={{ background: palette.primary, color: onPrimary }}
        >
          Continue
        </div>
      </div>
    </div>
  );
}
