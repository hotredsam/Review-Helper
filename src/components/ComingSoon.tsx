import { GraduationCap } from "lucide-react";

/** A clearly-stubbed placeholder for the future non-vibecoding learning mode.
 *  No backend — this is intentionally not built yet. Everything is centered as
 *  one group so the "coming soon" badge sits with the content, not at the very
 *  bottom of the pane. */
export function ComingSoon() {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-10 text-center">
      <div className="flex h-14 w-14 items-center justify-center rounded-2xl border border-border bg-surface text-fg-subtle">
        <GraduationCap className="h-7 w-7" strokeWidth={1.75} />
      </div>
      <div className="max-w-sm space-y-1.5">
        <h2 className="text-lg font-semibold text-fg">Learning mode — coming soon</h2>
        <p className="text-sm text-fg-muted">
          A future mode for structured study beyond vibecoding (e.g. CPA exam prep). It isn't built
          yet — this is a placeholder for what's next.
        </p>
      </div>
      <span className="rounded-full bg-surface-2 px-2 py-0.5 text-xs font-medium text-fg-muted">
        Coming soon
      </span>
    </div>
  );
}
