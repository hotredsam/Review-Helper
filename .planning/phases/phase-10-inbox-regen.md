# Phase 10 — Feature inbox & plan regeneration
Status: done
Goal: Capture features (text + audio stub), triage them into the plan, and regenerate on demand with an audit trail.
Depends on: Phase 4

## Tasks
- [x] **T1 Feature inbox + audio stub** — text capture → `features`; a mic button wired to the stub transcription interface (clear TODO). Done when: text features land in the inbox and the mic button calls the stub and shows its placeholder. (`features` module add/list/set_status/pending_count + `transcribe_audio_stub`; `InboxPane` capture + mic-stub + queue + soft nudge.)
- [x] **T2 Update plan (merge)** — incremental merge weaving approved answers in and triaging the inbox (dedupe, map to phases, flag conflicts), preserving prior phases and status. Done when: updating yields a new plan version that keeps completed phases and incorporates the items, and the counter resets. (`MERGE_SYSTEM` + `merge_user`; `store::carry_status` carries phase status by marker + task status by title — tested; `update_plan` marks incorporated features in_plan.)
- [x] **T3 Rebuild + audit log** — "Rebuild plan" (warned) and an audit log mapping source → plan version. Done when: rebuild regenerates and the audit log shows the source→version mapping for an update. (`rebuild_plan` (fresh, no carry); `audit` module in `settings` kv records analyze/kickoff/update/rebuild → version; PlanPane warned Rebuild + History list.)

## Watch for (this phase)
- The merge MUST preserve per-phase completion status — losing it reintroduces the "restart at Phase 1" bug.
- Soft nudge at ~10 pending; never auto-run a regeneration.
