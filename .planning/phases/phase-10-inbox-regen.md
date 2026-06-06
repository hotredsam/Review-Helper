# Phase 10 — Feature inbox & plan regeneration
Status: not started
Goal: Capture features (text + audio stub), triage them into the plan, and regenerate on demand with an audit trail.
Depends on: Phase 4

## Tasks
- [ ] **T1 Feature inbox + audio stub** — text capture → `features`; a mic button wired to the stub transcription interface (clear TODO). Done when: text features land in the inbox and the mic button calls the stub and shows its placeholder.
- [ ] **T2 Update plan (merge)** — incremental merge weaving approved answers in and triaging the inbox (dedupe, map to phases, flag conflicts), preserving prior phases and status. Done when: updating yields a new plan version that keeps completed phases and incorporates the items, and the counter resets.
- [ ] **T3 Rebuild + audit log** — "Rebuild plan" (warned) and an audit log mapping source → plan version. Done when: rebuild regenerates and the audit log shows the source→version mapping for an update.

## Watch for (this phase)
- The merge MUST preserve per-phase completion status — losing it reintroduces the "restart at Phase 1" bug.
- Soft nudge at ~10 pending; never auto-run a regeneration.
