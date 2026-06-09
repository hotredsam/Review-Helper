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

/** Return a card, generating + caching it if it has no content yet. `projectId`
 *  (when on a project) associates the card with it for the "This project" filter. */
export function cardExplain(term: string, projectId?: number | null): Promise<Card> {
  return invoke<Card>("card_explain", { term, projectId: projectId ?? null });
}

/** Capture an explanation (e.g. from a chat answer) as a retrievable card. */
export function cardCapture(
  term: string,
  explanation: string,
  domain?: string,
  projectId?: number | null,
): Promise<Card> {
  return invoke<Card>("card_capture", { term, explanation, domain: domain ?? null, projectId: projectId ?? null });
}

export interface CardMsg {
  role: "user" | "assistant";
  content: string;
}

/** Terms of cards that belong to this project (the "This project" filter). */
export function cardProjectTerms(projectId: number): Promise<string[]> {
  return invoke<string[]>("card_project_terms", { projectId });
}

/** Fix the spelling/grammar of a typed term before explaining + carding it. */
export function cardCleanTerm(term: string): Promise<string> {
  return invoke<string>("card_clean_term", { term });
}

/** 5–10 starter questions for a card (cached after the first generation). */
export function cardPremadeQuestions(term: string): Promise<string[]> {
  return invoke<string[]>("card_premade_questions", { term });
}

export function cardChatHistory(projectId: number, term: string): Promise<CardMsg[]> {
  return invoke<CardMsg[]>("card_chat_history", { projectId, term });
}

/** Send a message in a card's inline chat; returns the assistant reply. */
export function cardChatSend(projectId: number, term: string, message: string): Promise<string> {
  return invoke<string>("card_chat_send", { projectId, term, message });
}
