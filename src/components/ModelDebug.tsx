import { RotateCw } from "lucide-react";
import { useStatusStore } from "../store/statusStore";

/** Debug foldout (Settings): the last provider probe — command, exit code,
 *  stderr, version — plus a Recheck button. */
export function ModelDebug() {
  const status = useStatusStore((s) => s.status);
  const loading = useStatusStore((s) => s.loading);
  const refresh = useStatusStore((s) => s.refresh);

  return (
    <details className="rounded-lg border border-border bg-surface">
      <summary className="cursor-pointer px-3 py-2 text-sm font-medium text-fg">Debug</summary>
      <div className="space-y-3 border-t border-border p-3 text-xs">
        {!status && <p className="text-fg-subtle">No probe yet.</p>}
        {status && (
          <dl className="space-y-1 text-fg-muted">
            <Row label="provider" value={status.provider} />
            <Row label="available" value={String(status.available)} />
            {status.version && <Row label="version" value={status.version} />}
            <Row label="command" value={status.command} mono />
            <Row label="exit" value={status.exit_code === null ? "—" : String(status.exit_code)} />
            {status.reason && <Row label="reason" value={status.reason} />}
            {status.stderr && (
              <div>
                <span className="text-fg-subtle">stderr: </span>
                <span className="whitespace-pre-wrap text-danger">{status.stderr}</span>
              </div>
            )}
          </dl>
        )}
        <button
          type="button"
          onClick={() => void refresh()}
          disabled={loading}
          className="flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1 text-fg-muted hover:bg-surface-2 disabled:opacity-60"
        >
          <RotateCw className={"h-3.5 w-3.5 " + (loading ? "animate-spin" : "")} />
          Recheck
        </button>
      </div>
    </details>
  );
}

function Row({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <span className="text-fg-subtle">{label}: </span>
      <span className={mono ? "font-mono text-fg" : "text-fg"}>{value}</span>
    </div>
  );
}
