import { useEffect, useRef, useState, type FormEvent } from "react";
import { Plus, Mic, Square, X, AlertTriangle, Loader2 } from "lucide-react";
import { useFeaturesStore } from "../store/featuresStore";
import { usePlanStore } from "../store/planStore";
import type { Feature } from "../api/features";
import { transcribeStart, transcribeStop, transcribeCancel, onTranscribeEvent, type TranscribeEvent } from "../api/transcribe";
import { ConfirmDialog } from "./ConfirmDialog";
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
  const planBusy = usePlanStore((s) => s.analysis[id] === "running");

  const [title, setTitle] = useState("");
  const [confirmReject, setConfirmReject] = useState<Feature | null>(null);

  type MicState = "idle" | "starting" | "recording" | "transcribing";
  const [mic, setMic] = useState<MicState>("idle");
  const [micPartial, setMicPartial] = useState("");
  const [micStatus, setMicStatus] = useState<string | null>(null);
  const [micError, setMicError] = useState<string | null>(null);
  const micRef = useRef<MicState>("idle");
  micRef.current = mic;

  useEffect(() => {
    void load(id);
  }, [id, load]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    void onTranscribeEvent((e: TranscribeEvent) => {
      switch (e.type) {
        case "state":
          if (e.state === "downloading") setMicStatus("Downloading the speech model (one-time, ~550 MB)…");
          else if (e.state === "loading_model") setMicStatus("Loading the speech model…");
          else if (e.state === "recording") setMicStatus(null);
          else if (e.state === "transcribing") setMicStatus("Finishing the transcript…");
          break;
        case "model_download":
          setMicStatus(`Downloading the speech model… ${Math.round((e.done / e.total) * 100)}%`);
          break;
        case "partial":
          setMicPartial(e.text);
          break;
        case "error":
          setMicError(e.detail);
          break;
        case "final":
          break;
      }
    }).then((u) => {
      unlisten = u;
    });
    return () => {
      unlisten?.();
      // Leaving the pane mid-recording discards the session — never a zombie mic.
      if (micRef.current === "recording" || micRef.current === "starting") void transcribeCancel();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const pending = features.filter((f) => f.status === "inbox" || f.status === "triaged").length;

  const submit = (e: FormEvent) => {
    e.preventDefault();
    const t = title.trim();
    if (!t) return;
    void add(id, t, "");
    setTitle("");
  };

  const onMic = async () => {
    setMicError(null);
    if (mic === "idle") {
      setMic("starting");
      setMicPartial("");
      try {
        await transcribeStart();
        setMic("recording");
      } catch (e) {
        setMicError(String(e));
        setMic("idle");
        setMicStatus(null);
      }
      return;
    }
    if (mic === "recording") {
      setMic("transcribing");
      try {
        const text = await transcribeStop();
        if (text.trim()) setTitle((t) => (t.trim() ? `${t.trim()} ${text.trim()}` : text.trim()));
      } catch (e) {
        setMicError(String(e));
      } finally {
        setMic("idle");
        setMicPartial("");
        setMicStatus(null);
      }
    }
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
          disabled={mic === "starting" || mic === "transcribing"}
          aria-label={mic === "recording" ? "Stop recording" : "Capture by voice"}
          title={mic === "recording" ? "Stop recording" : "Capture by voice"}
          className={
            "flex h-10 w-10 items-center justify-center rounded-lg disabled:opacity-60 " +
            (mic === "recording"
              ? "bg-danger text-danger-fg hover:opacity-90"
              : "border border-border text-fg-muted hover:bg-surface-2")
          }
        >
          {mic === "starting" || mic === "transcribing" ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : mic === "recording" ? (
            <Square className="h-4 w-4" />
          ) : (
            <Mic className="h-4 w-4" />
          )}
        </button>
        <button
          type="submit"
          disabled={!title.trim()}
          className="flex items-center gap-1.5 rounded-lg bg-accent px-3 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
        >
          <Plus className="h-4 w-4" /> Add
        </button>
      </form>

      {(micStatus || mic === "recording") && (
        <p className="rounded-md border border-border bg-surface-2 px-3 py-2 text-xs text-fg-muted" role="status">
          {micStatus ?? (micPartial ? micPartial : "Listening… speak your idea, then press stop.")}
        </p>
      )}
      {micError && (
        <p className="text-sm text-danger" role="alert">
          {micError}
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
            disabled={planBusy}
            className="shrink-0 rounded-md bg-accent px-2.5 py-1 text-xs font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
          >
            {planBusy ? "Updating…" : "Update plan"}
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
                <span className="mr-2 rounded-full bg-surface-2 px-2 py-0.5 text-xs text-fg-muted">
                  {STATUS_LABEL[f.status] ?? f.status}
                </span>
                <span className="text-sm text-fg">{f.title}</span>
                {f.detail && <span className="ml-2 text-xs text-fg-subtle">{f.detail}</span>}
              </div>
              {f.status !== "rejected" && (
                <button
                  type="button"
                  onClick={() => setConfirmReject(f)}
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

      <ConfirmDialog
        open={confirmReject !== null}
        title="Reject this idea?"
        body={`"${confirmReject?.title ?? ""}" is marked rejected. There's no way to un-reject it from the UI.`}
        confirmLabel="Reject"
        onConfirm={() => {
          if (confirmReject) void setStatus(id, confirmReject.id, "rejected");
          setConfirmReject(null);
        }}
        onCancel={() => setConfirmReject(null)}
      />
    </div>
  );
}
