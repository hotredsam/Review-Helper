import { Check } from "lucide-react";
import type { UiSpec } from "../api/grill";

const TEXT_CLS =
  "mt-2 w-full rounded-md border border-border bg-surface-2 px-2 py-1.5 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none";
const PILL = "rounded-full border px-3 py-1 text-sm transition-colors ";
const PILL_ON = "border-accent bg-accent/10 text-fg";
const PILL_OFF = "border-border text-fg-muted hover:bg-surface-2 hover:text-fg";

/** Renders the right input widget for a grill question from its model-emitted
 *  `ui_spec` — choice pills, a scale slider, or text — always reporting the
 *  answer back as a plain string. Falls back to a textarea (the original
 *  behavior) for `long_text` or any missing/unknown spec. */
export function QuestionInput({
  spec,
  value,
  onChange,
  ariaLabel,
}: {
  spec?: UiSpec | null;
  value: string;
  onChange: (v: string) => void;
  ariaLabel: string;
}) {
  const field = spec?.field ?? "long_text";
  const options = spec?.options ?? [];

  if (field === "single_choice" && options.length > 0) {
    return (
      <div role="radiogroup" aria-label={ariaLabel} className="mt-2 flex flex-wrap gap-2">
        {options.map((o) => {
          const active = value === o;
          return (
            <button
              key={o}
              type="button"
              role="radio"
              aria-checked={active}
              onClick={() => onChange(o)}
              className={PILL + (active ? PILL_ON : PILL_OFF)}
            >
              {o}
            </button>
          );
        })}
      </div>
    );
  }

  if (field === "multi_choice" && options.length > 0) {
    const selected = new Set(value ? value.split(/,\s*/).filter(Boolean) : []);
    const toggle = (o: string) => {
      const next = new Set(selected);
      if (next.has(o)) next.delete(o);
      else next.add(o);
      onChange(Array.from(next).join(", "));
    };
    return (
      <div aria-label={ariaLabel} className="mt-2 flex flex-wrap gap-2">
        {options.map((o) => {
          const active = selected.has(o);
          return (
            <button
              key={o}
              type="button"
              aria-pressed={active}
              onClick={() => toggle(o)}
              className={"flex items-center gap-1 " + PILL + (active ? PILL_ON : PILL_OFF)}
            >
              {active && <Check className="h-3 w-3" />} {o}
            </button>
          );
        })}
      </div>
    );
  }

  if (field === "scale") {
    const min = spec?.min ?? 1;
    const max = spec?.max ?? 5;
    const cur = Number(value) || min;
    return (
      <div className="mt-2">
        <input
          type="range"
          min={min}
          max={max}
          value={cur}
          aria-label={ariaLabel}
          onChange={(e) => onChange(e.target.value)}
          className="w-full accent-accent"
        />
        <div className="flex justify-between text-xs text-fg-subtle">
          <span>{spec?.min_label ?? min}</span>
          <span className="font-medium text-fg">{cur}</span>
          <span>{spec?.max_label ?? max}</span>
        </div>
      </div>
    );
  }

  if (field === "short_text") {
    return (
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        aria-label={ariaLabel}
        maxLength={10000}
        placeholder="Your answer…"
        className={TEXT_CLS}
      />
    );
  }

  // long_text (default / fallback)
  return (
    <textarea
      value={value}
      onChange={(e) => onChange(e.target.value)}
      rows={2}
      maxLength={10000}
      aria-label={ariaLabel}
      placeholder="Your answer…"
      className={TEXT_CLS}
    />
  );
}
