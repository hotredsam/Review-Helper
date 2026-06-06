# CLAUDE.md — Review Helper

Standing rules for building this repo. Re-read before each task. Keep this file lean — it loads every session.

## What this is

Review Helper is a macOS Tauri app (Rust backend + React/TS/Tailwind frontend, SQLite) that helps the user vibecode the right way. Full context is in `.planning/`. Read `.planning/PLAN.md` (the index) and the current phase file before writing code.

## How to work the plan

- Work one task at a time from the current phase file in `.planning/phases/`. Stop after each task for review.
- Open only the current phase file plus this one. Don't load the other phase files — that's wasted context.
- Re-check `.planning/PLAN.md`'s "Current state" and the phase `Status` before starting. A phase marked `done` is finished — **never rebuild it.** If a done phase looks wrong, raise it; don't silently redo it.
- Tick a task's checkbox when its "Done when" check passes and you've committed. Set a phase to `done` only after its phase-end verification passes; then update the table in `PLAN.md`.
- Use `/start-phase` to run a phase safely (it runs `phase-check` first).

## Rules enforced by mechanism (not just this file)

These are wired so they can't be skipped — but follow them anyway:

- **Secrets never get committed.** A `PreToolUse` hook (`.claude/settings.json` → `.claude/hooks/guard-commit.sh`) runs `scripts/scan_secrets.py` before every `git commit` and blocks it if a key/credential is staged. Don't try to route around it.
- **The SQLite schema is fixed and tested** — `.planning/schema.sql`. Use it as-is; don't hand-write a divergent schema.
- Why mechanism: prose rules in a file like this get ignored under load, and the model doesn't think adversarially about security. The riskiest rules live in scripts/hooks for that reason. This file holds judgment, not guarantees.

## Hard rules (judgment, hold them)

- **The model is read-only against any user's source.** In Review Helper's own logic, planning/analysis model calls pass only read/search tools — never Bash, Edit, or Write. The app performs all file writes and commits itself. Never add a code path that lets the model write or commit a user's repo.
- **Secrets live in the OS keychain or env, never in code, config, or client bundles.** The GitHub token is keychain-only.
- **The frontend never does privileged work.** Filesystem, GitHub API, and `claude` subprocess calls happen in Rust behind named Tauri commands. React calls commands; it doesn't touch disk, GitHub, or subprocesses.
- **Nothing writes to the record or GitHub silently.** Model-inferred changes become pending suggestions the user approves; GitHub deletions/closes happen only after an explicit confirmed preview.

## Prompt for the unhappy path

Build each task's error and edge handling *as you build it*, not "later":

- Handle the offline / `claude`-unavailable / credit-exhausted paths in the phase that introduces the call, not in hardening.
- For every input, ask what happens on empty, malformed, duplicate, or too-large — and handle it.
- A feature isn't done when the happy path works; it's done when the failure paths are handled too.

## Test as you go

- Verify each task against its "Done when" check before moving on. Don't defer testing to Phase 13.
- Test integration early: a UI action that "saves" must be confirmed to actually persist (wire and check the DB layer before building on top of it). The classic vibecoding failure is a button that silently does nothing because a lower layer broke in an earlier step.
- Phase 13 hardens; it does not rescue.

## Context hygiene

- Scope each task to the files it names. Don't refactor unrelated code mid-task.
- `/clear` between unrelated tasks so old context doesn't leak into new work; `/compact` when a session gets long.
- Before accepting a change, summarize what changed, list deleted files, note new dependencies, and flag anything that could break untested code.
- If you hit a fix-one-break-ten loop, stop and roll back to the last green commit instead of digging deeper.

## Architecture & style

- **Small, single-responsibility files.** Split before ~300 lines; treat ~500 as a hard ceiling. A task touching more than 3–4 files is a sign to split the work.
- **Externalize anything that changes** — rubric weights, the seed card list, bank topics, grill-depth presets, theme tokens — into config/JSON, not logic.
- **One model entry point.** All model use goes through `ModelProvider`. No ad-hoc `claude` calls.
- **No hardcoded colors.** Use theme tokens; every color works in all four themes.
- **Match the planning vocabulary** (Understand hub, State pane, pending suggestions, push to main) so code and docs agree.

## Git, dependencies, scope

- Git from the first task; commit at each task's "Commit when" checkpoint — atomic, working commits. Don't overwrite working code to fix something unrelated.
- Justify each new dependency; prefer the standard library or an existing util. Check license compatibility.
- Ship the phase's scope and stop. Don't gold-plate beyond the task or chase perfection — the plan is the scope.
