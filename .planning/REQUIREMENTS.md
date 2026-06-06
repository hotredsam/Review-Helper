# Review Helper — Requirements

Structured from the planning sessions. This is the authoritative list of what v1 does. `ARCHITECTURE.md` says how; `PLAN.md` says in what order.

## 1. Platform & shape

- macOS desktop app, built with Tauri (Rust backend + React/TypeScript frontend, Tailwind).
- A single bundled native app: one signed icon, native window (no browser tab), backend and database inside the app. Nothing to launch separately.
- Local single-user data in embedded SQLite.
- Target polish: ~90% of App-Store-grade feel — fast UI, no rough edges, reliable, works as intended. Code-signed and notarized so it opens without Gatekeeper warnings. No App Store submission committed in v1.

## 2. Navigation & projects

- Left hamburger menu switches between projects.
- "Add project" offers four paths:
  - **Import from GitHub** — pick one of your repos.
  - **New blank project** — no repo yet.
  - **Link existing repo** — paste a GitHub URL to attach a repo you made elsewhere.
  - **Create repo from app** — Review Helper creates the GitHub repo for you.
- A blank/new project shows clean, non-ugly empty states and fills in progressively. It opens with a "What is this repo?" prompt to start the conversation; app type and panes populate as you answer.
- A project without a repo can be converted to a GitHub project later by pasting a URL or hitting "create repo."

## 3. Cold start (what you see right after attaching)

- **Imported repo:** Review Helper shallow-clones to a local cache, then shows a loading indicator while Claude analyzes the code and ingests any existing planning docs. A solid plan appears when it's ready — no half-finished "draft" state shown.
- **Blank project:** starts empty; the "What is this repo?" prompt seeds it; everything builds up as you answer and chat.
- A refresh button re-pulls the repo from GitHub on demand.

## 4. The planning loop (the core)

The loop is where you live. Inputs feed one record; you iterate until the project is specified well enough to build.

### 4.1 Understand hub (the spine — most of your time is here)

- A first-class, prominent pane — not a glossary in the corner.
- Covers **any topic, any depth**, across both the build side (architecture, frontend, backend, the pipes/glue, deployment) and the product side (business model and monetization, graphic design, user behavior and UX).
- Reached three ways, all resolving to the same hub:
  - **Why? button** on every decision and every choice (a stack pick, a design call, a pricing assumption).
  - **The hub pane** — browse what you've learned, or ask something cold.
  - **The chat** — open-ended discussion with full project context.
- Whatever you learn is saved as a **card** so it's there next time.
- Cards are **self-extending**: any technology or concept you or Claude name that lacks a card gets one generated and stored.
- Card library is **seeded** from a curated recommended set (~40–60 high-value entries) plus the technologies detected in your GitHub repos. Anything not seeded is generated on demand and cached, so you never hit a dead end.

### 4.2 Grill-me cards

- Questions come from a **hybrid** source: a bank supplies the topic/dimension; Claude writes the actual, repo-specific question text. Each question ships with Claude's **recommended answer** so you correct a draft rather than face a blank box.
- Shown **five at a time**. Each card has:
  - **Submit** — record your typed (or spoken) answer.
  - **Not relevant** — archive it; it won't resurface.
  - **I don't know** — Claude offers its recommended answer as the decision, or re-explains the question more simply.
  - **Let's chat about this** — opens the chat for that one question; the resolution flows back into the card as its answer.
  - **Delete** — remove it entirely.
- Answered and dismissed both count as "addressed."
- A **detail/rigor slider** (low → exhaustive) controls how deep and how long the grilling goes — expect anywhere from ~1 to ~5 hours at the high end. The slider sets the saturation target.
- A **Detail Coverage meter** estimates, per dimension, whether the project is specified enough to build. It flips to **"Done grilling"** when coverage saturates against the slider setting and no high-value questions remain. It re-opens if new features or answers create gaps.
- Underspecified items from the Feature Inbox trigger grilling.

