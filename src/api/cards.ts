import { invoke } from "@tauri-apps/api/core";

export interface Card {
  id: number;
  term: string;
  domain: string | null;
  what_md: string | null;
  when_md: string | null;
  why_md: string | null;
  source: string | null;
}

export function cardsList(): Promise<Card[]> {
  return invoke<Card[]>("cards_list");
}

export function cardGet(term: string): Promise<Card | null> {
  return invoke<Card | null>("card_get", { term });
}

/** Return a card, generating + caching it if it has no content yet. */
export function cardExplain(term: string): Promise<Card> {
  return invoke<Card>("card_explain", { term });
}

/** Capture an explanation (e.g. from a chat answer) as a retrievable card. */
export function cardCapture(term: string, explanation: string, domain?: string): Promise<Card> {
  return invoke<Card>("card_capture", { term, explanation, domain: domain ?? null });
}
