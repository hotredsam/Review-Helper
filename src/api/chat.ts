import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface TranscriptMeta {
  id: number;
  title: string | null;
  updated_at: string;
  message_count: number;
}

export interface StoredMessage {
  role: "user" | "assistant";
  content: string;
}

export type ChatEvent =
  | { type: "started"; project_id: number; transcript_id: number }
  | { type: "token"; project_id: number; transcript_id: number; text: string }
  | { type: "tool"; project_id: number; transcript_id: number; name: string }
  | { type: "done"; project_id: number; transcript_id: number; reply: string; suggestions: number }
  | { type: "failed"; project_id: number; transcript_id: number; detail: string };

/** Send a turn in a specific transcript (the full chat history is injected backend-side). */
export function chatSend(id: number, transcriptId: number, message: string): Promise<void> {
  return invoke("chat_send", { projectId: id, transcriptId, message });
}

/** Start a fresh chat; returns the new transcript id. */
export function chatNew(id: number): Promise<number> {
  return invoke<number>("chat_new", { projectId: id });
}

export function chatTranscripts(id: number): Promise<TranscriptMeta[]> {
  return invoke<TranscriptMeta[]>("chat_transcripts", { projectId: id });
}

export function chatMessages(transcriptId: number): Promise<StoredMessage[]> {
  return invoke<StoredMessage[]>("chat_messages", { transcriptId });
}

export function chatDelete(transcriptId: number): Promise<void> {
  return invoke("chat_delete", { transcriptId });
}

export function onChatEvent(cb: (e: ChatEvent) => void): Promise<UnlistenFn> {
  return listen<ChatEvent>("chat-event", (e) => cb(e.payload));
}
