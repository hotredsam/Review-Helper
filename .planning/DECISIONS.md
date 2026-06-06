# Review Helper — Decisions

Architecture decision records, newest first. Each is the format the app's decisions pane uses. Status is `active` unless superseded.

---

### D1 — Desktop framework: Tauri
**Status:** active
**Choice:** Build as a Tauri app (Rust backend + React/TS/Tailwind frontend), one signed native bundle with embedded SQLite.
**Rationale:** A single self-contained app (no browser, no separate backend/DB to launch) that plays to existing Rust skill and a familiar web UI.
**Alternatives:** SwiftUI (truest native, but new language, Mac-only, more to rebuild); Electron (same shape as Tauri but heavier, no Rust synergy).
**Consequences:** Cross-platform-capable later; the web UI carries the panel-heavy design cheaply.

### D2 — Model access via `claude -p`, behind a provider interface
**Status:** active
**Choice:** Drive Claude Code through `claude -p` headless (stream-json), reusing the existing Claude login, with all calls behind a `ModelProvider` interface.
**Rationale:** No separate API key to manage; one seam for all model use.
**Alternatives:** Direct Anthropic API (separate billing); Agent SDK in TS (heavier integration).
**Consequences:** After 2026-06-15, programmatic use draws on a separate Agent-SDK credit; the interface plus a stubbed local-model slot exist to manage cost later.

### D3 — The model is read-only against source; the app owns all writes
**Status:** active
**Choice:** Planning/analysis calls pass only read/search tools (no Bash/Edit/Write); Review Helper performs every file write and commit via the GitHub API.
**Rationale:** Closes the top vibecoding failure mode — the AI loose in the filesystem — by construction.
**Alternatives:** Let the model write planning files directly (rejected: removes the safety guarantee).
**Consequences:** A clear seam; the model proposes, the app persists.

### D4 — Assessment is 0–100 (numbers + color in app; numbers-only in the skill)
**Status:** active
**Choice:** Score six vibecoding dimensions plus a separate Production Readiness scorecard and a hygiene check, all 0–100. The app shows number + color tint; the `big-picture` skill prints numbers only.
**Rationale:** The skill runs in a terminal where color isn't reliable; the app can use both. Same rubric so they agree.
**Alternatives:** Red/amber/green only (rejected: less precise; doesn't carry to the terminal).
**Consequences:** Scores are grounded in `scan.py` metrics, not impressions.

### D5 — Plan is the source of truth; one-way sync to phased GitHub issues
**Status:** active
**Choice:** The plan drives one issue per phase, matched by a stable marker (no dupes); phase status maps to issue state + label; GitHub-side edits are not pulled back in v1.
**Rationale:** Avoids two-way merge conflicts while keeping issues consistent.
**Alternatives:** Two-way sync (deferred: conflict complexity not worth it for v1).
**Consequences:** Editing issues on GitHub won't change the plan.

### D6 — Push to main is gated; old plans are consolidated, not lost
**Status:** active
**Choice:** Two pushes — a `planning` branch (WIP) and a gated push to `main`. Push-to-main previews every change (new docs, legacy files to remove, issues to relabel/close) and waits for confirmation; existing plan content is absorbed into the new plan before legacy files are removed.
**Rationale:** No silent changes to the user's GitHub; nothing is forgotten.
**Alternatives:** Auto-apply (rejected: unsafe); never remove old files (rejected: leaves inconsistent plans).
**Consequences:** Every repo converges on one consistent plan shape.

### D7 — Suggestions are approved, not auto-written; single + mass approve
**Status:** active
**Choice:** Model-inferred decisions/answers/features/stack changes appear as pending suggestions with single Approve and Approve all.
**Rationale:** Keeps the user in control without per-item tedium.
**Alternatives:** Auto-accept with undo (rejected: less control); approve-each-only (rejected: too manual).

### D8 — The Understand hub is the spine, spanning build and product domains
**Status:** active
**Choice:** A prominent, self-extending learning hub covering architecture, frontend, backend, pipes, deployment, business, design, and UX — reached via a Why? button, the hub pane, and the chat.
**Rationale:** Understanding is the main activity (most of the user's time), not a side glossary.
**Alternatives:** A tech-only glossary in a corner (rejected: too narrow, under-weighted).

### D9 — Grilling is hybrid and depth-controlled
**Status:** active
**Choice:** A bank supplies the topic; the model writes repo-specific question text plus a recommended answer. A slider sets depth (~1–5h) and a Detail Coverage meter signals "Done grilling."
**Rationale:** Repo-specific questions with a draft answer beat a static form; the user controls how exhaustive it gets.

### D10 — Build-time discipline is enforced by mechanism, not prose
**Status:** active
**Choice:** A `PreToolUse` hook blocks any `git commit` that contains secrets (via a tested scanner); deterministic facts back the skills; the SQLite schema ships pre-tested.
**Rationale:** Prose rules in CLAUDE.md get ignored; the riskiest rules belong in scripts/hooks the agent can't skip.
**Alternatives:** Prose-only rules (rejected per the research: the AI doesn't think adversarially and CLAUDE.md gets ignored).
