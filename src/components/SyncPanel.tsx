import { useState } from "react";
import { GitBranch } from "lucide-react";
import {
  syncPushPlanning,
  syncIssuePreview,
  syncIssueApply,
  syncPushMain,
  type IssueAction,
} from "../api/sync";
import type { Project } from "../api/projects";

/**
 * GitHub sync-out. Pushing to the planning branch is non-destructive. Pushing to
 * main + changing issues is always previewed first and applied only on an
 * explicit confirm — nothing is closed/deleted on GitHub without the preview.
 */
export function SyncPanel({ project }: { project: Project }) {
  const id = project.id;
  const [preview, setPreview] = useState<IssueAction[] | null>(null);
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

  return (
    <section className="space-y-3 rounded-lg border border-border bg-surface p-4">
      <div className="flex items-center gap-2">
        <GitBranch className="h-4 w-4 text-fg-subtle" />
        <h2 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">GitHub sync</h2>
      </div>
      <p className="text-sm text-fg-muted">
        Push the planning package and phase issues. Changes to main and to issues are previewed before
        anything is written.
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
              setPreview(await syncIssuePreview(id));
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
        <div className="space-y-2 rounded-md border border-border bg-surface-2 p-3">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">
            Issue changes ({preview.length})
          </h3>
          {preview.length === 0 ? (
            <p className="text-sm text-fg-subtle">No issue changes — docs will still be pushed.</p>
          ) : (
            <ul className="space-y-0.5 text-sm">
              {preview.map((a, i) => (
                <li key={i} className="text-fg-muted">
                  <span className="font-medium capitalize text-fg">{a.kind}</span>{" "}
                  {a.kind === "close" ? `#${a.number} ` : ""}
                  {"title" in a ? a.title : ""}
                </li>
              ))}
            </ul>
          )}
          <button
            type="button"
            disabled={!!busy}
            onClick={() =>
              void run("apply", async () => {
                const issues = await syncIssueApply(id);
                const files = await syncPushMain(id);
                setPreview(null);
                setMsg(`Synced ${issues} issue(s) and pushed ${files} files to main.`);
              })
            }
            className="rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
          >
            {busy === "apply" ? "Pushing…" : "Confirm: sync issues + push to main"}
          </button>
        </div>
      )}
    </section>
  );
}
