import { useEffect, useState } from "react";
import { FileText, Layers, ListChecks, Loader2, Sparkles } from "lucide-react";
import {
  type ModuleKind,
  type ProposedModule,
  learningModules,
  learningModuleSetIncluded,
  learningConfirmPlan,
} from "../../api/learning";

const KIND_META: Record<ModuleKind, { label: string; icon: typeof FileText }> = {
  notes: { label: "Notes", icon: FileText },
  flashcards: { label: "Flashcards", icon: Layers },
  quiz: { label: "Quiz", icon: ListChecks },
  tutor: { label: "Tutor", icon: Sparkles },
};

/**
 * L2 — the editable study plan. The model proposed these modules from the
 * scoping answers; the learner toggles which to keep, then starts studying.
 * Toggling persists immediately; "Start studying" locks the plan in.
 */
export function ModuleProposalPane({
  subjectId,
  onConfirmed,
}: {
  subjectId: number;
  onConfirmed: () => void;
}) {
  const [modules, setModules] = useState<ProposedModule[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    learningModules(subjectId)
      .then((m) => live && setModules(m))
      .catch((e) => live && setError(String(e)));
    return () => {
      live = false;
    };
  }, [subjectId]);

  const toggle = async (m: ProposedModule) => {
    const next = !m.included;
    setModules((prev) => prev?.map((x) => (x.id === m.id ? { ...x, included: next } : x)) ?? prev);
    try {
      await learningModuleSetIncluded(m.id, next);
    } catch (e) {
      setError(String(e));
      // Revert on failure so the UI matches the record.
      setModules((prev) => prev?.map((x) => (x.id === m.id ? { ...x, included: m.included } : x)) ?? prev);
    }
  };

  const includedCount = modules?.filter((m) => m.included).length ?? 0;

  const start = async () => {
    setBusy(true);
    setError(null);
    try {
      await learningConfirmPlan(subjectId);
      onConfirmed();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  if (error && !modules) {
    return (
      <p className="text-sm text-danger" role="alert">
        {error}
      </p>
    );
  }
  if (!modules) {
    return <p className="text-sm text-fg-subtle">Loading the proposed plan…</p>;
  }

  return (
    <div className="space-y-4">
      <p className="text-sm text-fg-muted">
        Here's a plan tailored to your answers. Keep what's useful, drop the rest — you can always add more later.
      </p>

      {error && (
        <p className="text-sm text-danger" role="alert">
          {error}
        </p>
      )}

      <ul className="space-y-2">
        {modules.map((m) => {
          const meta = KIND_META[m.kind] ?? KIND_META.notes;
          const Icon = meta.icon;
          return (
            <li key={m.id}>
              <label
                className={
                  "flex cursor-pointer items-start gap-3 rounded-xl border p-4 transition-colors " +
                  (m.included ? "border-accent bg-accent/5" : "border-border bg-surface opacity-70 hover:opacity-100")
                }
              >
                <input
                  type="checkbox"
                  checked={m.included}
                  onChange={() => void toggle(m)}
                  className="mt-0.5 h-4 w-4 accent-[var(--accent)]"
                />
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <Icon className="h-4 w-4 shrink-0 text-accent" />
                    <span className="font-medium text-fg">{m.title}</span>
                    <span className="rounded-full bg-surface-2 px-2 py-0.5 text-xs text-fg-muted">{meta.label}</span>
                  </div>
                  {m.summary && <p className="mt-1 text-sm text-fg-muted">{m.summary}</p>}
                </div>
              </label>
            </li>
          );
        })}
      </ul>

      <div className="flex items-center justify-between gap-3 border-t border-border pt-4">
        <span className="text-xs text-fg-subtle">
          {includedCount} module{includedCount === 1 ? "" : "s"} selected
        </span>
        <button
          onClick={() => void start()}
          disabled={busy || includedCount === 0}
          className="flex items-center gap-1.5 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
        >
          {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <Sparkles className="h-4 w-4" />}
          Start studying
        </button>
      </div>
    </div>
  );
}
