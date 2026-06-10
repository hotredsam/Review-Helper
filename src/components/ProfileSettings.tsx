import { useEffect, useState } from "react";
import { Loader2, RefreshCcw, Save } from "lucide-react";
import {
  profileGet,
  profileReset,
  profileReflect,
  profileSaveNotes,
  profileSetEnabled,
  type ProfileStatus,
} from "../api/profile";
import { ConfirmDialog } from "./ConfirmDialog";
import { useUiStore } from "../store/uiStore";

const NOTES_HEADER = "## Your notes (never auto-edited)";

function splitNotes(content: string): { auto: string; notes: string } {
  const idx = content.indexOf(NOTES_HEADER);
  if (idx === -1) return { auto: content, notes: "" };
  return { auto: content.slice(0, idx), notes: content.slice(idx + NOTES_HEADER.length).trim() };
}

const FILE_LABEL: Record<string, string> = {
  "learner-profile.md": "How you learn (Learning mode)",
  "review-preferences.md": "How you like reviews (Plan / Grill / Assess / Chat)",
};

/** The adaptive profile: human-readable MD files the app maintains from
 *  measured behavior. Facts + Observations are automatic; Your notes are
 *  sacred. One master toggle kills capture, reflection, and injection. */
export function ProfileSettings() {
  const [status, setStatus] = useState<ProfileStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [resetTarget, setResetTarget] = useState<string | null>(null);
  const [reflecting, setReflecting] = useState(false);
  const notify = (m: string) => useUiStore.getState().setNotice(m);

  const load = () =>
    profileGet()
      .then((s) => {
        setStatus(s);
        setDrafts(Object.fromEntries(s.files.map((f) => [f.name, splitNotes(f.content).notes])));
      })
      .catch((e) => setError(String(e)));

  useEffect(() => {
    void load();
  }, []);

  if (error) {
    return (
      <p className="text-sm text-danger" role="alert">
        Couldn't load your profile: {error}
      </p>
    );
  }
  if (!status) return <p className="text-sm text-fg-subtle">Loading…</p>;

  return (
    <div className="space-y-4">
      <label className="flex items-center justify-between gap-3 text-sm text-fg">
        <span>
          Adaptive profile
          <span className="block text-xs text-fg-subtle">
            Learns how you study and review from measured behavior — capture, one cheap end-of-session
            reflection, and prompt hints all switch off together.
          </span>
        </span>
        <input
          type="checkbox"
          checked={status.enabled}
          onChange={(e) =>
            void profileSetEnabled(e.target.checked)
              .then(load)
              .catch((err) => setError(String(err)))
          }
          aria-label="Adaptive profile enabled"
        />
      </label>

      <div className="flex items-center justify-between text-xs text-fg-subtle">
        <span>{status.unreflected_events} new signals since the last reflection (runs at 15+).</span>
        <button
          type="button"
          disabled={reflecting || !status.enabled}
          onClick={() => {
            setReflecting(true);
            profileReflect()
              .then((r) => {
                notify(`Profile reflection: ${r}.`);
                return load();
              })
              .catch((e) => setError(String(e)))
              .finally(() => setReflecting(false));
          }}
          className="flex items-center gap-1 rounded-md border border-border px-2 py-0.5 text-xs text-fg-muted hover:bg-surface-2 disabled:opacity-60"
        >
          {reflecting ? <Loader2 className="h-3 w-3 animate-spin" /> : <RefreshCcw className="h-3 w-3" />}
          Reflect now
        </button>
      </div>

      {status.files.map((f) => {
        const { auto } = splitNotes(f.content);
        return (
          <details key={f.name} className="rounded-lg border border-border bg-surface p-3">
            <summary className="cursor-pointer text-sm font-medium text-fg">
              {FILE_LABEL[f.name] ?? f.name}
            </summary>
            <pre className="mt-2 max-h-56 overflow-auto whitespace-pre-wrap rounded-md bg-surface-2 p-2 text-xs text-fg-muted">
              {auto.trim()}
            </pre>
            <label className="mt-2 block text-xs font-medium text-fg-subtle" htmlFor={`notes-${f.name}`}>
              Your notes (never auto-edited)
            </label>
            <textarea
              id={`notes-${f.name}`}
              value={drafts[f.name] ?? ""}
              onChange={(e) => setDrafts((d) => ({ ...d, [f.name]: e.target.value }))}
              rows={3}
              className="mt-1 w-full rounded-md border border-border bg-bg px-2 py-1.5 text-xs text-fg focus:border-accent focus:outline-none"
            />
            <div className="mt-2 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setResetTarget(f.name)}
                className="rounded-md border border-border px-2 py-1 text-xs text-fg-muted hover:bg-surface-2"
              >
                Reset auto sections
              </button>
              <button
                type="button"
                onClick={() =>
                  void profileSaveNotes(f.name, drafts[f.name] ?? "")
                    .then(() => {
                      notify("Notes saved.");
                      return load();
                    })
                    .catch((e) => setError(String(e)))
                }
                className="flex items-center gap-1 rounded-md bg-accent px-2 py-1 text-xs font-medium text-accent-fg hover:bg-accent-hover"
              >
                <Save className="h-3 w-3" /> Save notes
              </button>
            </div>
          </details>
        );
      })}

      <ConfirmDialog
        open={resetTarget !== null}
        title="Reset the automatic sections?"
        body="Facts and Observations in this file are cleared (they rebuild from future sessions). Your notes are untouched."
        confirmLabel="Reset"
        onConfirm={() => {
          if (resetTarget) {
            void profileReset(resetTarget)
              .then(() => {
                notify("Auto sections reset.");
                return load();
              })
              .catch((e) => setError(String(e)));
          }
          setResetTarget(null);
        }}
        onCancel={() => setResetTarget(null)}
      />
    </div>
  );
}
