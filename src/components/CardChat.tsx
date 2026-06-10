import { useEffect, useRef, useState } from "react";
import { Loader2, Send, Sparkles } from "lucide-react";
import { cardChatHistory, cardChatSend, cardPremadeQuestions, type CardMsg } from "../api/cards";

/** Inline mini-chat scoped to one concept card: model-generated starter
 *  questions (cached) you can tap, plus a free-form box. Persists per
 *  project + term. The answer leads with the direct answer (backend prompt). */
export function CardChat({ project, term }: { project: number; term: string }) {
  const [msgs, setMsgs] = useState<CardMsg[]>([]);
  const [questions, setQuestions] = useState<string[]>([]);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Identity of the card currently on screen; a reply that resolves after the
  // user switched cards is dropped instead of landing in the wrong chat.
  const keyRef = useRef(`${project}:${term}`);
  useEffect(() => {
    keyRef.current = `${project}:${term}`;
  }, [project, term]);

  useEffect(() => {
    let alive = true;
    setMsgs([]);
    setQuestions([]);
    setError(null);
    void cardChatHistory(project, term).then((h) => alive && setMsgs(h)).catch(() => {});
    void cardPremadeQuestions(term).then((q) => alive && setQuestions(q)).catch(() => {});
    return () => {
      alive = false;
    };
  }, [project, term]);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [msgs]);

  const ask = async (q: string) => {
    const m = q.trim();
    if (!m || busy) return;
    setBusy(true);
    setError(null);
    setMsgs((prev) => [...prev, { role: "user", content: m }]);
    setDraft("");
    const key = `${project}:${term}`;
    try {
      const reply = await cardChatSend(project, term, m);
      if (keyRef.current !== key) return; // user switched cards mid-flight
      setMsgs((prev) => [...prev, { role: "assistant", content: reply }]);
    } catch (e) {
      if (keyRef.current === key) setError(String(e));
    } finally {
      if (keyRef.current === key) setBusy(false);
    }
  };

  return (
    <div className="mt-4 border-t border-border pt-3">
      <div className="mb-2 flex items-center gap-1.5">
        <Sparkles className="h-3.5 w-3.5 text-accent" />
        <h3 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Ask about this</h3>
      </div>

      {questions.length > 0 && msgs.length === 0 && (
        <div className="mb-2 flex flex-wrap gap-1.5" aria-label="Suggested questions">
          {questions.map((q, i) => (
            <button
              key={i}
              type="button"
              onClick={() => void ask(q)}
              className="rounded-full border border-border px-2.5 py-1 text-xs text-fg-muted hover:bg-surface-2 hover:text-fg"
            >
              {q}
            </button>
          ))}
        </div>
      )}

      {msgs.length > 0 && (
        <div ref={scrollRef} className="mb-2 max-h-60 space-y-2 overflow-auto">
          {msgs.map((m, i) => (
            <div key={i} className={m.role === "user" ? "flex justify-end" : "flex justify-start"}>
              <div
                className={
                  "max-w-[85%] whitespace-pre-wrap rounded-2xl px-3 py-1.5 text-sm " +
                  (m.role === "user" ? "bg-accent text-accent-fg" : "bg-surface-2 text-fg")
                }
              >
                {m.content}
              </div>
            </div>
          ))}
          {busy && (
            <div className="flex justify-start">
              <Loader2 className="h-4 w-4 animate-spin text-fg-subtle" role="status" aria-label="Thinking" />
            </div>
          )}
        </div>
      )}

      {error && (
        <p className="mb-2 text-xs text-danger" role="alert">
          {error}
        </p>
      )}

      <div className="flex items-end gap-2">
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              void ask(draft);
            }
          }}
          maxLength={4000}
          placeholder={`Ask anything about ${term}…`}
          aria-label={`Ask about ${term}`}
          className="flex-1 rounded-lg border border-border bg-surface px-3 py-1.5 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none"
        />
        <button
          type="button"
          onClick={() => void ask(draft)}
          disabled={busy || !draft.trim()}
          aria-label="Send"
          className="flex h-8 w-8 items-center justify-center rounded-lg bg-accent text-accent-fg hover:bg-accent-hover disabled:opacity-60"
        >
          {busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <Send className="h-4 w-4" />}
        </button>
      </div>
    </div>
  );
}
