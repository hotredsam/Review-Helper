import { create } from "zustand";
import { chatSend, onChatEvent, type ChatEvent } from "../api/chat";
import { suggestionsList, type Suggestion } from "../api/suggestions";

export interface Message {
  role: "user" | "assistant";
  text: string;
  streaming?: boolean;
}

type Status = "idle" | "streaming" | "error";

interface ChatStore {
  messages: Record<number, Message[]>;
  session: Record<number, string | null>;
  status: Record<number, Status>;
  error: Record<number, string | null>;
  lastSuggestions: Record<number, number>;
  pending: Record<number, Suggestion[]>;
  send: (id: number, message: string) => Promise<void>;
  loadPending: (id: number) => Promise<void>;
}

export const useChatStore = create<ChatStore>((set, get) => ({
  messages: {},
  session: {},
  status: {},
  error: {},
  lastSuggestions: {},
  pending: {},

  loadPending: async (id) => {
    try {
      const ps = await suggestionsList(id, "pending");
      set((s) => ({ pending: { ...s.pending, [id]: ps } }));
    } catch {
      // non-fatal: the chat still works without the proposals panel
    }
  },

  send: async (id, message) => {
    const msg = message.trim();
    if (!msg || get().status[id] === "streaming") return;
    set((s) => ({
      messages: {
        ...s.messages,
        [id]: [
          ...(s.messages[id] ?? []),
          { role: "user", text: msg },
          { role: "assistant", text: "", streaming: true },
        ],
      },
      status: { ...s.status, [id]: "streaming" },
      error: { ...s.error, [id]: null },
    }));
    try {
      await chatSend(id, msg, get().session[id] ?? null);
    } catch (e) {
      patchLastAssistant(id, (m) => ({ ...m, streaming: false }));
      set((s) => ({ status: { ...s.status, [id]: "error" }, error: { ...s.error, [id]: String(e) } }));
    }
  },
}));

function patchLastAssistant(id: number, fn: (m: Message) => Message) {
  useChatStore.setState((s) => {
    const msgs = (s.messages[id] ?? []).slice();
    for (let i = msgs.length - 1; i >= 0; i--) {
      if (msgs[i].role === "assistant") {
        msgs[i] = fn(msgs[i]);
        break;
      }
    }
    return { messages: { ...s.messages, [id]: msgs } };
  });
}

function handle(e: ChatEvent) {
  const id = e.project_id;
  switch (e.type) {
    case "started":
      break; // the streaming assistant placeholder is added in send()
    case "token":
      patchLastAssistant(id, (m) => ({ ...m, text: m.text + e.text }));
      break;
    case "done":
      patchLastAssistant(id, (m) => ({ ...m, text: e.reply, streaming: false }));
      useChatStore.setState((s) => ({
        session: { ...s.session, [id]: e.session_id },
        status: { ...s.status, [id]: "idle" },
        lastSuggestions: { ...s.lastSuggestions, [id]: e.suggestions },
      }));
      if (e.suggestions > 0) void useChatStore.getState().loadPending(id);
      break;
    case "failed":
      patchLastAssistant(id, (m) => ({ ...m, streaming: false }));
      useChatStore.setState((s) => ({
        status: { ...s.status, [id]: "error" },
        error: { ...s.error, [id]: e.detail },
      }));
      break;
  }
}

let wired = false;
export function ensureChatListener() {
  if (wired) return;
  wired = true;
  onChatEvent(handle).catch(() => {
    wired = false;
  });
}
