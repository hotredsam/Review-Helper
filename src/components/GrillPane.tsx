import { useEffect, type ReactNode } from "react";
import { Loader2, MessageSquareQuote, AlertTriangle, Sparkles } from "lucide-react";
import { useGrillStore, ensureGrillListener } from "../store/grillStore";
import { QuestionCard } from "./QuestionCard";
import type { Project } from "../api/projects";

const DEFAULT_DEPTH = 3;
const ACTIVE_AT_ONCE = 5;

function statusLabel(status: string): string {
  switch (status) {
    case "answered":
      return "Answered";
    case "not_relevant":
      return "Not relevant";
    case "unknown":
      return "Don't know";
    default:
      return status;
  }
}

/**
 * The Grill pane: generates repo-specific questions (each with a recommended
 * answer, tagged by dimension) and lists them. Card actions (T2) and the depth
 * slider + coverage meter (T3) build on this.
 */
export function GrillPane({ project }: { project: Project }) {
  const id = project.id;
  const questions = useGrillStore((s) => s.questions[id]);
  const status = useGrillStore((s) => s.status[id] ?? "idle");
  const progressRaw = useGrillStore((s) => s.progress[id]);
  const error = useGrillStore((s) => s.error[id]);
  const load = useGrillStore((s) => s.load);
  const generate = useGrillStore((s) => s.generate);

  useEffect(() => {
    ensureGrillListener();
  }, []);
  useEffect(() => {
    if (questions === undefined) void load(id);
  }, [id, questions, load]);

  if (status === "running") {
    const recent = (progressRaw ?? []).slice(-4).join(" · ");
    return (
      <Center>
        <Loader2 className="h-7 w-7 animate-spin text-accent" />
        <p className="text-sm font-medium text-fg">Writing repo-specific questions…</p>
        <p className="max-w-sm text-xs text-fg-subtle">
          Reading the repo read-only and drafting questions with recommended answers.
        </p>
        {recent && <p className="text-xs text-fg-subtle">{recent}</p>}
      </Center>
    );
  }

  if (status === "error") {
    return (
      <Center>
        <AlertTriangle className="h-7 w-7 text-danger" />
        <p className="max-w-sm text-sm text-danger" role="alert">
          {error ?? "Grilling failed."}
        </p>
        <GenerateButton onClick={() => void generate(id, DEFAULT_DEPTH)} label="Try again" />
      </Center>
    );
  }

  if (questions === undefined) {
    return (
      <Center>
        <Loader2 className="h-6 w-6 animate-spin text-fg-subtle" />
      </Center>
    );
  }

  if (questions.length === 0) {
    return (
      <Center>
        <MessageSquareQuote className="h-8 w-8 text-fg-subtle" />
        <p className="text-sm font-medium text-fg">Not grilled yet</p>
        <p className="max-w-sm text-sm text-fg-muted">
          Review Helper writes sharp, repo-specific questions — each with a recommended answer — to
          pin down what you're building.
        </p>
        <GenerateButton onClick={() => void generate(id, DEFAULT_DEPTH)} label="Start grilling" />
      </Center>
    );
  }

  const open = questions.filter((q) => q.status === "open");
  const active = open.slice(0, ACTIVE_AT_ONCE);
  const addressed = questions.filter((q) => q.status !== "open");

  return (
    <div className="mx-auto max-w-3xl space-y-4 p-8">
      <div className="flex items-center justify-between">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">
          {open.length} open · {addressed.length} addressed
        </h2>
        <button
          type="button"
          onClick={() => void generate(id, DEFAULT_DEPTH)}
          className="rounded-md border border-border px-3 py-1.5 text-xs text-fg-muted hover:bg-surface-2"
        >
          Ask more
        </button>
      </div>

      {active.length > 0 ? (
        <ul className="space-y-3">
          {active.map((q) => (
            <QuestionCard key={q.id} projectId={id} question={q} />
          ))}
        </ul>
      ) : (
        <p className="rounded-lg border border-border bg-surface p-4 text-sm text-fg-muted">
          All questions addressed. Use “Ask more” to go deeper.
        </p>
      )}

      {addressed.length > 0 && (
        <div className="space-y-1">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Addressed</h3>
          <ul className="space-y-1">
            {addressed.map((q) => (
              <li key={q.id} className="flex items-center gap-2 text-sm text-fg-subtle">
                <span className="shrink-0 rounded bg-surface-2 px-1.5 py-0.5 text-xs">
                  {statusLabel(q.status)}
                </span>
                <span className="truncate">{q.text}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

function Center({ children }: { children: ReactNode }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 p-10 text-center">
      {children}
    </div>
  );
}

function GenerateButton({ onClick, label }: { onClick: () => void; label: string }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="mt-1 flex items-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover"
    >
      <Sparkles className="h-4 w-4" /> {label}
    </button>
  );
}
