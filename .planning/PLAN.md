# Review Helper — Build Plan (index)

This is the index. Each phase lives in its own file under `.planning/phases/` so Claude Code loads only the phase it's working on, not the whole plan. Phase status is tracked here and in each phase file.

## Current state

**Phases 1–4 are done; Phase 5 is current.** The current phase is the first row below not marked `done`. Before any phase, run `/start-phase` (or the `phase-check` skill) to confirm prior phases are actually finished. Work one task at a time; stop after each for review. Tick a task's checkbox when its "Done when" check passes and you've committed; set a phase to `done` only after its phase-end verification passes. **Never rebuild a phase marked `done`.**

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
| 5 | `phases/phase-05-assessment.md` — assessment engine & State pane | not started |
| 6 | `phases/phase-06-understand-hub.md` — the Understand hub | not started |
| 7 | `phases/phase-07-grill.md` — grill-me cards & detail coverage | not started |
| 8 | `phases/phase-08-chat.md` — two-way chat & structured proposals | not started |
| 9 | `phases/phase-09-decisions-stack.md` — decisions, suggestions & stack panes | not started |
| 10 | `phases/phase-10-inbox-regen.md` — feature inbox & plan regeneration | not started |
| 11 | `phases/phase-11-github-sync.md` — GitHub sync out | not started |
| 12 | `phases/phase-12-viz-firstrun.md` — visualization, first-run & polish | not started |
| 13 | `phases/phase-13-hardening.md` — production hardening | not started |
| 14 | `phases/phase-14-coming-soon.md` — coming-soon learning mode (stub) | not started |

Phases 1–13 build to production-ready; 14 is a stub. Order is by dependency — see each file's `Depends on`.

## Open questions

- **Audio transcription provider** — owner: you, during Phase 10. v1 ships a stub; wire local Whisper or an MCP server.
- **Seed card list contents** — owner: decide during Phase 6 Task 1. The ~40–60 entries are tunable as you see which domains you lean on.
- **App name** — owner: you, anytime. `Review Helper` matches the repo; not blocking.
