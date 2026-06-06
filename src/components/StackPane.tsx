import { useEffect } from "react";
import { useStackStore } from "../store/stackStore";
import { WhyExplain } from "./WhyExplain";
import type { Project } from "../api/projects";

const PANE_LABEL: Record<string, string> = {
  frontend: "Frontend",
  backend: "Backend",
  database: "Database",
  deployment: "Deployment",
  pipes: "Pipes",
};

/**
 * The Stack pane: five build panes, each with a recommendation + alternatives +
 * rationale + Why? (card tap-through). Pre-made stacks apply to all five; any
 * pane can be overridden. Every selection is recorded as a decision (backend).
 */
export function StackPane({ project }: { project: Project }) {
  const id = project.id;
  const catalog = useStackStore((s) => s.catalog);
  const premade = useStackStore((s) => s.premade);
  const selectionsRaw = useStackStore((s) => s.selections[id]);
  const selections = selectionsRaw ?? [];
  const error = useStackStore((s) => s.error[id]);
  const load = useStackStore((s) => s.load);
  const set = useStackStore((s) => s.set);
  const applyPremade = useStackStore((s) => s.applyPremade);

  useEffect(() => {
    void load(id);
  }, [id, load]);

  return (
    <div className="mx-auto max-w-3xl space-y-6 p-8">
      <section className="space-y-2">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Pre-made stacks</h2>
        <p className="text-sm text-fg-muted">Apply a full stack to all five panes, then override any pane.</p>
        <div className="flex flex-wrap gap-2">
          {premade.map((p) => (
            <button
              key={p.name}
              type="button"
              onClick={() => void applyPremade(id, p.name)}
              title={p.summary}
              className="rounded-lg border border-border bg-surface px-3 py-1.5 text-sm text-fg hover:border-accent hover:bg-surface-2"
            >
              {p.name}
            </button>
          ))}
        </div>
      </section>

      {error && (
        <p className="text-sm text-danger" role="alert">
          {error}
        </p>
      )}

      <section className="space-y-3">
        {selections.map((sel) => {
          const options = catalog[sel.pane] ?? [];
          return (
            <div key={sel.pane} className="rounded-lg border border-border bg-surface p-4">
              <div className="mb-2 flex items-center justify-between gap-2">
                <h3 className="font-semibold text-fg">{PANE_LABEL[sel.pane] ?? sel.pane}</h3>
                <div className="flex items-center">
                  {sel.choice ? (
                    <span className="text-sm text-fg">{sel.choice}</span>
                  ) : (
                    <span className="text-sm text-fg-subtle">Not chosen</span>
                  )}
                  {sel.choice && <WhyExplain term={sel.choice} />}
                </div>
              </div>
              <div className="flex flex-wrap gap-2">
                {options.map((o, i) => {
                  const selected = sel.choice === o.choice;
                  return (
                    <button
                      key={o.choice}
                      type="button"
                      onClick={() => void set(id, sel.pane, o.choice)}
                      aria-pressed={selected}
                      className={
                        "rounded-full border px-3 py-1 text-sm transition-colors " +
                        (selected
                          ? "border-accent bg-accent/10 text-fg"
                          : "border-border text-fg-muted hover:bg-surface-2 hover:text-fg")
                      }
                    >
                      {o.choice}
                      {i === 0 && <span className="ml-1 text-xs text-fg-subtle">· recommended</span>}
                    </button>
                  );
                })}
              </div>
              {sel.rationale && <p className="mt-2 text-sm text-fg-muted">{sel.rationale}</p>}
            </div>
          );
        })}
        {selections.length === 0 && <p className="text-sm text-fg-subtle">Loading panes…</p>}
      </section>
    </div>
  );
}
