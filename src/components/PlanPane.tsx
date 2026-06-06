import { useEffect, useState, type ReactNode } from "react";
import { Loader2, Play, Sparkles, AlertTriangle } from "lucide-react";
import { usePlanStore, ensureAnalysisListener } from "../store/planStore";
import { WhyExplain } from "./WhyExplain";
import type { Project } from "../api/projects";

/**
 * The Plan pane: triggers read-only analysis of the clone, shows a loading
 * indicator while it runs, and renders the resulting phased plan. The plan is
 * shown as the plan (no "draft" label).
 */
export function PlanPane({ project }: { project: Project }) {
  const plan = usePlanStore((s) => s.plans[project.id]);
  const analysis = usePlanStore((s) => s.analysis[project.id] ?? "idle");
  // Raw value (stable ref); default outside the selector to avoid an infinite
  // re-render loop from a fresh [] each render.
  const progressRaw = usePlanStore((s) => s.progress[project.id]);
  const error = usePlanStore((s) => s.error[project.id]);
  const loadPlan = usePlanStore((s) => s.loadPlan);
  const analyze = usePlanStore((s) => s.analyze);
  const kickoff = usePlanStore((s) => s.kickoff);
  const update = usePlanStore((s) => s.update);
  const rebuild = usePlanStore((s) => s.rebuild);
  const loadAudit = usePlanStore((s) => s.loadAudit);
  const auditRaw = usePlanStore((s) => s.audit[project.id]);
  const audit = auditRaw ?? [];
  const [desc, setDesc] = useState("");

  useEffect(() => {
    ensureAnalysisListener();
  }, []);
  useEffect(() => {
    if (plan === undefined) void loadPlan(project.id);
  }, [project.id, plan, loadPlan]);
  useEffect(() => {
    void loadAudit(project.id);
  }, [project.id, loadAudit]);

  const onRebuild = () => {
    if (
      window.confirm(
        "Rebuild regenerates the plan from scratch and does NOT carry over your phase completion. Use “Update plan” to weave in changes while keeping progress. Rebuild anyway?",
      )
    ) {
      void rebuild(project.id);
    }
  };

  const cloned = !!project.clone_path;
  const linked = !!project.github_repo_url;

  if (analysis === "running") {
    const recent = (progressRaw ?? []).slice(-4).join(" · ");
    return (
      <Center>
        <Loader2 className="h-7 w-7 animate-spin text-accent" />
        <p className="text-sm font-medium text-fg">Analyzing the repository…</p>
        <p className="max-w-sm text-xs text-fg-subtle">
          Reading files read-only and drafting a plan. This can take a minute.
        </p>
        {recent && <p className="text-xs text-fg-subtle">{recent}</p>}
      </Center>
    );
  }

  if (analysis === "error") {
    return (
      <Center>
        <AlertTriangle className="h-7 w-7 text-danger" />
        <p className="max-w-sm text-sm text-danger" role="alert">
          {error ?? "Analysis failed."}
        </p>
        {cloned && <AnalyzeButton onClick={() => void analyze(project.id)} label="Try again" />}
      </Center>
    );
  }

  if (plan === undefined) {
    return (
      <Center>
        <Loader2 className="h-6 w-6 animate-spin text-fg-subtle" />
      </Center>
    );
  }

  if (!plan) {
    if (linked && cloned) {
      return (
        <Center>
          <Sparkles className="h-8 w-8 text-fg-subtle" />
          <p className="text-sm font-medium text-fg">Ready to analyze</p>
          <p className="max-w-sm text-sm text-fg-muted">
            Review Helper reads your repo (read-only) and drafts a first phased plan, absorbing any
            existing planning docs.
          </p>
          <AnalyzeButton onClick={() => void analyze(project.id)} label="Analyze repository" />
        </Center>
      );
    }
    if (linked) {
      return (
        <Center>
          <p className="max-w-sm text-sm text-fg-muted">
            Clone the repo first (use Refresh in the header), then analyze.
          </p>
        </Center>
      );
    }
    return (
      <div className="mx-auto max-w-xl space-y-3 p-8">
        <div>
          <h2 className="text-sm font-semibold text-fg">What are you building?</h2>
          <p className="text-sm text-fg-muted">
            Describe your project in a few sentences. Review Helper drafts an honest starting plan
            from your description.
          </p>
        </div>
        <textarea
          value={desc}
          onChange={(e) => setDesc(e.target.value)}
          rows={5}
          placeholder="e.g. A macOS menu-bar app that tracks how long I spend in each app and shows a weekly chart…"
          className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none focus:ring-2 focus:ring-ring/40"
        />
        <button
          type="button"
          disabled={!desc.trim()}
          onClick={() => void kickoff(project.id, desc)}
          className="flex items-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
        >
          <Sparkles className="h-4 w-4" /> Generate starting plan
        </button>
        {error && (
          <p className="text-sm text-danger" role="alert">
            {error}
          </p>
        )}
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-3xl space-y-6 p-8">
      {plan.current_state && (
        <section>
          <h2 className="mb-1 text-xs font-semibold uppercase tracking-wide text-fg-subtle">
            Current state
          </h2>
          <p className="whitespace-pre-wrap text-sm text-fg-muted">{plan.current_state}</p>
        </section>
      )}

      <section className="space-y-3">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Phases</h2>
        {plan.phases.length === 0 && (
          <p className="text-sm text-fg-subtle">
            No phases yet — the repo didn't support more than this honestly.
          </p>
        )}
        {plan.phases.map((ph, i) => (
          <div key={ph.id} className="rounded-lg border border-border bg-surface p-4">
            <div className="mb-2 flex items-baseline gap-2">
              <span className="text-xs font-semibold text-fg-subtle">{i + 1}</span>
              <h3 className="font-semibold text-fg">{ph.title}</h3>
            </div>
            {ph.goal && <p className="mb-3 text-sm text-fg-muted">{ph.goal}</p>}
            <ul className="space-y-2">
              {ph.tasks.map((t) => (
                <li key={t.id} className="rounded-md bg-surface-2 px-3 py-2">
                  <p className="text-sm font-medium text-fg">{t.title}</p>
                  {t.body_md && <p className="mt-0.5 text-xs text-fg-muted">{t.body_md}</p>}
                  {t.verification && (
                    <p className="mt-1 text-xs text-fg-subtle">
                      <span className="font-medium">Done when:</span> {t.verification}
                    </p>
                  )}
                </li>
              ))}
            </ul>
          </div>
        ))}
      </section>

      {plan.decisions.length > 0 && (
        <section className="space-y-1.5">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Decisions</h2>
          {plan.decisions.map((d, i) => (
            <div key={i} className="text-sm">
              <span className="font-medium text-fg">{d.topic}:</span>{" "}
              <span className="text-fg-muted">{d.choice}</span>
              {d.rationale && <span className="text-fg-subtle"> — {d.rationale}</span>}
              <WhyExplain term={d.choice || d.topic} />
            </div>
          ))}
        </section>
      )}

      {plan.stack.some((s) => s.choice) && (
        <section className="space-y-1">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Stack</h2>
          {plan.stack.map(
            (s) =>
              s.choice && (
                <div key={s.pane} className="text-sm">
                  <span className="capitalize text-fg-subtle">{s.pane}:</span>{" "}
                  <span className="text-fg">{s.choice}</span>
                  <WhyExplain term={s.choice} />
                </div>
              ),
          )}
        </section>
      )}

      <div className="flex gap-2 pt-2">
        <button
          type="button"
          onClick={() => void update(project.id)}
          className="rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-accent-fg hover:bg-accent-hover"
        >
          Update plan
        </button>
        {cloned && (
          <button
            type="button"
            onClick={() => void analyze(project.id)}
            className="rounded-md border border-border px-3 py-1.5 text-xs text-fg-muted hover:bg-surface-2"
          >
            Re-analyze
          </button>
        )}
        <button
          type="button"
          onClick={onRebuild}
          className="rounded-md border border-danger/40 px-3 py-1.5 text-xs text-danger hover:bg-danger/10"
        >
          Rebuild plan
        </button>
      </div>

      {audit.length > 0 && (
        <section className="space-y-1 pt-2">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">History</h2>
          <ul className="space-y-0.5">
            {audit
              .slice()
              .reverse()
              .map((e, i) => (
                <li key={i} className="text-xs text-fg-subtle">
                  <span className="font-medium text-fg-muted">v{e.version}</span> ← {e.source}
                  <span className="ml-2">{e.at}</span>
                </li>
              ))}
          </ul>
        </section>
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

function AnalyzeButton({ onClick, label }: { onClick: () => void; label: string }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="mt-1 flex items-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover"
    >
      <Play className="h-4 w-4" /> {label}
    </button>
  );
}
