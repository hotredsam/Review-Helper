import { useUiStore } from "../store/uiStore";

const MODES: { id: "easy" | "technical"; label: string }[] = [
  { id: "easy", label: "Easy" },
  { id: "technical", label: "Technical" },
];

/** Global toggle for how explanatory text reads — plain ("Easy") vs precise
 *  ("Technical"). Drives the assessment dimension reasons, top fixes, hygiene,
 *  and the plan's simple/full text. Persisted in uiStore. */
export function TextModeToggle() {
  const mode = useUiStore((s) => s.textMode);
  const setMode = useUiStore((s) => s.setTextMode);
  return (
    <div
      role="radiogroup"
      aria-label="Explanation detail"
      className="inline-flex shrink-0 rounded-lg border border-border bg-surface p-0.5 text-xs"
    >
      {MODES.map((m) => {
        const active = m.id === mode;
        return (
          <button
            key={m.id}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => setMode(m.id)}
            className={
              "rounded-md px-2 py-0.5 font-medium transition-colors " +
              (active ? "bg-accent text-accent-fg" : "text-fg-muted hover:bg-surface-2 hover:text-fg")
            }
          >
            {m.label}
          </button>
        );
      })}
    </div>
  );
}
