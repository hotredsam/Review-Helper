# Review Helper — Engineering Handoff

_Last updated: 2026-06-06 · HEAD `ab29ac7` · branch `main`_

This is a complete handoff for a new engineer (or future Sam/Claude). It reflects the
**verified current state** of the repo after all 14 build phases and a 5‑round multi‑agent
"council" review finale. Everything below was grounded by reading the actual files at HEAD.

---

## 1. What this is

**Review Helper** is a macOS desktop app that helps you *"vibecode the right way."* You point it
at a project (a cloned GitHub repo or a blank idea); it reads the repo **read‑only**, assesses it
across six quality dimensions, interrogates you to pin down scope ("grill"), lets you chat and
record decisions/stack/features, synthesizes a **phased build plan**, and syncs that plan
(+ per‑phase GitHub issues) back to GitHub — all behind explicit user approval.

Single self‑contained native app:
- **Backend:** Rust + **Tauri 2**, embedded **SQLite** (rusqlite, bundled).
- **Frontend:** **React 19** + TypeScript + **Tailwind v4**, **Zustand** state, **Vite 7**.
- **AI:** Claude Code via `claude -p` (stream‑json), behind one `ModelProvider` abstraction. The
  model is **read‑only** against user source — the app performs every write/commit itself.

## 2. Status

- **All 14 build phases: done.** Each phase was built task‑by‑task, then reviewed from 5 angles
  (2 subagents each) with a critical/high fix pass.
- **5‑round council finale: complete.** 50 analyst subagents → 3 councils → grand council per
  round, 5 rounds, ~34 improvement steps implemented (one declined with cause — see §11).
- **Tests green:** 106 backend (`cargo test --lib`, +12 `#[ignore]` real‑model/keychain) and
  75 frontend (vitest, 20 files). Frontend `npm run build` clean.
- **Release:** code‑signing + hardened‑runtime wired; **final notarization is the one remaining
  blocking step** and needs Sam's Apple Developer ID (see §13).
- The 5 externally‑seeded GitHub repos were only **cloned/pulled locally** — nothing was pushed
  back to them.

## 3. Tech stack (verified versions)

| Layer | Tech |
|---|---|
| Shell | Tauri 2.x, single window 1200×800 (min 900×640), id `com.reviewhelper.app`, v0.1.0 |
| Backend | Rust (edition 2021), rusqlite 0.40 (bundled SQLite), keyring 3.6 (apple‑native), reqwest 0.12 (blocking+json) |
| Frontend | React 19.1, TypeScript ~5.8, Tailwind CSS v4.3 (`@tailwindcss/vite`), Zustand 5, Lucide icons, Vite 7 |
| Tests | Rust `cargo test --lib`; Vitest 4 + Testing Library |

## 4. How to run / build / test

```bash
# Frontend dev (Vite on :1420 — strict port)
npm install
npm run dev

# Full app in dev (spawns Vite + Tauri)
npm run tauri dev

# Frontend checks
npm test            # vitest run (75 tests)
npm run build       # tsc + vite build (type-check + bundle)

# Backend checks
cargo test --manifest-path src-tauri/Cargo.toml --lib     # 106 tests (12 ignored need creds)

# Production bundle (code-signs if APPLE_SIGNING_IDENTITY set; does NOT notarize)
npm run tauri build
```

> First `cargo build` is slow (Tauri + SQLite compile from source). Subsequent builds are
> incremental. Port 1420 is strict — kill stale dev processes or change it in `vite.config.ts`.

## 5. Architecture

### Backend (`src-tauri/src/`)
- **`lib.rs`** — app entry: opens/migrates the DB, seeds cards, registers **~59 Tauri commands**,
  and `manage()`s the shared state: `Db(Mutex<Connection>)`, `CardGate`, `GrillGate`, `PlanGate`.
  DB‑open failure logs an actionable message and exits cleanly (no opaque panic).
