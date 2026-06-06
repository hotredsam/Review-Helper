import { create } from "zustand";
import {
  runModel,
  onModelEvent,
  type ModelEvent,
  type UnavailableReason,
} from "../api/model";

export interface Turn {
  role: "user" | "assistant";
  text: string;
}

interface ModelState {
  turns: Turn[];
  sessionId: string | null;
  streaming: boolean;
  error: string | null;
  unavailable: { reason: UnavailableReason; detail: string } | null;
  tools: string[];
  send: (prompt: string) => Promise<void>;
  reset: () => void;
}

/**
 * Drives the model console. A single global `model-event` listener (wired once
 * via `ensureModelListener`) feeds streamed deltas into the in-progress turn;
 * follow-up sends pass the captured `sessionId` so Claude resumes the session.
 */
export const useModelStore = create<ModelState>((set, get) => ({
  turns: [],
  sessionId: null,
  streaming: false,
  error: null,
  unavailable: null,
  tools: [],

  send: async (prompt) => {
    const text = prompt.trim();
    if (!text || get().streaming) return;
    set((s) => ({
      turns: [...s.turns, { role: "user", text }, { role: "assistant", text: "" }],
      streaming: true,
      error: null,
      unavailable: null,
      tools: [],
    }));
    try {
      await runModel(text, get().sessionId);
    } catch (e) {
      set({ streaming: false, error: String(e) });
    }
  },

  reset: () =>
    set({
      turns: [],
      sessionId: null,
      streaming: false,
      error: null,
      unavailable: null,
      tools: [],
    }),
}));

function appendAssistant(text: string) {
  useModelStore.setState((s) => {
    const turns = s.turns.slice();
    const last = turns[turns.length - 1];
    if (last && last.role === "assistant") {
      turns[turns.length - 1] = { ...last, text: last.text + text };
    }
    return { turns };
  });
}

function handleEvent(e: ModelEvent) {
  switch (e.type) {
    case "started":
      if (e.session_id) useModelStore.setState({ sessionId: e.session_id });
      break;
    case "assistant_text":
      appendAssistant(e.text);
      break;
    case "tool_use":
      useModelStore.setState((s) => ({ tools: [...s.tools, e.name] }));
      break;
    case "notice":
      useModelStore.setState((s) => ({ tools: [...s.tools, e.message] }));
      break;
    case "completed":
      useModelStore.setState((s) => {
        // Backfill the full text if no deltas streamed (e.g. a very short reply).
        const turns = s.turns.slice();
        const last = turns[turns.length - 1];
        if (last && last.role === "assistant" && last.text.length === 0) {
          turns[turns.length - 1] = { ...last, text: e.text };
        }
        return { turns, streaming: false, sessionId: e.session_id ?? s.sessionId };
      });
      break;
    case "unavailable":
      useModelStore.setState({
        streaming: false,
        unavailable: { reason: e.reason, detail: e.detail },
      });
      break;
    case "failed":
      useModelStore.setState({ streaming: false, error: e.detail });
      break;
  }
}

let wired = false;
/** Wire the single global model-event listener (idempotent). If wiring fails
 *  (e.g. the event bridge isn't ready), reset so a later mount can retry. */
export function ensureModelListener() {
  if (wired) return;
  wired = true;
  onModelEvent(handleEvent).catch(() => {
    wired = false;
  });
}
