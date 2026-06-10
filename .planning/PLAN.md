# Review Helper — Build Plan (index)

This is the index. Each phase lives in its own file under `.planning/phases/` so Claude Code loads only the phase it's working on, not the whole plan. Phase status is tracked here and in each phase file.

## Current state

**Phases 1–14 and the A–H overhaul are done. Phases 15–19 (the audit overhaul) are in progress, then 20–21 (adaptive profile, study RAG).** They come from the 2026-06-09 verified bug audit — 52 confirmed findings, full evidence in `.planning/AUDIT-2026-06-09.md`. Exit bar for 15–19: every fixed finding gets a regression test that fails on the old code, and the Phase 15 IPC contract suite stays green. The current phase is the first row below not marked `done`. Before any phase, run `/start-phase` (or the `phase-check` skill) to confirm prior phases are actually finished. Work one task at a time; stop after each for review. Tick a task's checkbox when its "Done when" check passes and you've committed; set a phase to `done` only after its phase-end verification passes. **Never rebuild a phase marked `done`.**

## How to execute

1. Open the current phase file and `CLAUDE.md`. Do not load the other phase files.
2. Do one task; run its check; commit (the commit hook runs the secrets scan); tick the box.
3. At the phase end, run the phase verification, set Status to `done`, update the table below, and stop for review.

## Locked decisions

- **Language/runtime:** Rust (Tauri backend) + TypeScript (frontend). Native, self-contained, plays to existing Rust skill.
- **Frontend:** React + Tailwind, lightweight store (Zustand). Largest ecosystem and AI support.
- **Database:** embedded SQLite — use the tested `.planning/schema.sql` as-is.
- **Model:** Claude Code via `claude -p` (stream-json) behind a `ModelProvider` interface; local + credit slots stubbed in Settings.
- **Code access:** the model is read-only (read/search tools only); Review Helper performs all writes/commits via the GitHub API.
- **GitHub:** OAuth device flow; shallow-clone cache; plan→issues one-way sync.
- **Packaging:** signed + notarized local `.app` (~90% App-Store-grade feel, no store submission in v1).

## Out of scope (v1)

- Runtime LLM-generated UI (charts are a fixed, contextually-surfaced set).
- App Store submission.
- Two-way GitHub-issue sync.
- The "learning mode" beyond vibecoding (stubbed only — Phase 14).
- Any path that lets the model write or commit the user's source.

## Phases

| # | File | Status |
|---|------|--------|
| 1 | `phases/phase-01-shell.md` — project scaffold & app shell | done |
| 2 | `phases/phase-02-model-provider.md` — model provider & Claude availability | done |
| 3 | `phases/phase-03-projects-github.md` — projects & GitHub connect | done |
| 4 | `phases/phase-04-analysis-coldstart.md` — repo analysis & cold start | done |
| 5 | `phases/phase-05-assessment.md` — assessment engine & State pane | done |
| 6 | `phases/phase-06-understand-hub.md` — the Understand hub | done |
| 7 | `phases/phase-07-grill.md` — grill-me cards & detail coverage | done |
| 8 | `phases/phase-08-chat.md` — two-way chat & structured proposals | done |
| 9 | `phases/phase-09-decisions-stack.md` — decisions, suggestions & stack panes | done |
| 10 | `phases/phase-10-inbox-regen.md` — feature inbox & plan regeneration | done |
| 11 | `phases/phase-11-github-sync.md` — GitHub sync out | done |
| 12 | `phases/phase-12-viz-firstrun.md` — visualization, first-run & polish | done |
| 13 | `phases/phase-13-hardening.md` — production hardening | done |
| 14 | `phases/phase-14-coming-soon.md` — coming-soon learning mode (stub) | done |
| 15 | `phases/phase-15-destructive-safety.md` — destructive-action safety + IPC contract suite | done |
| 16 | `phases/phase-16-unfreeze-control.md` — unfreeze & control (async, timeouts, Stop) | done |
| 17 | `phases/phase-17-settings-truth.md` — settings truth & data integrity (provider, FSRS, transactions) | planned |
| 18 | `phases/phase-18-polish-sweep.md` — polish sweep (races, UX, a11y, hygiene) | planned |
| 19 | `phases/phase-19-voice-ingest.md` — live local Whisper mic + chunked document ingest | planned |
| 20 | `phases/phase-20-adaptive-profile.md` — adaptive self-learning profile (MD files + cheap reflection) | planned |
| 21 | `phases/phase-21-study-rag.md` — study-material RAG (hybrid retrieval, citations, labeled web fallback) | planned |

Phases 1–13 build to production-ready; 14 is a stub. 15–19 fix the 2026-06-09 audit findings; 20–21 are researched feature phases (adaptive profile, study RAG). Order is by dependency — see each file's `Depends on`.

## Open questions

- ~~**Audio transcription provider**~~ — resolved: Phase 19 wires live local Whisper (`large-v3-turbo-q5_0` via whisper-rs).
- **Seed card list contents** — owner: decide during Phase 6 Task 1. The ~40–60 entries are tunable as you see which domains you lean on.
- **App name** — owner: you, anytime. `Review Helper` matches the repo; not blocking.
- **Real Ollama provider** — owner: you. Phase 17 wires `provider_for()` + capability-gates the Local stub; making Local a real Ollama chat provider is a candidate Phase 22 with its own quality evals (Phase 21 already brings Ollama in for embeddings only).