- **Feature‑module pattern:** each feature is `mod.rs` (data/logic + tests) + `commands.rs`
  (the `#[tauri::command]` layer). Features: `assess`, `audit`, `cards`, `chat`, `context`,
  `decisions`, `features`, `github`, `grill`, `model`, `plan`, `projects`, `stack`, `suggestions`,
  `sync`, plus `db`, `util`, `settings`.
- **Background work:** long calls (analyze/kickoff/update/rebuild plan, grill_generate, chat,
  assess) run on `util::spawn_guarded` threads that `catch_unwind` and emit a domain `Failed`
  event on panic. Progress streams over serde‑tagged events (`analysis-event`, `grill-event`,
  `chat-event`, `assessment-event`).
- **Per‑project gates:** `Arc<Mutex<()>>` per project (plan/grill) or per term (cards) serialize
  generation so two runs can't double‑spend the model or collide on `UNIQUE(version)`. Gates
  **recover from poisoning** (`lock().unwrap_or_else(|e| e.into_inner())`) so one panic can't
  brick a project until restart — the `()` payload carries no invariant.

### Frontend (`src/`)
- **Shell:** `App.tsx` → `Sidebar` (nav) + `MainPane` (state‑driven section routing, no router)
  + `NoticeBar` (aria‑live toast). 
- **Sections** (`nav/sections.tsx`), in planning‑loop order:
  `overview → understand → grill → chat → inbox → stack → decisions → plan → learn → settings`.
  `DEFAULT_SECTION = "understand"` (new users land there; returning users land where they left off).
  All sections except `settings`/`learn` are disabled until a project exists.
- **State:** store‑per‑feature Zustand (`store/*.ts`); API wrappers in `api/*.ts` call `invoke()`.
  Background events are wired once via `ensure*Listener()` guards.
  **Pitfall convention:** never select a fresh object/`?? []` inside a selector (infinite re‑render);
  select raw, default outside (see `ChatPane`).
- **Theming:** 4 themes (light/dark/midnight/sand) as CSS‑var token sets in `theme/themes.css`,
  mapped to Tailwind utilities in `index.css`. **No hardcoded colors — tokens only.** A global
  `:focus-visible` rule gives every button/link a themed keyboard focus ring (WCAG 2.4.7).
  `--fg-subtle` meets WCAG AA (≥4.5:1) in all four themes.

