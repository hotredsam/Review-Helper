# Phase 16 — Unfreeze & control
Status: planned
Goal: Nothing blocks the UI thread; every long-running operation can be stopped and times out cleanly instead of bricking a feature until restart.
Depends on: Phase 15. Findings cited as A# live in `.planning/AUDIT-2026-06-09.md`.

## Tasks
- [ ] **T1 Kill-handle + timeout in ModelProvider** — `run()` gains a cancellation handle and a hard timeout (constant, ~300s); on timeout or cancel the child `claude` process is killed and a clean `failed` event is emitted with the reason. One shared mechanism — every feature inherits it through the single model entry point. Done when: a Rust test against a fake hanging child asserts kill + clean error at the timeout.
- [ ] **T2 Drain stderr concurrently** — stderr gets its own reader thread during streaming so a chatty child can never fill the pipe and deadlock the provider (A24/A36). Done when: a Rust test with a stderr-flooding child completes without hanging.
- [ ] **T3 Move learning/cards/assess off the main thread** — their model calls move to the spawned-thread + emitted-events pattern chat already uses; panes get progress and failure events instead of a frozen window (A2/A10). Done when: tests assert the event sequence, and a manual check confirms the UI stays interactive during a generation.
- [ ] **T4 Clone/fetch timeout** — `project_clone`'s git operations move off the sync command path and get a ~120s network timeout with a clean error (A5). Done when: a stalled-network simulation errors cleanly while the app stays responsive.
- [ ] **T5 Release the DB mutex before network I/O** — `sync_push_planning` and `sync_main_preview` read what they need, drop the lock, then talk to GitHub — the pattern `apply_main_sync` already follows (A3). Done when: the lock is provably not held across GitHub calls and existing sync tests stay green.
- [ ] **T6 Stop and Cancel buttons** — chat and tutor get a Stop button while streaming (kept partial text stays in the transcript as-is); learning/cards/assess/plan generations get Cancel (partial output discarded). Done when: clicking Stop/Cancel kills the child, returns the pane to idle, and a test covers each path.
- [ ] **T7 Fix LearningGate keying** — the gate keys consistently on `subject_id` everywhere (today it mixes `module_id` and `subject_id`, so the documented per-subject serialization doesn't hold), and a poisoned map recovers instead of panicking (A25). Done when: Rust tests cover both.
- [ ] **Tend Phase verification** — start a long generation, switch panes (no freeze), cancel it; send a chat, stop mid-stream; pull the network mid-clone. Done when: all behave, suites green.

## Watch for (this phase)
- Killing the child mid-stream must not corrupt persisted state — only persist completed turns (the orphaned-turn fixes land in Phase 17; don't pre-solve them here, just don't make them worse).
- Timeout constants live in one place (externalize-what-changes rule), not scattered per call site.
