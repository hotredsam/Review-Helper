# Review Helper — Architecture

How the v1 in `REQUIREMENTS.md` is built. Decisions here are settled; the phase files in `phases/` assume them.

## 1. Process & stack

- **Tauri** desktop app. The frontend renders in a native webview; the Rust backend and SQLite live in-process. One bundled, signed, notarized `.app`.
- **Frontend:** React + TypeScript, Tailwind. State via a lightweight store (Zustand). Four themes as CSS-variable token sets; the UI never hardcodes colors.
- **Backend:** Rust (Tauri commands). Owns the database, the GitHub API client, the local clone cache, the model-provider layer, and every file write/commit. The frontend calls named commands; it never writes files, calls GitHub, or spawns `claude` directly.
- **Database:** SQLite in the app's data dir (`rusqlite` or `sqlx`), migrations on launch.

The frontend is "pixels + intent"; all privileged work (filesystem, GitHub network, spawning `claude`) happens in Rust behind named commands. Small security surface, auditable.

## 2. Model (LLM) provider layer

One trait, `ModelProvider`, used everywhere the app needs the model. Each call states its purpose, the project context to inject, and the tools it's allowed.

```
trait ModelProvider { fn run(&self, req: ModelRequest) -> Stream<ModelEvent>; }

struct ModelRequest {
    system_append: String,          // task-specific instructions
    prompt: String,
    context_bundle: ProjectContext, // plan + decisions + answered questions + stack
    allowed_tools: Vec<Tool>,       // read-only set for planning (see §3)
    output_format: StreamJson,
    session_id: Option<String>,     // for multi-turn (chat)
}
```

**v1 adapter — `ClaudeCodeProvider`:** spawns `claude -p` with `--output-format stream-json`, `--append-system-prompt`, an `--allowedTools` allow-list, and `--resume`/`--continue` for multi-turn chat. Reuses the user's Claude login. Stream events are parsed and forwarded to the frontend live.

**Stubbed adapters (Settings, off by default):** `LocalModelProvider` (an OpenAI-compatible endpoint for routine work — interface defined, implementation a "configure me" stub) and an API-credit/overflow flag (read by future routing; no behavior in v1).

**Routing:** v1 routes everything to `ClaudeCodeProvider`. The interface is shaped so a future router can send routine work (assessment scoring, card lookups, summaries) to the local provider and reserve `claude -p` for heavy reasoning.

**Unavailability:** if the subprocess can't start or errors, the provider returns a typed `Unavailable` event. The UI shows "Claude not available" + retry and stays read-only. The debug panel surfaces the last command, exit code, and stderr.

## 3. Read-only code access (security model)

Planning never lets the model touch source. Three layers:

1. **Tool allow-list:** planning `ModelRequest`s pass only read/search tools (Read, Grep, Glob, WebSearch). No Bash, Edit, or Write.
2. **Clone-only visibility:** the model reads the shallow clone in the cache dir; it has no path to the user's working tree.
3. **App owns writes:** every `.planning/` file, `CLAUDE.md`, commit, branch, and issue is created by the Rust backend via the GitHub API / file writes — never by the model. The model proposes; the app persists.

This same discipline is enforced on Review Helper's *own* build by a commit hook — see §9.

## 4. Data model (SQLite) — see `schema.sql`

The schema is in `.planning/schema.sql` and has been tested: it compiles, the `CHECK` enums reject bad values, and FK cascades work. **Use it directly; don't re-derive it from prose.** It defines 13 tables: `projects`, `plans` (versioned), `phases` (with `status` and a `marker` for issue matching), `tasks`, `decisions` (ADR fields + `status`), `questions`, `answers`, `features` (inbox lifecycle), `assessments`, `stack_selections`, `learning_cards` (shared across projects), `suggestions` (the approval queue), and `settings`. `project_id` scopes most rows; deleting a project cascades.

## 5. The planning engine

- **Context bundle:** before any model call, the backend assembles `ProjectContext` (latest plan + active decisions + answered questions + current stack) and injects it. Every call is grounded in current state.
- **Structured proposals:** the system prompt instructs the model to emit any inferred update as a tagged block (decision / answer / feature / stack). The backend parses these into `suggestions` rows shown as pending cards (Approve / Approve all). No silent writes.
- **Assessment:** a read-only call runs `skills/big-picture/scan.py` for deterministic facts (file sizes, tests/CI/CLAUDE.md presence, secret patterns, git depth), then scores the six dimensions + production-readiness + hygiene from those facts → an `assessments` row. The `big-picture` skill issues the same scan + rubric in the terminal.
- **Grill generation:** bank topics seed the dimensions; a model call writes repo-specific question text + a recommended answer per seed → `questions` rows. The Detail Coverage meter is the assessment engine pointed at "is this *specified* enough," scaled by the grill-depth slider.
- **Plan update:** "Update plan" feeds approved answers + the triaged inbox into a merge call that returns a new plan body **preserving prior phases, decisions, and per-phase status** → a new `plans` version. "Rebuild plan" regenerates from scratch (rare).

## 6. GitHub sync

- **Auth:** OAuth device flow; token in the OS keychain.
- **Clone cache:** shallow clone per project under the app data dir; refresh re-pulls. The model reads only here.
- **Import reconciliation:** on attach, the backend reads existing planning files and open issues and prepares a reconciliation plan (docs to absorb; issues to adopt/relabel/close; dupes). Nothing applies without the user's OK.
- **Push to planning branch:** writes `.planning/` to a `planning` branch (WIP).
- **Push to main:** previews the full change set (new `.planning/` + `CLAUDE.md`, legacy files to remove, issues to open/relabel/close), waits for confirmation, then applies.
- **Issue sync:** one issue per phase, matched by the phase `marker` so re-pushes update rather than duplicate. Phase status → open/closed + `phase:current` / `phase:future`. One-way (plan → GitHub) in v1.

## 7. Visualization

Fixed components rendered from assessment/plan data, surfaced contextually — radar (six dimensions), gauge (production readiness), progress bars (phase status), donut (stack composition, hygiene). No model-generated UI.

## 8. Conventions

- Small, single-responsibility files; see `CLAUDE.md` for the size threshold and the rest.
- All tunable values (rubric weights, the seed card list, bank topics, grill-depth presets, theme tokens) live in config/JSON, not hardcoded.

## 9. Build-time enforcement (this repo)

The riskiest rules are enforced by mechanism while Claude Code builds this app, not left to prose:

- **`.claude/settings.json`** registers a `PreToolUse` hook on `Bash`.
- **`.claude/hooks/guard-commit.sh`** intercepts `git commit`, runs **`scripts/scan_secrets.py`** over staged files, and exits 2 (blocking the commit) if anything is found. Tested: it blocks a staged secret, allows a clean commit, and ignores non-commit commands.
- **`scripts/scan_secrets.py`** is the deterministic scanner (the same logic ships as the `secrets-gate` skill for any repo).
- **`.claude/commands/start-phase.md`** runs `phase-check` then a single phase, one task at a time.

This mirrors the read-only-model principle (§3): the agent can't commit a leaked credential even if a prose rule were ignored.
