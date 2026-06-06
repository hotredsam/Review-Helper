---
name: big-picture
description: "Score how well a project is set up for vibe coding — 0-100 across architecture, modularity, context hygiene, security, git discipline, and workflow, plus a production-readiness read and a hygiene cleanup list. Use when the user asks how their repo or plan is doing, whether it's well-organized, whether it's production-ready, what to fix first, or mentions \"big picture\"."
---

Assess this project's big picture. This is the counterpart to grilling: grill-me decides *what to build*; big-picture judges *how well what exists is set up*. Score honestly — a low number with a clear fix beats a generous one.

## Step 1 — get the facts (don't eyeball it)

Run the bundled scanner first so scores are grounded in measurements, not impressions:

```
python3 ~/.claude/skills/big-picture/scan.py [REPO_DIR]
```
(That's the bundled scanner at the system-wide install path. If you installed the skill per-repo instead, it's at `.claude/skills/big-picture/scan.py`.)

It returns JSON: file counts, source lines, any files over 300/500 lines, TODO/FIXME count, secret-pattern hits, git commit count, and presence of README / CLAUDE.md / `.planning/` / tests / CI / `.gitignore`. Read it before scoring. Then explore read-only (read/search) to judge the things a script can't — separation of concerns, whether the plan matches the code, real security posture. Never edit, write, or commit while assessing.

## Step 2 — report (terminal-friendly: numbers, not colors)

```
BIG PICTURE — <project name>

Vibecoding dimensions (0-100, 100 = best)
  Architecture      <n>   <reason, tied to something you saw>
  Modularity        <n>   <reason; cite the scan's large-file list>
  Context hygiene   <n>   <reason; note CLAUDE.md / .planning presence>
  Security/secrets  <n>   <reason; note any secret-pattern hits>
  Git discipline    <n>   <reason; note commit count>
  Workflow          <n>   <reason; does code match the plan?>
  ---------------------------------
  Overall           <n>

Production readiness (0-100)
  Tests             <n>   (scan: has_tests)
  Error handling    <n>
  Secrets handling  <n>   (scan: secret_pattern_hits)
  Build + CI        <n>   (scan: has_ci)
  Dependencies      <n>
  Docs              <n>   (scan: has_readme)
  ---------------------------------
  Overall           <n>

Top 3 fixes
  1. <highest-leverage fix>
  2. <next>
  3. <next>

Hygiene / cleanup
  - <orphaned/unused file, dead code, unused dependency, oversized file>
  - <...>   (or "clean — nothing obvious")
```

## What each dimension means

- **Architecture** — a real, intended structure (core vs shared vs disposable), concerns separated, not accreted.
- **Modularity** — small, single-responsibility files. The scan flags files over ~300 lines (hard over ~500); duplicate utilities and tangled state lower this.
- **Context hygiene** — is the code scoped so an agent can work a task without holding everything, and are there a `CLAUDE.md` / planning docs keeping it on rails?
- **Security/secrets** — hardcoded keys/credentials, client-side secret exposure, missing auth on data access. Costliest failure mode — weight heavily, never soften. Any `secret_pattern_hits` caps this dimension low until resolved.
- **Git discipline** — meaningful history, no giant untracked state, no signs working code gets overwritten.
- **Workflow** — is there a written plan the work follows, and does the current state match it? If there's a `.planning/PLAN.md`, check the code against its phase status (pairs with phase-check).

## Rules

- Read-only. Never edit, write, or commit while assessing.
- Every score gets a one-line reason tied to the scan output or something you actually saw — not a vibe.
- Be specific in fixes: name the file or pattern, not "improve architecture."
