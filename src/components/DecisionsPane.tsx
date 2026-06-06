import { useEffect } from "react";
import { CheckCheck, Check, X, Inbox } from "lucide-react";
import { useDecisionsStore } from "../store/decisionsStore";
import { summarizeSuggestion } from "../api/suggestions";
import type { Project } from "../api/projects";

/**
 * The Decisions pane. T1: the pending-suggestions approval surface (Approve,
 * Dismiss, Approve all). T2 appends the ADR-style decisions record below.
 */
export function DecisionsPane({ project }: { project: Project }) {
  const id = project.id;
  const pendingRaw = useDecisionsStore((s) => s.pending[id]);
  const pending = pendingRaw ?? [];
  const error = useDecisionsStore((s) => s.error[id]);
  const loadPending = useDecisionsStore((s) => s.loadPending);
  const approve = useDecisionsStore((s) => s.approve);
  const dismiss = useDecisionsStore((s) => s.dismiss);
  const approveAll = useDecisionsStore((s) => s.approveAll);

  useEffect(() => {
    void loadPending(id);
  }, [id, loadPending]);

  return (
    <div className="mx-auto max-w-3xl space-y-6 p-8">
      <section className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">
            Pending suggestions {pending.length > 0 && `(${pending.length})`}
          </h2>
          {pending.length > 0 && (
            <button
              type="button"
              onClick={() => void approveAll(id)}
              className="flex items-center gap-1.5 rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-accent-fg hover:bg-accent-hover"
            >
              <CheckCheck className="h-3.5 w-3.5" /> Approve all
            </button>
          )}
        </div>

        {error && (
          <p className="text-sm text-danger" role="alert">
            {error}
          </p>
        )}

        {pending.length === 0 ? (
          <div className="flex flex-col items-center gap-2 rounded-lg border border-dashed border-border p-8 text-center">
            <Inbox className="h-6 w-6 text-fg-subtle" />
            <p className="max-w-sm text-sm text-fg-muted">
              No pending suggestions. Chat about your project and any updates it infers show up here
              for you to approve — nothing changes the record on its own.
            </p>
          </div>
        ) : (
          <ul className="space-y-2">
            {pending.map((s) => (
              <li
                key={s.id}
                className="flex items-center justify-between gap-3 rounded-lg border border-border bg-surface p-3"
              >
                <div className="min-w-0">
                  <span className="mr-2 rounded-full bg-surface-2 px-2 py-0.5 text-xs capitalize text-fg-subtle">
                    {s.kind}
                  </span>
                  <span className="text-sm text-fg">{summarizeSuggestion(s)}</span>
                </div>
                <div className="flex shrink-0 gap-1.5">
                  <button
                    type="button"
                    onClick={() => void approve(id, s.id)}
                    aria-label={`Approve ${s.kind}`}
                    className="flex items-center gap-1 rounded-md bg-accent px-2.5 py-1 text-xs font-medium text-accent-fg hover:bg-accent-hover"
                  >
                    <Check className="h-3 w-3" /> Approve
                  </button>
                  <button
                    type="button"
                    onClick={() => void dismiss(id, s.id)}
                    aria-label={`Dismiss ${s.kind}`}
                    className="flex items-center gap-1 rounded-md border border-border px-2.5 py-1 text-xs text-fg-muted hover:bg-surface-2"
                  >
                    <X className="h-3 w-3" /> Dismiss
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
