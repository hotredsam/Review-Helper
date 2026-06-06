import { useEffect, useState, type FormEvent } from "react";
import { Plus, Mic, X, AlertTriangle } from "lucide-react";
import { useFeaturesStore } from "../store/featuresStore";
import { usePlanStore } from "../store/planStore";
import { transcribeAudioStub, type Feature } from "../api/features";
import type { Project } from "../api/projects";

const EMPTY: Feature[] = [];
const NUDGE_AT = 10;

const STATUS_LABEL: Record<string, string> = {
  inbox: "Inbox",
  triaged: "Triaged",
  in_plan: "In plan",
  rejected: "Rejected",
};

/** The feature inbox: capture ideas (text now, audio via a stub), see the
 *  queue, and triage. "Update plan" (T2) weaves pending items into the plan. */
export function InboxPane({ project }: { project: Project }) {
  const id = project.id;
  const featuresRaw = useFeaturesStore((s) => s.features[id]);
  const features = featuresRaw ?? EMPTY;
  const error = useFeaturesStore((s) => s.error[id]);
  const load = useFeaturesStore((s) => s.load);
  const add = useFeaturesStore((s) => s.add);
  const setStatus = useFeaturesStore((s) => s.setStatus);
  const updatePlan = usePlanStore((s) => s.update);

  const [title, setTitle] = useState("");
  const [micNote, setMicNote] = useState<string | null>(null);

  useEffect(() => {
    void load(id);
  }, [id, load]);

  const pending = features.filter((f) => f.status === "inbox" || f.status === "triaged").length;

  const submit = (e: FormEvent) => {
    e.preventDefault();
    const t = title.trim();
    if (!t) return;
    void add(id, t, "");
    setTitle("");
  };

  const onMic = async () => {
    setMicNote(await transcribeAudioStub());
  };

  return (
    <div className="mx-auto max-w-3xl space-y-5 p-8">
      <form onSubmit={submit} className="flex gap-2">
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="Capture a feature idea…"
          aria-label="Feature idea"
          className="flex-1 rounded-lg border border-border bg-surface px-3 py-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none focus:ring-2 focus:ring-ring/40"
        />
        <button
          type="button"
          onClick={() => void onMic()}
          aria-label="Capture by voice"
          title="Capture by voice"
          className="flex h-10 w-10 items-center justify-center rounded-lg border border-border text-fg-muted hover:bg-surface-2"
        >
          <Mic className="h-4 w-4" />
        </button>
        <button
          type="submit"
          disabled={!title.trim()}
          className="flex items-center gap-1.5 rounded-lg bg-accent px-3 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
        >
          <Plus className="h-4 w-4" /> Add
        </button>
      </form>

      {micNote && (
        <p className="rounded-md border border-border bg-surface-2 px-3 py-2 text-xs text-fg-muted" role="status">
          {micNote}
        </p>
      )}
      {error && (
        <p className="text-sm text-danger" role="alert">
          {error}
        </p>
      )}

      {pending >= NUDGE_AT && (
        <div className="flex items-center justify-between gap-2 rounded-lg border border-warning/40 bg-warning/10 px-3 py-2 text-sm text-warning">
          <span className="flex items-center gap-2">
            <AlertTriangle className="h-4 w-4" />
            {pending} ideas waiting — a good time to update the plan (you choose when).
          </span>
          <button
            type="button"
            onClick={() => void updatePlan(id)}
            className="shrink-0 rounded-md bg-accent px-2.5 py-1 text-xs font-medium text-accent-fg hover:bg-accent-hover"
          >
            Update plan
          </button>
        </div>
      )}

      {features.length === 0 ? (
        <p className="text-sm text-fg-subtle">
          Inbox empty. Capture ideas as they come; triage them into the plan later.
        </p>
      ) : (
        <ul className="space-y-2">
          {features.map((f) => (
            <li
              key={f.id}
              className="flex items-center justify-between gap-3 rounded-lg border border-border bg-surface p-3"
            >
              <div className="min-w-0">
                <span className="mr-2 rounded-full bg-surface-2 px-2 py-0.5 text-xs text-fg-subtle">
                  {STATUS_LABEL[f.status] ?? f.status}
                </span>
                <span className="text-sm text-fg">{f.title}</span>
                {f.detail && <span className="ml-2 text-xs text-fg-subtle">{f.detail}</span>}
              </div>
              {f.status !== "rejected" && (
                <button
                  type="button"
                  onClick={() => void setStatus(id, f.id, "rejected")}
                  aria-label={`Reject ${f.title}`}
                  className="flex shrink-0 items-center gap-1 rounded-md border border-border px-2 py-0.5 text-xs text-fg-muted hover:bg-surface-2"
                >
                  <X className="h-3 w-3" /> Reject
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
