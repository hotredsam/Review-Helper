import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Loader2, Send, MessagesSquare } from "lucide-react";
import { useChatStore, ensureChatListener, type Message } from "../store/chatStore";
import type { Project } from "../api/projects";

const EMPTY: Message[] = [];

/** Two-way chat: a grounded conversation that references project state and
 *  resumes across turns. Inferred updates surface as pending suggestions (T2). */
export function ChatPane({ project }: { project: Project }) {
  const id = project.id;
  // Raw selects + defaults outside the selector (avoids fresh-value render loops).
  const messagesRaw = useChatStore((s) => s.messages[id]);
  const messages = messagesRaw ?? EMPTY;
  const status = useChatStore((s) => s.status[id] ?? "idle");
  const error = useChatStore((s) => s.error[id]);
  const send = useChatStore((s) => s.send);

  const [draft, setDraft] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    ensureChatListener();
  }, []);
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [messages]);

  const streaming = status === "streaming";

  const submit = () => {
    const msg = draft.trim();
    if (!msg || streaming) return;
    void send(id, msg);
    setDraft("");
  };
  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  };

  return (
    <div className="mx-auto flex h-full max-w-3xl flex-col p-6">
      <div ref={scrollRef} className="flex-1 space-y-3 overflow-auto pb-4">
        {messages.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
            <MessagesSquare className="h-8 w-8 text-fg-subtle" />
            <p className="text-sm font-medium text-fg">Talk through your project</p>
            <p className="max-w-sm text-sm text-fg-muted">
              The chat knows your plan, decisions, and stack. Anything it infers becomes a pending
              suggestion you approve — nothing changes the record on its own.
            </p>
          </div>
        ) : (
          messages.map((m, i) => <Bubble key={i} message={m} />)
        )}
      </div>

      {error && (
        <p className="mb-2 text-sm text-danger" role="alert">
          {error}
        </p>
      )}

      <div className="flex items-end gap-2 border-t border-border pt-3">
        <textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={onKeyDown}
          rows={2}
          maxLength={20000}
          placeholder="Ask about your project, or think out loud…"
          aria-label="Chat message"
          className="flex-1 resize-none rounded-lg border border-border bg-surface px-3 py-2 text-sm text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none focus:ring-2 focus:ring-ring/40"
        />
        <button
          type="button"
          onClick={submit}
          disabled={streaming || !draft.trim()}
          aria-label="Send"
          className="flex h-10 w-10 items-center justify-center rounded-lg bg-accent text-accent-fg hover:bg-accent-hover disabled:opacity-60"
        >
          {streaming ? <Loader2 className="h-4 w-4 animate-spin" /> : <Send className="h-4 w-4" />}
        </button>
      </div>
    </div>
  );
}

function Bubble({ message }: { message: Message }) {
  const isUser = message.role === "user";
  return (
    <div className={isUser ? "flex justify-end" : "flex justify-start"}>
      <div
        className={
          "max-w-[80%] whitespace-pre-wrap rounded-2xl px-3 py-2 text-sm " +
          (isUser ? "bg-accent text-accent-fg" : "bg-surface-2 text-fg")
        }
      >
        {message.text}
        {message.streaming && message.text === "" && (
          <Loader2 className="h-4 w-4 animate-spin text-fg-subtle" />
        )}
      </div>
    </div>
  );
}
