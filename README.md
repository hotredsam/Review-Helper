<div align="center">

<img src="./docs/banner.svg" alt="Review Helper" width="100%" />

<br/>

**A macOS desktop app that helps you _vibecode the right way_ — analyze a project, score it, get grilled until it's specified well enough to actually build, then ship a phased plan to GitHub. Plus a full _Learning mode_ that adapts study material to how you actually learn.**

<br/>

![macOS](https://img.shields.io/badge/macOS-desktop-111?logo=apple&logoColor=white)
![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-backend-CE412B?logo=rust&logoColor=white)
![React 19](https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=black)
![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white)
![Tailwind v4](https://img.shields.io/badge/Tailwind-v4-38BDF8?logo=tailwindcss&logoColor=white)
![SQLite](https://img.shields.io/badge/SQLite-embedded-003B57?logo=sqlite&logoColor=white)
<br/>
![status](https://img.shields.io/badge/status-v0.1%20feature--complete-3FB950)
![modes](https://img.shields.io/badge/modes-Code%20%2B%20Learning-8957E5)
![themes](https://img.shields.io/badge/themes-8-38BDF8)
![review](https://img.shields.io/badge/council%20review-5%20rounds-8957E5)
![tests](https://img.shields.io/badge/tests-124%20backend%20%C2%B7%2086%20frontend-3FB950)

</div>

---

## What is this?

Review Helper turns _"I have a vague app idea"_ into a build you can trust. It points a model at your project (or just your description), scores it across the dimensions that make AI-assisted builds succeed or fail, interrogates the gaps, teaches you the concepts as you go, and produces a single consistent **phased plan** synced to GitHub issues. You hand that plan to your coding agent and build it phase by phase — with the guardrails already in place.

It's one self-contained native app: a Rust backend with embedded SQLite and a React UI. No servers to run, no separate database to launch.

## Why

Most AI-assisted projects don't fail at the code — they fail at the **spec**: under-specified ideas, the model loose in your filesystem, secrets committed by accident, finished work silently rebuilt. Review Helper closes those failure modes by construction.

> [!NOTE]
> **The model is read-only against your source.** Planning and analysis run with read/search tools only — never write, edit, or shell. The app performs every file write, commit, and issue change itself, and only after you approve it. Model-inferred changes arrive as **pending suggestions**, never silent writes; secrets are blocked from commits by a deterministic scanner; and the database schema ships pre-tested.

## Features

- 🔍 **Analyze & score** — six vibecoding dimensions, a separate production-readiness scorecard, and a hygiene check, all 0–100 and grounded in real repo metrics rather than vibes.
- 🔥 **Grill-me** — repo-specific questions (each with a draft answer) that pin down what you're actually building, **rendered as real inputs** (choices, sliders, short/long text) the model picks per question. A depth slider and a **Detail Coverage** meter tell you when you've specified enough.
- 📚 **Understand hub** — a self-extending, **filterable** card library spanning architecture, frontend, backend, pipes, deployment, business, design and UX. Filter to **just this project**, generate a card for any term, and open an **inline mini-chat** on any card to dig deeper.
- 💬 **Two-way chat & memory** — talk your project through; chats **persist across restarts** with a past-chats rail and **full cross-chat memory**, and anything the model infers becomes a pending suggestion you approve (single or bulk) — nothing reaches the record silently.
- 🪄 **Easy ↔ Technical** — one global toggle flips every explanation between plain-language and precise-technical registers; the model pre-generates both, so switching is instant and free.
- 🗂️ **Decisions, stack & feature inbox** — an ADR-style decision log (with supersede history), a five-pane stack picker, and a quick-capture inbox that nudges you to fold ideas into the plan.
- 🧭 **Plan → GitHub** — one consistent phased plan rendered as clean Markdown with collapsible phases, synced one-way to issues (one per phase, matched by a stable marker so re-pushes update instead of duplicating), behind a gated, fully-previewed push to `main`.
- 🎙️ **Local voice capture** — dictate Inbox ideas to a real on-device Whisper model (`large-v3-turbo`, Metal-accelerated): live streaming partials while you speak, a clean final transcript on stop, and nothing ever leaves your Mac. Tested at ~3× realtime on an M4.
- 🎓 **Learning mode** — a second top-level shell: name a subject (describe it or upload notes/a PDF), get **grilled on scope first**, pick from a **generatively-proposed study plan**, then study generated **notes, flashcards, and quizzes** with a built-in **tutor** — all adapting to how you actually learn ([see below](#learning-mode--study-anything-adaptively)).
- 📊 **Visualization & onboarding** — radar/gauge/donut charts for the scorecard, a first-run tour, and inline "why" explainers.
- 🖼️ **Design export** — `npm run export:design` bundles every screen (sample data, all themes) into one self-contained HTML snapshot on your Desktop.
- 🎨 **Eight themes** — `light`, `dark`, `midnight`, `sand`, `nord`, `forest`, `rose`, `grape`, every surface driven by design tokens, WCAG-AA contrast, and full keyboard focus rings.

## Screenshots

> 📸 This section is wired up — drop the six PNGs named below into [`docs/screenshots/`](./docs/screenshots/) (see the [capture guide](./docs/screenshots/README.md)) and they render here automatically. Capture on a project that has a generated plan + assessment so the data-rich views show real content.

<table>
  <tr>
    <td width="50%"><img src="./docs/screenshots/overview.png" alt="Overview — assessment scorecard"/><br/><sub><b>Overview</b> — six-dimension scorecard, production readiness, and hygiene as radar + gauges.</sub></td>
    <td width="50%"><img src="./docs/screenshots/grill.png" alt="Grill — repo-specific questions"/><br/><sub><b>Grill</b> — repo-specific questions with draft answers, the depth slider, and the coverage meter.</sub></td>
  </tr>
  <tr>
    <td><img src="./docs/screenshots/plan.png" alt="Plan — phased plan"/><br/><sub><b>Plan</b> — the phased plan with tasks, decisions, and stack, plus the gated GitHub sync panel.</sub></td>
    <td><img src="./docs/screenshots/chat.png" alt="Chat — pending suggestions"/><br/><sub><b>Chat</b> — two-way conversation with pending suggestions you approve or dismiss inline.</sub></td>
  </tr>
  <tr>
    <td><img src="./docs/screenshots/understand.png" alt="Understand hub"/><br/><sub><b>Understand</b> — the self-extending concept-card hub across every build domain.</sub></td>
    <td><img src="./docs/screenshots/themes.png" alt="Eight themes"/><br/><sub><b>Themes</b> — the same screen across all eight: <code>light</code> · <code>dark</code> · <code>midnight</code> · <code>sand</code> · <code>nord</code> · <code>forest</code> · <code>rose</code> · <code>grape</code>.</sub></td>
  </tr>
</table>

## How it works — the planning loop

The sidebar follows the loop, in order: **understand → grill → chat → inbox → stack → decisions → plan → sync**. You gather understanding and scope first; the plan is the synthesis, not the starting point.

```mermaid
flowchart LR
    U["📚 Understand"] --> G["🔥 Grill"] --> C["💬 Chat"] --> I["🗂️ Inbox"]
    I --> S["🧱 Stack"] --> D["🧭 Decisions"] --> P["📋 Plan"] --> Y["🔄 Sync → GitHub"]
    A["🔍 Assess / Overview"] -.-> U
    Y -.->|next iteration| U
```

## Learning mode — study anything, adaptively

Flip the **Code ↔ Learn** switch and the whole app becomes a study workspace. Add a subject (describe a goal, or upload material — text, Markdown, or PDF) and it scopes, proposes, then builds material that adapts as you go. Big documents are **fully covered**: uploads are split into structure-aware sections, the study plan is proposed per section (with live progress), and every module's notes/cards/quiz are grounded on the exact part of the document they came from — no silent truncation. Flashcards run on a real **FSRS due queue** (due cards first, capped new cards, honest "nothing due until…" states):

```mermaid
flowchart LR
    N["📝 Scope<br/>(intake grill)"] --> M["🧩 Propose<br/>(editable module plan)"] --> S["📚 Study<br/>notes · flashcards · quiz · tutor"]
    S -->|every answer + grade| P["📈 Learner profile"]
    P -.->|adapts difficulty & pacing| S
```

It **grills you on scope first** (never teaching before it understands the goal), then **generatively proposes** which modules to build, then generates the material you kept — and a built-in **tutor** answers questions at your level.

> [!NOTE]
> **Evidence-based, not "learning styles."** A `/deep-research` pass (25/25 claims verified) confirmed the popular "match instruction to a VARK learning style" idea is scientifically debunked (a negligible **d≈0.04** causal effect). So Learning mode adapts on what actually works: **retrieval practice** (quizzes + flashcards beat re-reading), **spaced repetition** (FSRS, via the `rs-fsrs` engine), and a per-skill **mastery estimate** (Bayesian Knowledge Tracing). The model is handed only a **bounded, numbers-only learner profile** (accuracy, pace, per-skill mastery) — never a personality label — and uses it to pitch difficulty and target weak skills. All of it stays in local SQLite.

## Architecture

The frontend is "pixels + intent"; all privileged work — the filesystem, the GitHub network, spawning `claude` — happens in Rust behind named Tauri commands. A single mutex-guarded SQLite connection serializes all state; background model runs stream progress over events and are panic-guarded.

```mermaid
flowchart LR
    subgraph FE["Frontend · React + TS + Tailwind (webview)"]
        UI["Panes and nav"]
        ZS["Zustand stores"]
    end
    subgraph BE["Backend · Rust (Tauri commands)"]
        SQL[("SQLite")]
        MP["ModelProvider"]
        GHC["GitHub client"]
    end
    UI <--> ZS
    ZS -->|"invoke()"| BE
    MP -.->|"claude -p · read-only tools"| CC["Claude Code"]
    GHC -.-> GH[("GitHub")]
    BE --> SQL
```

## Tech stack

| Layer | Choice |
|---|---|
| Shell | **Tauri 2** — one signed, notarized native `.app` |
| Backend | **Rust** — owns SQLite, the GitHub client, the model layer, and every write/commit |
| Frontend | **React 19 + TypeScript + Tailwind v4**, lightweight **Zustand** state |
| Database | **embedded SQLite** via `rusqlite` (bundled — no system dependency) |
| Model | **Claude Code** via `claude -p` (stream-json) behind a `ModelProvider` interface |
| Learning engine | **`rs-fsrs`** (FSRS spaced repetition) + an in-house Rust **Bayesian Knowledge Tracing** mastery model + **`pdf-extract`** (PDF upload ingest) + **react-markdown** (notes/tutor) |
| Voice capture | **`whisper-rs`** (whisper.cpp, Metal) running `ggml-large-v3-turbo-q5_0` locally + **`cpal`** microphone capture — auto-downloaded (sha256-verified) on first use |

## Security & trust boundaries

- **Read-only model** — planning/analysis pass only read/search tools (`Read, Grep, Glob, WebSearch, WebFetch`), plus an explicit `--disallowedTools` list as defense in depth. No code path lets the model write or commit your repo.
- **Secrets stay out of the tree** — the GitHub token lives only in the macOS Keychain; a `PreToolUse` git hook runs a deterministic secret scanner and blocks any commit that stages a credential (CI runs the same scan).
- **Nothing silent** — inferred changes are pending suggestions you approve; GitHub deletions/closes happen only after a confirmed, exact preview.
- **Hardened inputs** — prompt context is fenced as untrusted data (backtick-delimited + neutralized, length-bounded); clone scanning refuses symlinks that escape the clone; GitHub URLs are HTTPS/SSH-only; the HTTP client has connect/overall timeouts.

See [`SECURITY.md`](./SECURITY.md) for the full trust model and dependency audit.

## Quality — how it was built

Built one phase at a time, each phase verified against its "Done when" checks before the next began, and each followed by a **5-angle review** (two subagents per angle) with a critical/high fix pass.

After the 14th phase, a **5-round multi-agent council finale** hardened the codebase: each round dispatched **50 analyst subagents** → three specialist councils (correctness/security/data-integrity · product/UX/accessibility · architecture/maintainability/testing) → a **grand council** that picked bounded, test-backed, cross-council improvements and discarded the false positives. ~34 improvements shipped across the five rounds (CSP, symlink-escape closure, HTTP timeouts, crash-safe WAL + busy-timeout, prompt-injection hardening, gate poison-recovery, a full WCAG-AA accessibility pass, and more).

- ✅ **124 backend tests** (`cargo test --lib`) + **86 frontend tests** (Vitest) — all green.
- ✅ Every feature verified against rendered UI screenshots (a headless Playwright harness renders each pane with sample data), reviewed by a subagent before each phase shipped.
- ✅ CI on pinned `macos-15`: secrets gate → frontend build/test → `cargo test` → `cargo build`.

## Roadmap

All 14 original build phases are complete, and a follow-up **bug-fix & feature overhaul** (phases A–H) added Learning mode, the Easy↔Technical toggle, persistent chat memory, generative grill inputs, the Understand redesign, and the design export. The project is feature-complete for v0.1 and in continuous-improvement mode.

<details>
<summary><b>Full 14-phase build roadmap</b></summary>

| # | Phase | Status |
|---|-------|--------|
| 1 | Project scaffold & app shell | ✅ Done |
| 2 | Model provider & Claude availability | ✅ Done |
| 3 | Projects & GitHub connect | ✅ Done |
| 4 | Repo analysis & cold start | ✅ Done |
| 5 | Assessment engine & State pane | ✅ Done |
| 6 | The Understand hub | ✅ Done |
| 7 | Grill-me cards & detail coverage | ✅ Done |
| 8 | Two-way chat & structured proposals | ✅ Done |
| 9 | Decisions, suggestions & stack panes | ✅ Done |
| 10 | Feature inbox & plan regeneration | ✅ Done |
| 11 | GitHub sync out | ✅ Done |
| 12 | Visualization, first-run & polish | ✅ Done |
| 13 | Production hardening | ✅ Done |
| 14 | Learning-mode entry point | ✅ Done (now a full mode — below) |

</details>

<details>
<summary><b>Bug-fix &amp; feature overhaul (phases A–H)</b></summary>

| # | Phase | Status |
|---|-------|--------|
| A | Visual fixes + Stack "Why?" bug | ✅ Done |
| B | Easy↔Technical global toggle | ✅ Done |
| C | Plan readability (Markdown + collapsible phases) | ✅ Done |
| D | Persistent chat history + cross-chat memory | ✅ Done |
| E | Grill: model-generated input per question | ✅ Done |
| F | Understand redesign (filters + inline per-card chat) | ✅ Done |
| G | **Learning mode** (intake → propose → notes/flashcards/quiz/tutor + FSRS/BKT adaptive engine) | ✅ Done |
| H | Design export (self-contained snapshot to Desktop) | ✅ Done |

</details>

A new-engineer [`HANDOFF.md`](./HANDOFF.md) covers the architecture, schema, dev process, and the full change history in depth.

## Install & run

**Prerequisites:** macOS, [Claude Code](https://claude.com/claude-code) installed and signed in (the app drives the `claude` CLI), and for building from source: **Rust** (stable) + **Node 20+** with Xcode Command Line Tools (`xcode-select --install`).

### Build the app

```bash
npm install
npm run tauri build -- --bundles app   # produces "Review Helper.app" under src-tauri/target/release/bundle/macos/
```

The first build is slow (it compiles Tauri + SQLite from source); later builds are incremental. The bundle is ad-hoc signed — on first launch, right-click → **Open** to clear Gatekeeper. (Full Developer-ID notarization is documented in [`RELEASE.md`](./RELEASE.md) and needs an Apple signing identity.)

### Develop

```bash
npm run tauri dev                                  # run the app (native window) with HMR
npm test                                           # frontend tests (Vitest + Testing Library)
cargo test --manifest-path src-tauri/Cargo.toml --lib   # Rust tests (model, plan, sync, schema, security, learning)
npm run build                                       # production frontend build
npm run export:design                               # write a self-contained design snapshot (every screen, all 8 themes) to your Desktop
```

---

<div align="center">
<sub><b>Review Helper</b> · vibecode the right way</sub>
</div>
