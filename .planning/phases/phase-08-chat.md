# Phase 8 — Two-way chat & structured proposals
Status: done
Goal: A grounded, two-way chat that turns inferred updates into pending suggestions.
Depends on: Phase 2, Phase 4

## Tasks
- [x] **T1 Chat with context** — chat pane on the Claude adapter, context bundle injected each turn, multi-turn resume. Done when: a multi-turn chat references project state correctly and resumes across turns. (`chat_send` injects `ProjectContext` per turn + resumes via `session_id`; streams tokens; routes via `provider_for`. `ChatPane`/`chatStore` thread the session across turns — store test verifies.)
- [x] **T2 Structured proposals** — instruct the model to emit inferred updates as tagged blocks; parse into `suggestions`. Done when: a chat implying a decision and a feature produces matching pending suggestions, and a chat with none produces none. (`CHAT_SYSTEM` block protocol → `parse_suggestions` (robust: drops malformed/unknown, strips blocks) → `suggestions::save` pending. Pending panel in chat; approve UI is Phase 9.)

## Watch for (this phase)
- The parser must be robust to malformed or absent blocks — don't crash, don't invent.
- Nothing reaches the record from chat except through pending suggestions the user approves.
