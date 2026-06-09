import { useState } from "react";
import { Loader2, Check, X, HelpCircle, MessagesSquare, Trash2, type LucideIcon } from "lucide-react";
import { useGrillStore } from "../store/grillStore";
import type { Question } from "../api/grill";

/**
 * One grill question with its five actions: Submit (typed answer), Not relevant,
 * I don't know, Let's chat about this (writes the resolution back into the card),
 * Delete. On success the card moves to addressed (the store reloads).
 */
export function QuestionCard({ projectId, question }: { projectId: number; question: Question }) {
  const answer = useGrillStore((s) => s.answer);
  const chatResolve = useGrillStore((s) => s.chatResolve);
  const setStatus = useGrillStore((s) => s.setStatus);
  const remove = useGrillStore((s) => s.remove);

  const [body, setBody] = useState("");
  const [chatOpen, setChatOpen] = useState(false);
  const [chatNote, setChatNote] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const chatId = `grill-chat-${question.id}`;

  const run = async (key: string, fn: () => Promise<void>) => {
    if (busy) return;
    setBusy(key);
    setError(null);
    try {
      await fn(); // success → reload removes this card; leave busy set
    } catch (e) {
      setError(String(e));
      setBusy(null);
    }
  };

  return (
    <li className="rounded-lg border border-border bg-surface p-4">
      <div className="mb-1 flex items-center gap-2">
        {question.dimension && (
          <span className="rounded-full bg-surface-2 px-2 py-0.5 text-xs capitalize text-fg-muted">
            {question.dimension}
          </span>
        )}
        {question.bank_topic && <span className="text-xs text-fg-subtle">{question.bank_topic}</span>}
      </div>
      <p className="text-sm font-medium text-fg">{question.text}</p>
      {question.recommended_answer && (
        <p className="mt-1.5 text-sm text-fg-muted">
          <span className="font-medium text-fg-subtle">Recommended:</span> {question.recommended_answer}{" "}
          <button
            type="button"
            onClick={() => setBody(question.recommended_answer ?? "")}
            className="ml-1 rounded-md border border-border bg-surface-2 px-2 py-0.5 text-xs font-medium text-fg-muted hover:bg-surface hover:text-fg"
          >
            Use this
          </button>
        </p>
      )}

      <textarea
        value={body}
        onChange={(e) => setBody(e.target.value)}
        rows={2}
        maxLength={10000}
        aria-label={`Answer: ${question.text}`}
        placeholder="Your answer…"
        className="mt-2 w-full rounded-md border border-border bg-surface-2 px-2 py-1.5 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none"
      />

      <div className="mt-2 flex flex-wrap gap-2">
        <Action label="Submit" icon={Check} busy={busy === "submit"} disabled={!body.trim()} primary
          onClick={() => void run("submit", () => answer(projectId, question.id, body))} />
        <Action label="Not relevant" icon={X} busy={busy === "nr"}
          onClick={() => void run("nr", () => setStatus(projectId, question.id, "not_relevant"))} />
        <Action label="I don't know" icon={HelpCircle} busy={busy === "idk"}
          onClick={() => void run("idk", () => setStatus(projectId, question.id, "unknown"))} />
        <Action label="Let's chat" icon={MessagesSquare} busy={false}
          ariaExpanded={chatOpen} ariaControls={chatId}
          onClick={() => setChatOpen((v) => !v)} />
        <Action label="Delete" icon={Trash2} busy={busy === "del"}
          onClick={() => void run("del", () => remove(projectId, question.id))} />
      </div>

      {chatOpen && (
        <div id={chatId} className="mt-2 rounded-md border border-border bg-surface-2 p-2">
          <p className="mb-1 text-xs text-fg-subtle">
            Note what you concluded — the full chat opens here in a later phase:
          </p>
          <textarea
            value={chatNote}
            onChange={(e) => setChatNote(e.target.value)}
            rows={2}
            maxLength={10000}
            aria-label="Chat resolution"
            placeholder="What did you decide?"
            className="w-full rounded-md border border-border bg-surface px-2 py-1.5 text-sm text-fg focus:border-accent focus:outline-none"
          />
          <button
            type="button"
            disabled={!chatNote.trim() || !!busy}
            onClick={() => void run("chat", () => chatResolve(projectId, question.id, chatNote))}
            className="mt-1 rounded-md bg-accent px-3 py-1 text-xs font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
          >
            Save resolution
          </button>
        </div>
      )}

      {error && (
        <p className="mt-2 text-xs text-danger" role="alert">
          {error}
        </p>
      )}
    </li>
  );
}

function Action({
  label,
  icon: Icon,
  busy,
  disabled,
  primary,
  ariaExpanded,
  ariaControls,
  onClick,
}: {
  label: string;
  icon: LucideIcon;
  busy: boolean;
  disabled?: boolean;
  primary?: boolean;
  ariaExpanded?: boolean;
  ariaControls?: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={busy || disabled}
      aria-busy={busy}
      aria-expanded={ariaExpanded}
      aria-controls={ariaControls}
      className={
        "flex items-center gap-1 rounded-md px-2.5 py-1 text-xs disabled:opacity-60 " +
        (primary
          ? "bg-accent text-accent-fg hover:bg-accent-hover"
          : "border border-border text-fg-muted hover:bg-surface-2")
      }
    >
      {busy ? <Loader2 className="h-3 w-3 animate-spin" /> : <Icon className="h-3 w-3" />} {label}
    </button>
  );
}
