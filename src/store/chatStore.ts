import { create } from "zustand";
import {
  chatSend,
  chatNew,
  chatTranscripts,
  chatMessages,
  chatDelete,
  onChatEvent,
  type ChatEvent,
  type TranscriptMeta,
} from "../api/chat";
import { suggestionsList, type Suggestion } from "../api/suggestions";
import { useUiStore } from "./uiStore";

export interface Message {
  role: "user" | "assistant";
  text: string;
  streaming?: boolean;
}

type Status = "idle" | "streaming" | "error";

/**
 * Transcripts persist (v3). State is split by axis: transcripts + the active
 * transcript are per-project; messages are keyed by transcript id; status/error/
 * pending are per-project. Events route by transcript_id so the right chat gets
 * its tokens even if the user switched chats.
 */
interface ChatStore {
  transcripts: Record<number, TranscriptMeta[]>;
  activeId: Record<number, number | null>;
  messages: Record<number, Message[]>;
  status: Record<number, Status>;
  error: Record<number, string | null>;
  pending: Record<number, Suggestion[]>;
  loadProject: (project: number) => Promise<void>;
  openTranscript: (project: number, transcriptId: number) => Promise<void>;
  newChat: (project: number) => Promise<void>;
  removeTranscript: (project: number, transcriptId: number) => Promise<void>;
  send: (project: number, message: string) => Promise<void>;
  loadPending: (project: number) => Promise<void>;
}

const toMsgs = (rows: { role: "user" | "assistant"; content: string }[]): Message[] =>
  rows.map((r) => ({ role: r.role, text: r.content }));

export const useChatStore = create<ChatStore>((set, get) => ({
  transcripts: {},
  activeId: {},
  messages: {},
  status: {},
  error: {},
  pending: {},

  loadProject: async (project) => {
    try {
      const list = await chatTranscripts(project);
      // Already initialized: just refresh the rail.
      if (get().activeId[project] != null) {
        set((s) => ({ transcripts: { ...s.transcripts, [project]: list } }));
        return;
      }
      if (list.length === 0) {
        const id = await chatNew(project);
        const meta: TranscriptMeta = { id, title: null, updated_at: "", message_count: 0 };
        set((s) => ({
          transcripts: { ...s.transcripts, [project]: [meta] },
          activeId: { ...s.activeId, [project]: id },
          messages: { ...s.messages, [id]: [] },
        }));
      } else {
        const id = list[0].id;
        const msgs = toMsgs(await chatMessages(id));
        set((s) => ({
          transcripts: { ...s.transcripts, [project]: list },
          activeId: { ...s.activeId, [project]: id },
          messages: { ...s.messages, [id]: msgs },
        }));
      }
    } catch (e) {
      set((s) => ({ error: { ...s.error, [project]: String(e) } }));
    }
  },

  openTranscript: async (project, transcriptId) => {
    set((s) => ({ activeId: { ...s.activeId, [project]: transcriptId } }));
    if (get().messages[transcriptId] === undefined) {
      try {
        const msgs = toMsgs(await chatMessages(transcriptId));
        set((s) => ({ messages: { ...s.messages, [transcriptId]: msgs } }));
      } catch (e) {
        set((s) => ({ error: { ...s.error, [project]: String(e) } }));
      }
    }
  },

  newChat: async (project) => {
    try {
      const id = await chatNew(project);
      const meta: TranscriptMeta = { id, title: null, updated_at: "", message_count: 0 };
      set((s) => ({
        transcripts: { ...s.transcripts, [project]: [meta, ...(s.transcripts[project] ?? [])] },
        activeId: { ...s.activeId, [project]: id },
        messages: { ...s.messages, [id]: [] },
      }));
    } catch (e) {
      set((s) => ({ error: { ...s.error, [project]: String(e) } }));
    }
  },

  removeTranscript: async (project, transcriptId) => {
    try {
      await chatDelete(transcriptId);
    } catch (e) {
      // The delete didn't persist. Removing the row anyway would lie — it
      // reappears on the next launch. Keep it and say what happened.
      useUiStore.getState().setNotice(`Couldn't delete chat: ${String(e)}`);
      return;
    }
    const wasActive = get().activeId[project] === transcriptId;
    set((s) => ({
      transcripts: { ...s.transcripts, [project]: (s.transcripts[project] ?? []).filter((t) => t.id !== transcriptId) },
    }));
    if (wasActive) {
      set((s) => ({ activeId: { ...s.activeId, [project]: null } }));
      const remaining = get().transcripts[project] ?? [];
      if (remaining.length > 0) await get().openTranscript(project, remaining[0].id);
      else await get().newChat(project);
    }
  },

  send: async (project, message) => {
    const msg = message.trim();
    if (!msg || get().status[project] === "streaming") return;
    let tid = get().activeId[project];
    if (tid == null) {
      await get().newChat(project);
      tid = get().activeId[project];
      if (tid == null) return;
    }
    const transcriptId = tid;
    set((s) => ({
      messages: {
        ...s.messages,
        [transcriptId]: [
          ...(s.messages[transcriptId] ?? []),
          { role: "user", text: msg },
          { role: "assistant", text: "", streaming: true },
        ],
      },
      status: { ...s.status, [project]: "streaming" },
      error: { ...s.error, [project]: null },
    }));
    try {
      await chatSend(project, transcriptId, msg);
    } catch (e) {
      patchLastAssistant(transcriptId, (m) => ({ ...m, streaming: false }));
      set((s) => ({ status: { ...s.status, [project]: "error" }, error: { ...s.error, [project]: String(e) } }));
    }
  },

  loadPending: async (project) => {
    try {
      const ps = await suggestionsList(project, "pending");
      set((s) => ({ pending: { ...s.pending, [project]: ps } }));
    } catch {
      // non-fatal: the chat still works without the proposals panel
    }
  },
}));

