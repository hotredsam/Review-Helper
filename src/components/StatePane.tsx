import { useEffect, type ReactNode } from "react";
import { Loader2, Gauge, AlertTriangle } from "lucide-react";
import { useAssessStore, ensureAssessListener } from "../store/assessStore";
import type { DimScore } from "../api/assessment";
import type { Project } from "../api/projects";

const DIMENSIONS: [string, string][] = [
  ["architecture", "Architecture"],
  ["modularity", "Modularity"],
  ["context_hygiene", "Context hygiene"],
  ["security", "Security"],
  ["git_discipline", "Git discipline"],
  ["workflow", "Workflow"],
];

const PRODUCTION: [string, string][] = [
  ["tests", "Tests"],
  ["error_handling", "Error handling"],
  ["secrets", "Secrets"],
  ["build_ci", "Build + CI"],
  ["dependencies", "Dependencies"],
  ["docs", "Docs"],
];

// Static class strings (Tailwind must see them literally to generate the utility).
function tint(score: number): { text: string; bg: string } {
  if (score >= 75) return { text: "text-success", bg: "bg-success" };
  if (score >= 50) return { text: "text-warning", bg: "bg-warning" };
  return { text: "text-danger", bg: "bg-danger" };
}

/** The State pane (Overview): renders the assessment with numbers + color tint. */
export function StatePane({ project }: { project: Project }) {
  const assessment = useAssessStore((s) => s.assessments[project.id]);
  const status = useAssessStore((s) => s.status[project.id] ?? "idle");
  // Select the raw value (stable ref); default outside the selector to avoid an
  // infinite re-render loop from a fresh [] each render.
  const progressRaw = useAssessStore((s) => s.progress[project.id]);
  const error = useAssessStore((s) => s.error[project.id]);
  const load = useAssessStore((s) => s.load);
  const assess = useAssessStore((s) => s.assess);

  useEffect(() => {
    ensureAssessListener();
  }, []);
  useEffect(() => {
    if (assessment === undefined) void load(project.id);
  }, [project.id, assessment, load]);

  const cloned = !!project.clone_path;

  if (status === "running") {
    return (
      <Center>
        <Loader2 className="h-7 w-7 animate-spin text-accent" />
        <p className="text-sm font-medium text-fg">Assessing the project…</p>
        <p className="text-xs text-fg-subtle">{(progressRaw ?? []).slice(-3).join(" · ") || "Running the scan and scoring."}</p>
      </Center>
    );
  }
  if (status === "error") {
    return (
      <Center>
        <AlertTriangle className="h-7 w-7 text-danger" />
        <p className="max-w-sm text-sm text-danger" role="alert">{error ?? "Assessment failed."}</p>
        {cloned && <AssessButton onClick={() => void assess(project.id)} label="Try again" />}
      </Center>
    );
  }
  if (assessment === undefined) {
    return (
      <Center>
        <Loader2 className="h-6 w-6 animate-spin text-fg-subtle" />
      </Center>
    );
  }
  if (!assessment) {
    return (
      <Center>
        <Gauge className="h-8 w-8 text-fg-subtle" />
        <p className="text-sm font-medium text-fg">No assessment yet</p>
        <p className="max-w-sm text-sm text-fg-muted">
          Review Helper scores six vibecoding dimensions plus production readiness, grounded in a
          deterministic scan.
        </p>
        {cloned ? (
          <AssessButton onClick={() => void assess(project.id)} label="Assess project" />
        ) : (
          <p className="text-xs text-fg-subtle">Clone the repo first to assess it.</p>
        )}
      </Center>
    );
  }

  const overallTint = tint(assessment.overall);
  return (
    <div className="mx-auto max-w-3xl space-y-6 p-8">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Overall</p>
          <p className={"text-4xl font-bold " + overallTint.text}>{assessment.overall}</p>
        </div>
        <button
          type="button"
          onClick={() => void assess(project.id)}
          className="rounded-md border border-border px-3 py-1.5 text-xs text-fg-muted hover:bg-surface-2"
        >
          Re-assess
        </button>
      </div>

      <section className="space-y-3">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Vibecoding dimensions</h2>
        {DIMENSIONS.map(([key, label]) => (
          <ScoreRow key={key} label={label} dim={assessment.dimensions?.[key]} />
        ))}
      </section>

      <section className="space-y-2">
        <div className="flex items-baseline justify-between">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Production readiness</h2>
          <span className={"text-sm font-semibold " + tint(assessment.production?.overall ?? 0).text}>
            {assessment.production?.overall ?? 0}
          </span>
        </div>
        <div className="grid grid-cols-2 gap-x-6 gap-y-1.5">
          {PRODUCTION.map(([key, label]) => {
            const d = assessment.production?.scores?.[key];
            return (
              <div key={key} className="flex items-center justify-between text-sm">
                <span className="text-fg-muted">{label}</span>
                <span className={"font-medium " + tint(d?.score ?? 0).text}>{d?.score ?? "—"}</span>
              </div>
            );
          })}
        </div>
      </section>

      {assessment.top_fixes?.length > 0 && (
        <section className="space-y-1.5">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Top fixes</h2>
          <ol className="list-inside list-decimal space-y-1 text-sm text-fg">
            {assessment.top_fixes.map((f, i) => (
              <li key={i}>{f}</li>
            ))}
          </ol>
        </section>
      )}

      {assessment.hygiene?.length > 0 && (
        <section className="space-y-1.5">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Hygiene / cleanup</h2>
          <ul className="list-inside list-disc space-y-1 text-sm text-fg-muted">
            {assessment.hygiene.map((h, i) => (
              <li key={i}>{h}</li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

function ScoreRow({ label, dim }: { label: string; dim?: DimScore }) {
  const score = dim?.score ?? 0;
  const t = tint(score);
  const width = Math.max(0, Math.min(100, score));
  return (
    <div>
      <div className="flex items-baseline justify-between text-sm">
        <span className="text-fg">{label}</span>
        <span className={"font-semibold " + t.text}>{dim ? score : "—"}</span>
      </div>
      <div className="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-surface-2">
        <div className={"h-full rounded-full " + t.bg} style={{ width: `${width}%` }} />
      </div>
      {dim?.reason && <p className="mt-0.5 text-xs text-fg-subtle">{dim.reason}</p>}
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

function AssessButton({ onClick, label }: { onClick: () => void; label: string }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="mt-1 flex items-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover"
    >
      <Gauge className="h-4 w-4" /> {label}
    </button>
  );
}
