import { useEffect, useRef, useState, type FormEvent } from "react";
import { Loader2, Send, Square } from "lucide-react";
import { modelStop } from "../../api/model";
import { Globe } from "lucide-react";
import { type TutorMsg, learningTutorHistory, learningTutorSend } from "../../api/learning";
import { MarkdownBlock } from "../MarkdownBlock";

/** The subject's tutor: a persistent chat that adapts to the learner's profile
 *  (mastery + pace). History survives restarts. Never dead-ends — send failures
 *  surface inline and the question stays in the box to retry. */
export function TutorPane({ subjectId }: { subjectId: number }) {
  const [messages, setMessages] = useState<TutorMsg[]>([]);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sources, setSources] = useState<string[]>([]);
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let live = true;
    learningTutorHistory(subjectId)
      .then((h) => live && setMessages(h))
      .catch((e) => live && setError(String(e)));
    return () => {
      live = false;
    };
  }, [subjectId]);

  useEffect(() => {
    endRef.current?.scrollIntoView({ block: "end" });
  }, [messages, busy]);

  const send = async (e: FormEvent) => {
    e.preventDefault();
    const text = draft.trim();
    if (!text || busy) return;
    setBusy(true);
    setError(null);
    setMessages((m) => [...m, { role: "user", content: text }]);
    setDraft("");
    try {
      const r = await learningTutorSend(subjectId, text);
      setMessages((m) => [...m, { role: "assistant", content: r.reply, grounding: r.grounding }]);
      setSources(r.sources);
    } catch (err) {
      setError(String(err));
      // Drop the optimistic user bubble and restore the draft to retry.
      setMessages((m) => m.slice(0, -1));
      setDraft(text);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-[60vh] flex-col rounded-xl border border-border bg-surface">
      <div className="flex-1 space-y-3 overflow-y-auto p-4">
        {messages.length === 0 && !busy && (
          <p className="text-sm text-fg-subtle">
            Ask the tutor anything about this subject. It knows where you're strong and where to push — and explains at
            your level.
          </p>
        )}
        {messages.map((m, i) => (
          <div key={i} className={m.role === "user" ? "flex justify-end" : "flex justify-start"}>
            <div
              className={
                "max-w-[85%] rounded-2xl px-3 py-2 text-sm " +
                (m.role === "user" ? "bg-accent text-accent-fg" : "border border-border bg-bg text-fg")
              }
            >
              {m.role === "user" ? (
                <span className="whitespace-pre-wrap">{m.content}</span>
              ) : (
                <>
                  {(m.grounding === "web" || m.grounding === "mixed") && (
                    <span className="mb-1 flex w-fit items-center gap-1 rounded-full bg-warning/15 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-warning">
                      <Globe className="h-3 w-3" /> Includes web results
                    </span>
                  )}
                  <MarkdownBlock>{m.content}</MarkdownBlock>
                </>
              )}
            </div>
          </div>
        ))}
        {sources.length > 0 && (
          <div className="ml-1 text-[11px] text-fg-subtle" aria-label="Sources">
            Sources: {sources.map((s, i) => `[${i + 1}] ${s}`).join("  ·  ")}
          </div>
        )}
        {busy && (
          <div className="flex justify-start">
            <div className="flex items-center gap-2 rounded-2xl border border-border bg-bg px-3 py-2 text-sm text-fg-subtle">
              <Loader2 className="h-4 w-4 animate-spin" /> Thinking…
            </div>
          </div>
        )}
        <div ref={endRef} />
      </div>

      {error && (
        <p className="px-4 pb-1 text-sm text-danger" role="alert">
          {error}
        </p>
      )}

      <form onSubmit={(e) => void send(e)} className="flex gap-2 border-t border-border p-3">
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          maxLength={20000}
          placeholder="Ask the tutor…"
          aria-label="Message the tutor"
          className="flex-1 rounded-lg border border-border bg-bg px-3 py-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none focus:ring-2 focus:ring-ring/40"
        />
        {busy ? (
          <button
            type="button"
            onClick={() => void modelStop(`tutor:${subjectId}`)}
            aria-label="Stop"
            title="Stop the tutor's reply"
            className="flex items-center gap-1.5 rounded-lg bg-danger px-3 py-2 text-sm font-medium text-danger-fg hover:opacity-90"
          >
            <Square className="h-4 w-4" />
          </button>
        ) : (
          <button
            type="submit"
            disabled={!draft.trim()}
            aria-label="Send"
            className="flex items-center gap-1.5 rounded-lg bg-accent px-3 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
          >
            <Send className="h-4 w-4" />
          </button>
        )}
      </form>
    </div>
  );
}