function patchLastAssistant(transcriptId: number, fn: (m: Message) => Message) {
  useChatStore.setState((s) => {
    const msgs = (s.messages[transcriptId] ?? []).slice();
    for (let i = msgs.length - 1; i >= 0; i--) {
      if (msgs[i].role === "assistant") {
        msgs[i] = fn(msgs[i]);
        break;
      }
    }
    return { messages: { ...s.messages, [transcriptId]: msgs } };
  });
}

function handle(e: ChatEvent) {
  switch (e.type) {
    case "started":
    case "tool":
      break;
    case "token":
      patchLastAssistant(e.transcript_id, (m) => ({ ...m, text: m.text + e.text }));
      break;
    case "done":
      patchLastAssistant(e.transcript_id, (m) => ({ ...m, text: e.reply, streaming: false }));
      useChatStore.setState((s) => ({ status: { ...s.status, [e.project_id]: "idle" } }));
      // refresh the rail (titles + counts) and pending proposals
      void chatTranscripts(e.project_id)
        .then((list) => useChatStore.setState((s) => ({ transcripts: { ...s.transcripts, [e.project_id]: list } })))
        .catch(() => {});
      if (e.suggestions > 0) void useChatStore.getState().loadPending(e.project_id);
      break;
    case "failed":
      patchLastAssistant(e.transcript_id, (m) => ({ ...m, streaming: false }));
      useChatStore.setState((s) => ({
        status: { ...s.status, [e.project_id]: "error" },
        error: { ...s.error, [e.project_id]: e.detail },
      }));
      break;
    case "stopped":
      // Keep the partial text (it's persisted backend-side); a stop before any
      // token just drops the empty placeholder bubble.
      if (e.partial.trim() === "") {
        useChatStore.setState((s) => {
          const msgs = (s.messages[e.transcript_id] ?? []).slice();
          const last = msgs[msgs.length - 1];
          if (last?.role === "assistant" && last.text === "") msgs.pop();
          return { messages: { ...s.messages, [e.transcript_id]: msgs } };
        });
      } else {
        patchLastAssistant(e.transcript_id, (m) => ({ ...m, text: e.partial, streaming: false }));
      }
      useChatStore.setState((s) => ({ status: { ...s.status, [e.project_id]: "idle" } }));
      break;
  }
}

let wired = false;
export function ensureChatListener() {
  if (wired) return;
  wired = true;
  onChatEvent(handle).catch((e) => {
    wired = false;
    console.error("chat: failed to attach event listener", e);
  });
}
