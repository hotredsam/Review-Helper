import { GraduationCap } from "lucide-react";
import { EmptyState } from "./EmptyState";

/** A clearly-stubbed placeholder for the future non-vibecoding learning mode.
 *  No backend — this is intentionally not built yet. */
export function ComingSoon() {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3">
      <EmptyState
        icon={GraduationCap}
        title="Learning mode — coming soon"
        body="A future mode for structured study beyond vibecoding (e.g. CPA exam prep). It isn't built yet — this is a placeholder for what's next."
      />
      <span className="rounded-full bg-surface-2 px-3 py-1 text-xs font-medium text-fg-subtle">
        Coming soon
      </span>
    </div>
  );
}
