# Phase 6 — Understand hub
Status: done
Goal: A prominent, self-extending learning hub spanning build and product domains, reachable three ways.
Depends on: Phase 2

## Tasks
- [x] **T1 Cards store + seed** — `learning_cards` seeded with ~40–60 curated cards plus cards for tech detected in attached repos. Done when: a fresh install has seed cards and attaching a repo adds detected-tech cards.
- [x] **T2 On-demand generation** — generate and cache a card for any term/concept without one (any domain). Done when: asking about an unseeded term creates and caches a card; re-asking reuses it. (`card_explain`: reuses a card with content, else `generate_card` → upsert; verified by `real_card_generation`.)
- [x] **T3 Hub pane + chat-to-card** — browse + "explain anything" box; capture chat explanations as cards. Done when: browse, ask-cold, and a chat explanation all yield retrievable cards. (`UnderstandHub` covers browse + ask-cold; `card_capture`/`capture()` is the chat-to-card mechanism — unit-tested. The chat UI that calls it lands in Phase 8.)
- [x] **T4 Why? everywhere** — Why?/Explain on every decision and stack choice, surfacing rationale and expanding to a card. Done when: clicking Why? on a decision and a stack choice shows an explanation and links a card. (`WhyExplain` on PlanPane decisions + stack.)

## Watch for (this phase)
- Cards span build AND product domains (business, design, UX), not just tech.
- Never dead-end: any unknown term must offer generation. This pane is where most of the user's time goes — make it fast and prominent.
