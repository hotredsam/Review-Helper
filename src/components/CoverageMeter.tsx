import { CheckCircle2 } from "lucide-react";
import { InfoDot } from "./InfoDot";
import type { Coverage } from "../store/grillStore";

/** Detail Coverage meter: overall progress + per-dimension breakdown, flipping
 *  to "Done grilling" at saturation (no open questions remain). */
export function CoverageMeter({ cov }: { cov: Coverage }) {
  const pct = cov.total ? Math.round((cov.addressed / cov.total) * 100) : 0;
  return (
    <div className="rounded-lg border border-border bg-surface p-4">
      <div className="mb-2 flex items-center justify-between">
        <h3 className="flex items-center text-xs font-semibold uppercase tracking-wide text-fg-subtle">
          Detail coverage
          <InfoDot
            term="detail coverage"
            explanation="How much of the project you've pinned down — answered and dismissed questions both count. It flips to “Done grilling” when none are left open."
          />
        </h3>
        {cov.done ? (
          <span className="flex items-center gap-1 rounded-full bg-success/15 px-2 py-0.5 text-xs font-medium text-success">
            <CheckCircle2 className="h-3 w-3" /> Done grilling
          </span>
        ) : (
          <span className="text-xs text-fg-subtle">
            {cov.addressed}/{cov.total} addressed
          </span>
        )}
      </div>
      <div
        className="h-2 w-full overflow-hidden rounded-full bg-surface-2"
        role="progressbar"
        aria-label={`Detail coverage: ${cov.addressed} of ${cov.total} addressed`}
        aria-valuenow={pct}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div
          className={"h-full rounded-full transition-all " + (cov.done ? "bg-success" : "bg-accent")}
          style={{ width: `${pct}%` }}
        />
      </div>
      {cov.byDimension.length > 0 && (
        <div className="mt-3 flex flex-wrap gap-1.5">
          {cov.byDimension.map((d) => (
            <span
              key={d.dimension}
              className={
                "rounded-full px-2 py-0.5 text-xs capitalize " +
                (d.addressed >= d.total
                  ? "bg-success/15 text-success"
                  : "bg-surface-2 text-fg-subtle")
              }
            >
              {d.dimension} {d.addressed}/{d.total}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
