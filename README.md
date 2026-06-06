# Review Helper — build package

Repo: https://github.com/hotredsam/Review-Helper

This package is the planning + enforcement setup for a macOS desktop app that helps you vibecode the right way: analyze a project, score it, grill you until it's specified well enough to build, teach you as you go, and push a consistent plan + phased GitHub issues. You hand the plan to Claude Code and build it phase by phase.

**It's laid out to mirror the repo** — most of it drops straight in.

## Layout

```
CLAUDE.md                      -> repo root (standing rules for Claude Code)
.claude/
  settings.json                -> PreToolUse hook registration
  hooks/guard-commit.sh        -> blocks any git commit that stages a secret
  commands/start-phase.md      -> /start-phase slash command
scripts/
  scan_secrets.py              -> deterministic secrets scanner (used by the hook)
.planning/
  PROPOSAL.md                  -> why this exists, success criteria
  REQUIREMENTS.md              -> every decision from planning, structured
  ARCHITECTURE.md              -> stack, data model, LLM + GitHub designs, enforcement
  schema.sql                   -> tested SQLite schema (use as-is)
  PLAN.md                      -> the plan index + phase status table
  DECISIONS.md                 -> seeded ADR record
  phases/phase-01..14-*.md     -> one file per phase (Claude loads only the current one)
skills/                        -> install separately (see below); not part of the repo
  big-picture/  (SKILL.md + scan.py)
  phase-check/  (SKILL.md)
  secrets-gate/ (SKILL.md + scan.py)
```

## Install

1. Copy everything **except `skills/`** into the repo, preserving paths: `CLAUDE.md` at the root, and the `.claude/`, `scripts/`, and `.planning/` directories. Then `chmod +x .claude/hooks/guard-commit.sh scripts/scan_secrets.py`.
2. Install the three skills where Claude Code finds them — globally at `~/.claude/skills/<name>/` (each with its `SKILL.md` and any `scan.py`), or per-repo at `.claude/skills/<name>/`. `chmod +x` the `scan.py` files.
3. Requires `python3` on PATH (the hook and skills use it; pure standard library).

## Kick off the build

In Claude Code, in the repo:

> Read `.planning/PLAN.md` and `CLAUDE.md`. Confirm the current state, then run `/start-phase 1` — one task at a time, stopping after each for me to review.

`/start-phase` runs `phase-check` first (so finished phases are never rebuilt), then works one phase. The commit hook runs the secrets scan on every commit automatically. Run the `big-picture` skill anytime to score the repo.

## Pushing this to GitHub

I couldn't push for you — pushing to `hotredsam/Review-Helper` needs your GitHub credentials, which I don't have in this environment (I could read the public repo, but not write). The repo currently has just a README on `main`, so this drops in cleanly. From a machine that's authenticated to your GitHub:

```bash
git clone https://github.com/hotredsam/Review-Helper.git
cd Review-Helper

# copy the package contents in (everything except skills/), preserving paths:
#   CLAUDE.md, .claude/, scripts/, .planning/
# (from wherever you unpacked this package)

chmod +x .claude/hooks/guard-commit.sh scripts/scan_secrets.py

git checkout -b planning          # optional: stage on a branch first
git add -A
git commit -m "Add planning package + build-time enforcement"
git push -u origin planning       # open a PR, or push straight to main:
# git checkout main && git merge planning && git push
```

Then install the skills (step 2 above) and kick off the build.