### Model & grounding (`model/`, `context.rs`)
- One `ModelProvider` trait; `ClaudeCodeProvider` spawns `claude -p --output-format stream-json …`
  with an **allow‑list of read‑only tools** (`Read, Grep, Glob, WebSearch, WebFetch`) **and** an
  explicit `--disallowedTools Bash,Edit,Write,MultiEdit,NotebookEdit,Task`. No hardcoded auth
  (reuses the user's Claude CLI login).
- `ProjectContext::assemble()` builds a grounding bundle (name, current state, plan body, active
  decisions, answered questions, stack) fresh before each call. `to_prompt()` hardens it:
  a **DATA preamble**, **backtick‑delimited** values, `fence_safe()` (neutralizes backticks so a
  ```` ```fence ```` can't break out), and char‑boundary truncation (`MAX_PLAN_BODY` 16 KB,
  `MAX_FIELD` 800 B). Always injected as `system_append`, never in the user prompt.

### GitHub & sync (`github/`, `sync/`)
- Token lives **only in the macOS Keychain** (service `com.reviewhelper.app`); never logged, never
  in clone URLs or `.git/config` (clone uses a `GIT_ASKPASS` helper reading `GH_TOKEN` env).
- HTTP client: 10 s connect + 30 s overall timeout. URLs are **HTTPS/SSH only** (`parse_repo_ref`
  rejects `http://`).
- **Sync‑out is preview‑then‑confirm:** `sync_main_preview` is pure/read‑only (computes every issue
  action + file deletion); `sync_main_apply` replays the *exact* confirmed preview. In apply, the DB
  lock is **released across network I/O** (read under a short lock → drop → GitHub I/O → re‑lock only
  to record issue numbers) so a slow/hung request can't freeze the app.
- Issue reconciliation is **idempotent + rename‑safe**: match by recorded issue number, then by a
  stable marker in the body; never adopt foreign issues; close only owned orphans. `put_file`/
  `delete_file` return an actionable 409 ("modified since preview — re‑preview"); `delete_file`
  treats 404 as idempotent success.

## 6. Data model

Schema is **fixed and tested** — `.planning/schema.sql`, embedded at compile time via `include_str!`
in `db.rs`. **13 tables:** `projects, plans, phases, tasks, decisions, questions, answers, features,
assessments, stack_selections, learning_cards, suggestions, settings`. (The audit trail is a JSON
array stored in the `settings` KV under `audit:{project_id}`, capped FIFO at 50 — not its own table.)
FKs `ON DELETE CASCADE`, `foreign_keys` pragma per connection, 7 indexes.

Migrations are idempotent, guarded by `user_version`:
- **v1** applies the full schema.
- **v2** rebuilds `learning_cards` for `UNIQUE(term COLLATE NOCASE)` (case‑insensitive — no Foo/foo
  dupes; `INSERT OR IGNORE` collapses variants) and adds `idx_answers_project_question`.

Connection setup also enables **WAL + `synchronous=NORMAL`** (crash‑safe) and **`busy_timeout=5 s`**
(transient locks retry instead of failing). Concurrency model is a **single `Mutex<Connection>`**,
so all DB access is serialized — there are no cross‑connection races.

## 7. Security model (hard rules, enforced)

From `CLAUDE.md` / `SECURITY.md`, several enforced by mechanism:
1. **Model is read‑only against user source** — read/search tools only; never Bash/Edit/Write.
   The app performs all writes/commits.
2. **Secrets live in the Keychain or env, never in code/config/bundle.** A `PreToolUse` hook
   (`.claude/hooks/guard-commit.sh` → `scripts/scan_secrets.py`) **blocks any `git commit`** with a
   staged credential. CI runs the same scan. Don't route around it.
3. **Frontend never does privileged work** — all FS/GitHub/subprocess work is in Rust behind named
   commands; React only calls commands.
4. **Nothing reaches the record or GitHub silently** — model‑inferred changes become *pending
   suggestions* the user approves; GitHub deletions/closes happen only after a confirmed preview.
   `approve_in_tx` rejects a corrupt stored payload (rolls back) rather than writing blanks.
5. **Prompt‑injection defense** as in §5 (DATA preamble, backtick delimiting + neutralization,
   length bounds). Clone scanning **refuses symlinks that escape the clone** in both
   `cards/detect.rs` and `plan/ingest.rs`.
6. **Schema is fixed**; **files split before ~300 lines, ~500 hard ceiling**; **no hardcoded colors**.

## 8. Repo map

```
Review-Helper/
├─ CLAUDE.md                 # standing build rules (read before any task)
├─ HANDOFF.md                # this file
├─ SECURITY.md, RELEASE.md   # trust boundaries; two-step sign+notarize process
├─ .github/workflows/ci.yml  # CI (macos-15): secrets gate + npm build/test + cargo test/build
├─ scripts/scan_secrets.py   # deterministic secrets scanner (hook + CI)
├─ .claude/                  # settings.json (commit hook), hooks/guard-commit.sh
├─ .planning/
│  ├─ PLAN.md                # phase index + status table + locked decisions
│  ├─ schema.sql             # the FIXED 13-table schema
│  └─ phases/                # phase-01..14 task files
├─ src-tauri/                # Rust backend
│  ├─ src/lib.rs             # command registry + managed state
│  ├─ src/db.rs              # connection, pragmas, migrations
│  ├─ src/{assess,audit,cards,chat,context,decisions,features,github,
│  │       grill,model,plan,projects,stack,suggestions,sync,util,settings}*
│  ├─ entitlements.plist     # hardened-runtime entitlements
│  └─ tauri.conf.json        # window, bundle, CSP
└─ src/                      # React frontend
   ├─ App.tsx, components/MainPane.tsx, components/Sidebar.tsx
   ├─ components/*.tsx        # panes + primitives (Modal, Tour, charts, …)
   ├─ store/*.ts             # Zustand store-per-feature
   ├─ api/*.ts               # typed invoke() wrappers
   ├─ nav/sections.tsx       # section registry + DEFAULT_SECTION
   ├─ theme/                 # themes.ts + themes.css + themeStore
   └─ test/                  # 20 vitest files
```

## 9. Dev process

- Work **one task at a time** from the current `.planning/phases/` file; open only that file +
  `CLAUDE.md`. Tick a task only when its "Done when" check passes and you've committed.
- **A `done` phase is finished — never rebuild it.** Use `/start-phase` (runs `phase-check`) to
  confirm prior phases really exist before starting new work.
- Handle the **unhappy path as you build** (offline / claude‑unavailable / credit‑exhausted /
  empty / malformed / too‑large), not "later." Phase 13 hardens; it does not rescue.
- Commit atomically at each task; the secrets gate runs on every commit.

## 10. What was built — the 14 phases

1. Project scaffold & app shell · 2. Model provider layer + Claude availability ·
3. Projects & GitHub connect · 4. Repo analysis & cold start · 5. Assessment engine & State pane ·
6. Understand hub (concept cards) · 7. Grill‑me (repo‑specific interrogation) ·
8. Two‑way chat & structured proposals · 9. Decisions, suggestions & stack panes ·
10. Feature inbox & plan regeneration (update/merge with status carry) · 11. GitHub sync‑out
(planning branch + issues) · 12. Visualization, first‑run tour & polish · 13. Production hardening
(CI, SECURITY.md, RELEASE.md, entitlements, CSP, prompt‑injection) · 14. "Coming soon" learning‑mode
stub.

## 11. The council review finale (5 rounds)

Each round: **50 analyst subagents** → 3 councils (A=Correctness/Security/DataIntegrity,
B=Product/UX/Accessibility, C=Architecture/Maintainability/Testing) → grand council picks bounded,
test‑backed, cross‑council steps. Each round re‑audited the freshly‑improved repo.

| Round | Commit | Findings | Shipped (highlights) |
|---|---|---|---|
| 1/5 | `ac7c03e` | 272 | CSP; grounding fix (answer→question link); `spawn_guarded` panic guards; `PlanGate`; 2 indexes; inline chat approve/dismiss; toast |
| 2/5 | `062df06` | 260 | GitHub HTTP timeouts; sync lock‑off‑network‑IO (`&Db`); DB‑open failure logging; schema **v2** NOCASE migration; sync conflict notes; Modal focus trap; themed Rebuild confirm; debug‑binary cleanup |
| 3/5 | `bdbb1d4` | 254 | context plan/field caps + char‑boundary truncation; suggestion field caps; **WAL**+synchronous; grill `bank()` fallback; SyncPanel token fix; default→Understand + sidebar reorder; ProjectSwitcher keyboard/ARIA |
| 4/5 | `3937606` | 258 | **symlink‑escape fix in ingest**; reject `http://` URLs; **`busy_timeout`**; parameterized suggestions query; backtick neutralization; a11y names; CI pinned `macos-15` |
| 5/5 | `ab29ac7` | 263 | **gate poison recovery**; corrupt‑payload rejection on approve; `delete_file` 409 message; **global focus‑visible ring**; `--fg-subtle` WCAG AA in all themes |

**Engineering judgment exercised:** the grand councils dissolved many false‑positive "high"
findings (single‑`Mutex<Connection>` ⇒ no `last_insert_rowid` races; "291 unwraps" were test‑only;
plan‑body overflow already bounded). In Round 5 one endorsed step — *"make `audit::record` atomic"* —
was **declined**: `record` is only called from `commit_fresh`/`commit_merge`, both already inside a
transaction under the single connection lock, so appends are already atomic; the proposed nested
`BEGIN`/`&mut Connection` fix would have broken the build. Documented in the commit.

## 12. Testing & CI

- **Backend:** 106 `cargo test --lib` pass; 12 `#[ignore]` (real‑model/keychain, need creds).
  Coverage: model parsing, plan merge/status‑carry, issue reconciliation, schema migrations + NOCASE,
  symlink canonicalization, URL validation, context truncation, gate poison recovery, suggestion
  validation, sync.
- **Frontend:** 75 vitest across 20 files (every pane/flow + a11y).
- **CI** (`.github/workflows/ci.yml`, **pinned `macos-15`**): secrets gate → `npm ci` + `npm run
  build` + `npm test` → `cargo test --lib` → `cargo build` (config validation; no bundle/sign).

## 13. Release readiness

`RELEASE.md` is a **two‑step** process:
1. **Automated:** `APPLE_SIGNING_IDENTITY=… npm run tauri build` code‑signs the `.app` with the
   hardened runtime (`entitlements.plist`).
2. **Manual, credential‑gated:** `xcrun notarytool submit --wait` + `xcrun stapler staple`.

**The only blocking step for a public release is #2 — it needs Sam's Apple Developer ID cert +
app‑specific password** (a binary credential that can't be committed or env‑passed). Without it the
app runs locally but trips Gatekeeper on other Macs. Also flagged: consider dropping
`allow-unsigned-executable-memory` from `entitlements.plist` if a clean notarize+launch still works.

## 14. Known limitations / deferred

- **Audio transcription** — `transcribe_audio_stub` returns a TODO; wire local Whisper or an MCP
  server.
- **Local model provider** + `local_endpoint` / `api_credit_overflow` settings — stubs only
  (`LocalStubProvider` always returns Unavailable); routing point is `model::commands::provider_for`.
- **GitHub device‑flow OAuth** — built + tested but inactive until a `client_id` is configured
  (no UI yet; the working path is importing the `gh` CLI token into the Keychain).
- **Two‑way issue sync** — plan→issues is one‑way; pulling issue edits back is out of scope.
- **Chat transcripts not persisted** — multi‑turn uses the CLI `session_id`, but messages are
  in‑memory only.
- **Learn mode (Phase 14)** — coming‑soon stub.
- **App Store submission** — out of scope; ships as a signed/notarized `.app`.
- Minor: stale gate‑map entries for deleted projects aren't pruned (negligible); a few files near
  the size ceiling (`PlanPane.tsx` ~304, `NewProjectDialog.tsx` ~330) could be split.

## 15. Gotchas for the next engineer

- **Schema is locked** — change it only via a new idempotent migration (`v3+`, bump `user_version`),
  never by hand‑diverging `schema.sql`.
- **Themes touch two files** — add to `theme/themes.ts` (id + registry) *and* `theme/themes.css`
  (token block); the anti‑flash script + store share `THEME_STORAGE_KEY`. Run a contrast check.
- **Tauri command names must match** exactly between `invoke('name')` (TS) and `#[tauri::command]`
  (Rust) — mismatches fail at runtime. Wire + test the bridge early.
- **Build the model request via `ModelRequest::planning()`** so the read‑only allow‑list is applied;
  inject context as `system_append`, never in the user prompt.
- **`ClaudeCodeProvider.run()` is blocking** — always call it on a `spawn_guarded` thread.
- **Section order in `nav/sections.tsx` is the planning loop**, not alphabetical — don't reorder
  casually.
- **The secrets hook is real** — if `scan_secrets.py` flags something, it's almost always genuine;
  move it to env/Keychain or gitignore the file.

## 16. Suggested next steps

1. **Ship:** run `RELEASE.md` step 2 with Apple credentials; verify `spctl`/`codesign`, test
   notarize+launch on a clean Mac; minimize entitlements.
2. **`tauri dev` smoke test** under the restrictive CSP to confirm assets load.
3. Wire one real **local provider** (Ollama endpoint) behind `provider_for` to cut Claude spend on
   routine work (card lookups, scoring).
4. Persist **chat transcripts** to the DB (table or `settings` KV) for resumable history.
5. Implement **audio transcription** for the feature inbox.
6. Optional hygiene: split the two oversize components; add a small GC for stale clones/gate entries.
