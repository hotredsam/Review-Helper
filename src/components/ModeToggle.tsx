import { Code2, GraduationCap } from "lucide-react";
import { useUiStore, type AppMode } from "../store/uiStore";

const MODES: { id: AppMode; label: string; icon: typeof Code2 }[] = [
  { id: "code", label: "Code", icon: Code2 },
  { id: "learning", label: "Learn", icon: GraduationCap },
];

/** Top-level shell switch: Code-review ↔ Learning. Persisted in uiStore; the
 *  whole MainPane + sidebar nav branch on it. Collapses to icon-only. */
export function ModeToggle({ collapsed }: { collapsed: boolean }) {
  const mode = useUiStore((s) => s.appMode);
  const setMode = useUiStore((s) => s.setAppMode);
  return (
    <div
      role="radiogroup"
      aria-label="App mode"
      className={"flex rounded-lg border border-border bg-surface p-0.5 " + (collapsed ? "flex-col gap-0.5" : "")}
    >
      {MODES.map((m) => {
        const active = m.id === mode;
        const Icon = m.icon;
        return (
          <button
            key={m.id}
            type="button"
            role="radio"
            aria-checked={active}
            aria-label={m.label}
            title={m.label}
            onClick={() => setMode(m.id)}
            className={
              "flex flex-1 items-center justify-center gap-1.5 rounded-md px-2 py-1 text-xs font-medium transition-colors " +
              (active ? "bg-accent text-accent-fg" : "text-fg-muted hover:bg-surface-2 hover:text-fg")
            }
          >
            <Icon className="h-3.5 w-3.5 shrink-0" />
            {!collapsed && m.label}
          </button>
        );
      })}
    </div>
  );
}
