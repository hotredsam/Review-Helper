# Agent instructions — Review Helper

You are working in a Tauri 2 app: Rust backend (`src-tauri/src/`), React 19 + TS frontend (`src/`), embedded SQLite. Read `.planning/PLAN.md` and the current phase file before changing anything; never rebuild a phase marked done.

## Hard rules

- **The model is read-only against user repos.** Planning/analysis calls pass read/search tools only. Never add a path that lets a model write or commit a user's code.
- **No silent writes.** Model-inferred changes become pending suggestions; GitHub deletions/closes happen only behind an exact confirmed preview.
- **Destructive UI actions confirm via the shared `ConfirmDialog`/`Modal`** — `window.confirm` is dead under wry. Never swallow a failed delete.
- **Never hold the DB mutex across a model or network call.** Read under a short lock, drop it, then call out. Timeouts live in `src-tauri/src/config.rs`.
- **Grounded tutor calls never get web tools** (`ModelRequest::grounded()`); web access is the labeled, per-subject opt-in path only.
- **Secrets never get committed** — a pre-commit hook runs `scripts/scan_secrets.py`; don't route around it.

## Verifying work

- `npm test` (frontend) and `cargo test --manifest-path src-tauri/Cargo.toml --lib` (backend) must both be green.
- The IPC contract suite (`src/test/ipc-contract.test.ts`) statically checks every `invoke()` against registered Rust commands — if you add/rename a command, it will tell you.
- New behavior needs a regression test that fails on the old code.
