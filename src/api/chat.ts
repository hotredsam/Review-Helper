import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type ChatEvent =
  | { type: "started"; project_id: number }
  | { type: "token"; project_id: number; text: string }
  | { type: "tool"; project_id: number; name: string }
  | {
      type: "done";
      project_id: number;
      session_id: string | null;
      reply: string;
      suggestions: number;
    }
  | { type: "failed"; project_id: number; detail: string };

/** Send a chat turn. `sessionId` resumes the prior turn (null on the first). */
export function chatSend(id: number, message: string, sessionId: string | null): Promise<void> {
  return invoke("chat_send", { projectId: id, message, sessionId });
}

export function onChatEvent(cb: (e: ChatEvent) => void): Promise<UnlistenFn> {
  return listen<ChatEvent>("chat-event", (e) => cb(e.payload));
}
