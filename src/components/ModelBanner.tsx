import { AlertTriangle, RotateCw } from "lucide-react";
import { useStatusStore } from "../store/statusStore";

/**
 * Global banner shown only when the active provider is Claude and it isn't
 * available. The app stays read-only regardless; Retry re-probes.
 */
export function ModelBanner() {
  const status = useStatusStore((s) => s.status);
  const loading = useStatusStore((s) => s.loading);
  const refresh = useStatusStore((s) => s.refresh);

  if (!status || status.provider !== "claude" || status.available) return null;

  const reason = (status.reason ?? "unknown").replace(/_/g, " ");
  return (
    <div
      role="alert"
      className="flex items-center gap-3 border-b border-border bg-warning/15 px-4 py-2 text-sm text-fg"
    >
      <AlertTriangle className="h-4 w-4 shrink-0 text-warning" />
      <span className="flex-1">
        Claude not available ({reason}). Planning is paused — the app stays read-only.
      </span>
      <button
        type="button"
        onClick={() => void refresh()}
        disabled={loading}
        className="flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1 text-xs font-medium text-fg hover:bg-surface-2 disabled:opacity-60"
      >
        <RotateCw className={"h-3.5 w-3.5 " + (loading ? "animate-spin" : "")} />
        Retry
      </button>
    </div>
  );
}
