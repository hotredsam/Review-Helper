import { RefreshCw, Check } from "lucide-react";
import { useProjectStore } from "../store/projectStore";
import type { Project } from "../api/projects";

/** Clone-cache status + refresh for a GitHub-linked project (shown in the
 *  pane header). "cached" once the shallow clone exists; Refresh re-pulls. */
export function RepoCache({ project }: { project: Project }) {
  const explicit = useProjectStore((s) => s.cloneState[project.id]);
  const cloneError = useProjectStore((s) => s.cloneError[project.id]);
  const syncClone = useProjectStore((s) => s.syncClone);

  const state = explicit ?? (project.clone_path ? "done" : "idle");
  const busy = state === "cloning";

  return (
    <div className="flex items-center gap-2 text-xs">
      {state === "done" && (
        <span className="flex items-center gap-1 text-fg-subtle">
          <Check className="h-3.5 w-3.5" /> cached
        </span>
      )}
      {state === "cloning" && <span className="text-fg-subtle">cloning…</span>}
      {state === "error" && (
        <span className="text-danger" title={cloneError ?? undefined}>
          clone failed
        </span>
      )}
      {state === "idle" && <span className="text-fg-subtle">not cached</span>}

      <button
        type="button"
        onClick={() => void syncClone(project.id)}
        disabled={busy}
        title="Refresh clone"
        aria-label="Refresh clone"
        className="flex items-center gap-1 rounded-md border border-border px-2 py-1 text-fg-muted hover:bg-surface-2 disabled:opacity-60"
      >
        <RefreshCw className={"h-3.5 w-3.5 " + (busy ? "animate-spin" : "")} /> Refresh
      </button>
    </div>
  );
}
