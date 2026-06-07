import { useEffect, useRef, useState, type FormEvent } from "react";
import { Send, RotateCcw, Loader2 } from "lucide-react";
import { useModelStore, ensureModelListener } from "../store/modelStore";

/**
 * Phase 2 temp panel: send a prompt to Claude (`claude -p`) and watch the reply
 * stream in token-by-token; follow-ups resume the same session. Lives in
 * Settings for now; the real chat is a later phase.
 */
export function ModelConsole() {
  const { turns, streaming, error, unavailable, sessionId, tools, send, reset } =
    useModelStore();
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    ensureModelListener();
  }, []);

  useEffect(() => {
    const el = scrollRef.current;
    el?.scrollTo?.({ top: el.scrollHeight });
  }, [turns]);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (!input.trim() || streaming) return;
    void send(input);
    setInput("");
  };

  return (
    <div className="flex flex-col gap-3">
      <div
        ref={scrollRef}
        className="h-64 overflow-auto rounded-lg border border-border bg-surface p-3 text-sm"
      >
        {turns.length === 0 && !unavailable && !error && (
          <p className="text-fg-subtle">
            Send a prompt to test the Claude connection — replies stream in live.
          </p>
        )}
        {turns.map((t, i) => (
          <div key={i} className="mb-3">
            <p className="mb-0.5 text-xs font-medium uppercase tracking-wide text-fg-subtle">
              {t.role === "user" ? "You" : "Claude"}
            </p>
            <p className="whitespace-pre-wrap text-fg">
              {t.text || (streaming && t.role === "assistant" ? "…" : "")}
            </p>
          </div>
        ))}
        {tools.length > 0 && (
          <p className="text-xs text-fg-subtle">used: {tools.join(", ")}</p>
        )}
        {unavailable && (
          <p className="text-danger" role="alert">
            Unavailable ({unavailable.reason.replace(/_/g, " ")}): {unavailable.detail}
          </p>
        )}
        {error && (
          <p className="text-danger" role="alert">
            Error: {error}
          </p>
        )}
      </div>

      {sessionId && (
        <p className="text-xs text-fg-subtle">
          Session {sessionId.slice(0, 8)}… {streaming ? "· streaming" : "· follow-ups resume this session"}
        </p>
      )}

      <form onSubmit={submit} className="flex gap-2">
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          aria-label="Message to Claude"
          placeholder={sessionId ? "Continue the conversation…" : "Ask Claude something…"}
          className="flex-1 rounded-lg border border-border bg-surface px-3 py-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none focus:ring-2 focus:ring-ring/40"
        />
        <button
          type="submit"
          disabled={streaming || !input.trim()}
          className="flex items-center gap-1.5 rounded-lg bg-accent px-3 py-2 text-sm font-medium text-accent-fg hover:bg-accent-hover disabled:opacity-60"
        >
          {streaming ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Send className="h-4 w-4" />
          )}
          {streaming ? "Streaming" : "Send"}
        </button>
        {turns.length > 0 && (
          <button
            type="button"
            onClick={reset}
            title="New session"
            aria-label="New session"
            className="rounded-lg border border-border px-3 py-2 text-sm text-fg-muted hover:bg-surface-2"
          >
            <RotateCcw className="h-4 w-4" />
          </button>
        )}
      </form>
    </div>
  );
}