### 4.3 Chat (two-way)

- Powered by Claude Code via `claude -p` (see `ARCHITECTURE.md`).
- Two-way: Claude grills you, and you can ask it anything at any time, with the project's full context loaded.
- Claude surfaces anything it infers — a decision, an answer, a new feature, a stack change — as structured "proposed updates" that become pending suggestions (§4.6).

### 4.4 Stack panes

- Five panes: **frontend, backend, database, deployment, and pipes** (the glue — API clients, auth wiring, env/secrets, queues).
- Each pane shows Claude's recommended choice plus 2–3 alternatives, each with a one-line plain-English rationale and a tap-through to its learning card, plus a Why? button.
- Pre-made stacks per app type fill all five panes via **"apply to all"**; you can override any single pane afterward.
- Selections are recorded as decisions and feed the plan.

### 4.5 Feature Inbox

- A place to brain-dump features in **text or audio** (also available from the chat).
- Items land as candidates with a status: inbox → triaged → in-plan → rejected.
- On "Update plan," Claude triages the inbox — dedupes, maps each item to a phase, flags conflicts — and proposes incorporations you approve.
- The pending counter includes waiting features.

### 4.6 Pending suggestions & approval

- Anything Claude infers (decision, answer, feature, stack change) appears as a **pending suggestion card**.
- Approve them with a single **Approve** or a **Approve all**. Nothing is written to the record silently.

### 4.7 Decisions record

- Robust ADR-style entries: topic, the choice, a short rationale, alternatives considered, consequences, where it came from (which answer or chat), a timestamp, the plan version, and a status (active / superseded).
- This is the record the Why? button reads from and what gets written to `DECISIONS.md` on push.

### 4.8 Plan regeneration

- **Update plan (N pending)** — an incremental merge that preserves existing phases and decisions, weaves in new answers, and triages the inbox. This is the default action you'll use most.
- **Rebuild plan** — a full regeneration for big pivots; rare, with a warning.
- A counter badge shows unincorporated answers and features; it resets on update. A soft nudge appears at ~10 pending.
- Every incorporation is logged (which answer → which plan version) so merges are auditable.

## 5. Assessment

- Scored as **0–100 percentages, 100 = best.** In the app these show **both the number and a color tint**; the `big-picture` skill (terminal) shows **numbers only**, since color isn't reliable there. Both use the identical rubric.
- **Six vibecoding dimensions**, shown on a radar: architecture, modularity, context hygiene, security/secrets, git discipline, workflow. Plus an overall score and the **top three fixes**.
- A **separate Production Readiness scorecard** (its own panel, shown beside the radar): tests present and passing, error handling, secrets handling, build + CI, dependency health, docs.
- A **repo-hygiene sub-check** under architecture: orphaned/unused files, dead code, unused dependencies, structure sanity — surfaced as an explicit cleanup list.

## 6. Model (LLM) layer

- v1 drives Claude Code via `claude -p` headless mode (streaming), reusing your existing Claude login — no separate API key required.
- Every model call goes through one **provider interface**.
- Settings holds two "later" slots, off by default: an **API-credit / overflow** toggle, and a **local-model endpoint** (point it at your own GPU box) for cheap/routine work.
- Billing note carried into the design: after June 15, 2026, `claude -p` / Agent SDK usage on subscription plans draws from a separate monthly Agent-SDK credit (sized to the plan, opted into once, no rollover). The provider interface and the local-model slot exist partly to manage this.
- **Read-only code access:** planning runs give Claude read-only tools only (read, search) — no shell, no edit, no write. **Review Helper itself** performs every file write and every commit/push via the GitHub API. Claude never modifies your code.

## 7. GitHub integration

