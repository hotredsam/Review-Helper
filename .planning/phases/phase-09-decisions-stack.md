# Phase 9 — Decisions, suggestions & stack panes
Status: not started
Goal: A decisions record, an approval surface, and the five stack panes wired into the plan.
Depends on: Phase 8

## Tasks
- [ ] **T1 Pending suggestions** — surface with Approve and Approve all. Done when: approve writes the right row, approve-all clears the queue, and dismiss removes without writing.
- [ ] **T2 Decisions record** — ADR-style pane (topic, choice, rationale, alternatives, consequences, source, status) with supersede. Done when: decisions show all fields and superseding marks the old one while keeping history.
- [ ] **T3 Stack panes** — five panes (frontend, backend, database, deployment, pipes) with recommendation + 2–3 alternatives + rationale + Why? + card tap-through; pre-made stacks; apply-to-all; per-pane override. Done when: applying a pre-made stack fills all five, overriding one persists, and selections appear as decisions.

## Watch for (this phase)
- Scope each approval write to its own table; one approval must not mutate unrelated rows.
