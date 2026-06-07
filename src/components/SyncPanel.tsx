import { useState } from "react";
import { GitBranch, Trash2 } from "lucide-react";
import { syncPushPlanning, syncMainPreview, syncMainApply, type SyncPreview } from "../api/sync";
import type { Project } from "../api/projects";

/**
 * GitHub sync-out. Pushing to the planning branch is non-destructive. Pushing to
 * main shows EVERY change first — issue creates/updates/closes AND file
 * deletions — and applies only the exact previewed actions on an explicit
 * confirm. Nothing is closed/deleted on GitHub without that preview.
 */
export function SyncPanel({ project }: { project: Project }) {
  const id = project.id;
  const [preview, setPreview] = useState<SyncPreview | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!project.github_repo_url) return null;

  const run = async (key: string, fn: () => Promise<void>) => {
    if (busy) return;
    setBusy(key);
    setError(null);
    setMsg(null);
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const destructive = preview ? preview.issue_actions.filter((a) => a.kind === "close").length + preview.file_deletions.length : 0;

  return (
    <section className="space-y-3 rounded-lg border border-border bg-surface p-4">
      <div className="flex items-center gap-2">
        <GitBranch className="h-4 w-4 text-fg-subtle" />
        <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">GitHub sync</h2>
      </div>
      <p className="text-sm text-fg-muted">
        Push the planning package and phase issues. Every change to main — including issue closes and
        file deletions — is shown before anything is written.
      </p>

      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          disabled={!!busy}
          onClick={() =>
            void run("planning", async () => {
              const n = await syncPushPlanning(id);
              setMsg(`Pushed ${n} files to the planning branch.`);
            })
          }
          className="rounded-md border border-border px-3 py-1.5 text-xs text-fg-muted hover:bg-surface-2 disabled:opacity-60"
        >
          {busy === "planning" ? "Pushing…" : "Push to planning branch"}
        </button>
        <button
          type="button"
          disabled={!!busy}
          onClick={() =>
            void run("preview", async () => {
              setMsg(null);
              setPreview(await syncMainPreview(id));
            })
          }
          className="rounded-md border border-border px-3 py-1.5 text-xs text-fg-muted hover:bg-surface-2 disabled:opacity-60"
        >
          {busy === "preview" ? "Loading…" : "Preview push to main"}
        </button>
      </div>

      {msg && (
        <p className="text-sm text-success" role="status">
          {msg}
        </p>
      )}
      {error && (
        <p className="text-sm text-danger" role="alert">
          {error}
        </p>
      )}

      {preview && (
        <div className="space-y-3 rounded-md border border-border bg-surface-2 p-3">
          <div>
            <h3 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">
              Issue changes ({preview.issue_actions.length})
            </h3>
            {preview.issue_actions.length === 0 ? (
              <p className="text-sm text-fg-subtle">No issue changes.</p>
            ) : (
              <ul className="mt-1 space-y-0.5 text-sm">
                {preview.issue_actions.map((a, i) => (
                  <li key={i} className="text-fg-muted">
                    <span className="font-medium capitalize text-fg">{a.kind}</span>{" "}
                    {a.kind === "close" ? `#${a.number} ` : ""}
                    {"title" in a ? a.title : ""}
                  </li>
                ))}
              </ul>
            )}
          </div>

          {preview.file_deletions.length > 0 && (
            <div>
              <h3 className="flex items-center gap-1 text-xs font-semibold uppercase tracking-wide text-danger">
                <Trash2 className="h-3 w-3" /> Files to delete ({preview.file_deletions.length})
              </h3>
              <ul className="mt-1 space-y-0.5 text-sm">
                {preview.file_deletions.map((p) => (
                  <li key={p} className="text-fg-muted">
                    {p}
                  </li>
                ))}
              </ul>
            </div>
          )}

          <button
            type="button"
            disabled={!!busy}
            onClick={() =>
              void run("apply", async () => {
                const r = await syncMainApply(id, preview);
                if (r.failures.length === 0) {
                  setPreview(null);
                  setMsg(
                    `Pushed ${r.files_pushed} files, synced ${r.issues_applied} issue(s), removed ${r.files_deleted} stale file(s).`,
                  );
                } else {
                  // Keep the preview visible on partial failure.
                  setError(`Some steps failed: ${r.failures.join("; ")}`);
                  setMsg(`Partial: ${r.files_pushed} pushed, ${r.issues_applied} issues, ${r.files_deleted} deleted.`);
                }
              })
            }
            className={
              "rounded-md px-3 py-1.5 text-xs font-medium disabled:opacity-60 " +
              (destructive > 0
                ? "bg-danger text-danger-fg hover:opacity-90"
                : "bg-accent text-accent-fg hover:bg-accent-hover")
            }
          >
            {busy === "apply"
              ? "Pushing…"
              : destructive > 0
                ? `Confirm: push to main (${destructive} destructive)`
                : "Confirm: push to main"}
          </button>
        </div>
      )}
    </section>
  );
}
