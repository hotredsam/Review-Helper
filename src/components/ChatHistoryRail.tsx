import { useState } from "react";
import { Plus, Trash2, MessagesSquare } from "lucide-react";
import { useChatStore } from "../store/chatStore";
import type { TranscriptMeta } from "../api/chat";
import { ConfirmDialog } from "./ConfirmDialog";

/** Right-hand rail listing a project's past chats, with a New chat button and
 *  per-chat delete (Modal-confirmed — deletion is permanent). */
export function ChatHistoryRail({ project }: { project: number }) {
  const transcriptsRaw = useChatStore((s) => s.transcripts[project]);
  const transcripts = transcriptsRaw ?? [];
  const activeId = useChatStore((s) => s.activeId[project]);
  const open = useChatStore((s) => s.openTranscript);
  const newChat = useChatStore((s) => s.newChat);
  const remove = useChatStore((s) => s.removeTranscript);
  const [confirmDelete, setConfirmDelete] = useState<TranscriptMeta | null>(null);

  return (
    <aside className="flex w-56 shrink-0 flex-col border-l border-border bg-surface">
      <div className="flex items-center justify-between border-b border-border px-3 py-2.5">
        <h3 className="text-xs font-semibold uppercase tracking-wide text-fg-subtle">Chats</h3>
        <button
          type="button"
          onClick={() => void newChat(project)}
          aria-label="New chat"
          className="flex items-center gap-1 rounded-md bg-accent px-2 py-0.5 text-xs font-medium text-accent-fg hover:bg-accent-hover"
        >
          <Plus className="h-3 w-3" /> New
        </button>
      </div>
      <ul className="flex-1 overflow-auto p-2">
        {transcripts.length === 0 && <li className="px-2 py-2 text-xs text-fg-subtle">No chats yet.</li>}
        {transcripts.map((t) => {
          const active = t.id === activeId;
          return (
            <li key={t.id} className="group flex items-center">
              <button
                type="button"
                onClick={() => void open(project, t.id)}
                title={t.title ?? "New chat"}
                aria-current={active ? "true" : undefined}
                className={
                  "flex min-w-0 flex-1 items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm " +
                  (active ? "bg-accent/10 text-fg" : "text-fg-muted hover:bg-surface-2 hover:text-fg")
                }
              >
                <MessagesSquare className="h-3.5 w-3.5 shrink-0 text-fg-subtle" />
                <span className="min-w-0 flex-1 truncate">{t.title ?? "New chat"}</span>
              </button>
              <button
                type="button"
                onClick={() => setConfirmDelete(t)}
                aria-label="Delete chat"
                className="hidden shrink-0 rounded p-1 text-fg-subtle hover:text-danger group-hover:block"
              >
                <Trash2 className="h-3.5 w-3.5" />
              </button>
            </li>
          );
        })}
      </ul>

      <ConfirmDialog
        open={confirmDelete !== null}
        title="Delete this chat?"
        body={`"${confirmDelete?.title ?? "New chat"}" and all its messages are permanently deleted.`}
        confirmLabel="Delete chat"
        onConfirm={() => {
          if (confirmDelete) void remove(project, confirmDelete.id);
          setConfirmDelete(null);
        }}
        onCancel={() => setConfirmDelete(null)}
      />
    </aside>
  );
}