- Auth via GitHub OAuth **device flow** (no token pasting).
- On import, Review Helper reconciles what's already there:
  - **Existing planning docs** are ingested; their content is absorbed into the new plan (not lost). The legacy planning files themselves are removed on push to main.
  - **Existing issues** are read and grouped: ones that map to phases are adopted/relabeled into Review Helper's system, stale ones are proposed for closing, duplicates are flagged.
- Two push buttons:
  - **Push to planning branch** — for work in progress, on a dedicated `planning` branch.
  - **Push to main** — makes the plan canonical.
- Push to main writes `.planning/` (the planning docs) + `CLAUDE.md` + one GitHub issue per phase, and shows a **preview of everything it will change — and waits for your OK before deleting or closing anything.** No silent changes to your GitHub.
- Plan and issues are kept **in sync, with the plan as the source of truth.** Issues are matched by a stable hidden marker so re-pushing never duplicates them. GitHub-side edits are not pulled back in v1 (one-way).
- Phase status is tracked locally (not started / in progress / done) and reflected in the State pane and as the issue's open/closed state + label (`phase:current` / `phase:future`).

## 7.1 The plan that gets produced

- A phased plan with **at least 10 phases** (a given project may have more — complexity dictates the count). Each phase has multiple steps/tasks.
- The plan carries **explicit per-phase/step completion status** and a "current state" header, written so Claude Code resumes at the right place and never restarts from Phase 1.
- A per-project `CLAUDE.md` is generated and included in the pushed package.

## 8. Visualization

- No literal runtime LLM-generated UI. A fixed, polished component set surfaces contextually: a **radar** for the vibecoding assessment, a **gauge** for Production Readiness, **progress bars** for phase/step completion, a **donut** for stack composition and the hygiene cleanup list, and counters for pending answers/features.

## 9. Settings

- **4 themes** (light/dark + accents) on design tokens.
- **Model provider** config: Claude (`claude -p`) by default; the later slots for credit/overflow and a local endpoint.
- **Debug panel** — logs and diagnostics, useful when Claude is unreachable.
- **Audio input** config (see §10).

## 10. Audio input

- A mic button on the answer box, and audio capture for the Feature Inbox brain-dump.
- v1 ships a **stub transcription service** behind a clean interface (with a clear TODO); you wire it to local Whisper or an MCP server yourself later.

## 11. Error & offline behavior

- If `claude -p` is unreachable — offline, not installed, or credit exhausted — the app shows a plain **"Claude not available"** banner with a retry, and stays usable read-only (browse plan, decisions, cards, learning library).
- No data is ever lost (everything persists to SQLite).

## 12. First-run

- A light first-run flow: connect GitHub, then pick / create / link a project.
- A short 4–5 step tour of the panes.
- The Understand hub as the standing teacher, with an "explain anything" box.
- "?" tooltips on jargon throughout.

## Non-goals (v1)

- No runtime LLM-generated UI (charts/components are a fixed set surfaced contextually).
- No App Store submission (notarized local install only).
- No two-way GitHub-issue sync (plan → issues is one-way).
- The "learning mode" beyond vibecoding is stubbed/"coming soon," not built.
- Claude is read-only against your code; Review Helper never lets the model write or commit your source.

## 13. Build-time enforcement (for building Review Helper itself)

These govern Claude Code while it builds this app, turning the riskiest rules from prose into mechanism:

- A `PreToolUse` hook (`.claude/settings.json` → `.claude/hooks/guard-commit.sh`) runs the secrets scanner before any `git commit` and blocks the commit (exit 2) if anything is found — so the security rule can't be forgotten.
- `scripts/scan_secrets.py` is the deterministic scanner the hook uses (the same logic ships as the `secrets-gate` skill for use in any repo). Tested against planted secrets and clean files.
- `.planning/schema.sql` is the tested SQLite schema (compiles; CHECK constraints and FK cascades verified). Use it directly rather than re-deriving the schema from prose.
- The `/start-phase` slash command runs `phase-check`, then a single phase, one task at a time.
