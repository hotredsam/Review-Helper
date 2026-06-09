import { useEffect, useState } from "react";
import { Loader2, TrendingUp } from "lucide-react";
import { type ProfileSnapshot, learningProgress } from "../../api/learning";

/** Honest, evidence-based read on how the learner is doing — accuracy, reviews,
 *  and per-skill mastery (Bayesian estimate). The "how you're learning" summary
 *  is derived ONLY from real signals; it never claims a "learning style". */
export function ProgressPane({ subjectId }: { subjectId: number }) {
  const [p, setP] = useState<ProfileSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    setP(null);
    setError(null);
    learningProgress(subjectId)
      .then((d) => live && setP(d))
      .catch((e) => live && setError(String(e)));
    return () => {
      live = false;
    };
  }, [subjectId]);

  if (error) {
    return (
      <p className="text-sm text-danger" role="alert">
        {error}
      </p>
    );
  }
  if (!p) {
    return (
      <p className="flex items-center gap-2 text-sm text-fg-subtle">
        <Loader2 className="h-4 w-4 animate-spin" /> Loading your progress…
      </p>
    );
  }

  const noData = p.attempts === 0 && p.flashcard_reviews === 0;

  return (
    <div className="space-y-5">
      <div className="rounded-xl border border-border bg-surface p-5">
        <div className="mb-2 flex items-center gap-2">
          <TrendingUp className="h-4 w-4 text-accent" />
          <h2 className="text-sm font-semibold text-fg">How it's going</h2>
        </div>
        <p className="text-sm text-fg-muted">{summarize(p, noData)}</p>
      </div>

      {!noData && (
        <div className="grid grid-cols-3 gap-3">
          <Stat label="Quiz accuracy" value={p.attempts ? `${Math.round(p.accuracy * 100)}%` : "—"} sub={`${p.attempts} answered`} />
          <Stat label="Cards reviewed" value={String(p.flashcard_reviews)} sub="spaced repetition" />
          <Stat label="Avg. answer time" value={p.attempts ? `${(p.avg_latency_ms / 1000).toFixed(1)}s` : "—"} sub="per question" />
        </div>
      )}

      {p.skills.length > 0 && (
        <div className="rounded-xl border border-border bg-surface p-5">
          <h2 className="mb-3 text-xs font-semibold uppercase tracking-wide text-fg-subtle">Mastery by skill</h2>
          <ul className="space-y-3">
            {p.skills.map((s) => {
              const pct = Math.round(s.p_known * 100);
              return (
                <li key={s.skill}>
                  <div className="mb-1 flex items-center justify-between text-sm">
                    <span className="text-fg">{s.skill}</span>
                    <span className="text-fg-subtle">
                      {pct}% <span className="text-fg-subtle">· n={s.n_obs}</span>
                    </span>
                  </div>
                  <div className="h-2 w-full overflow-hidden rounded-full bg-surface-2">
                    <div className="h-full rounded-full bg-accent transition-all" style={{ width: `${pct}%` }} />
                  </div>
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </div>
  );
}

function Stat({ label, value, sub }: { label: string; value: string; sub: string }) {
  return (
    <div className="rounded-xl border border-border bg-surface p-4">
      <p className="text-xs text-fg-subtle">{label}</p>
      <p className="mt-1 text-2xl font-semibold text-fg">{value}</p>
      <p className="text-xs text-fg-subtle">{sub}</p>
    </div>
  );
}

/** Compose a friendly summary from real numbers only — no "learning style". */
function summarize(p: ProfileSnapshot, noData: boolean): string {
  if (noData) {
    return "Study a little — answer a quiz or review some flashcards — and a read on how you're doing (and where to focus) shows up here.";
  }
  const parts: string[] = [];
  if (p.attempts > 0) parts.push(`You've answered ${p.attempts} question${p.attempts === 1 ? "" : "s"} (${Math.round(p.accuracy * 100)}% correct)`);
  if (p.flashcard_reviews > 0) parts.push(`reviewed ${p.flashcard_reviews} flashcard${p.flashcard_reviews === 1 ? "" : "s"}`);
  let s = parts.join(", ") + ".";

  if (p.skills.length > 0) {
    const sorted = [...p.skills].sort((a, b) => b.p_known - a.p_known);
    const top = sorted[0];
    const low = sorted[sorted.length - 1];
    s += ` Strongest so far: ${top.skill} (${Math.round(top.p_known * 100)}%).`;
    if (low.skill !== top.skill && low.p_known < 0.6) {
      s += ` Worth more practice: ${low.skill} (${Math.round(low.p_known * 100)}%) — the plan will keep bringing it back.`;
    }
  }
  return s;
}
