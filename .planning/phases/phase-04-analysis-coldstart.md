# Phase 4 — Repo analysis & cold start
Status: not started
Goal: Turn an attached repo into a solid first plan; seed a blank project from a prompt.
Depends on: Phase 2, Phase 3

## Tasks
- [x] **T1 Context bundle** — assemble ProjectContext (plan + decisions + answers + stack) for model calls. Done when: a unit test builds a bundle from seeded rows and from an empty project.
- [ ] **T2 Analysis + first plan** — read-only analysis of the clone → first plan, behind a loading indicator (no "draft" label). Done when: importing a sample repo shows loading then a sensible, persisted plan.
- [ ] **T3 Blank-project kickoff** — "What is this repo?" prompt seeds a blank project. Done when: answering it produces a starting plan and populates panes.
- [ ] **T4 Ingest existing docs** — fold any pre-existing planning files into the first plan (content absorbed, not lost). Done when: a repo with a stray PLANNING.md yields a plan reflecting its content.

## Watch for (this phase)
- Analysis is read-only — pass only read/search tools, never Edit/Write/Bash.
- Don't fabricate plan content the repo and answers don't support. A confident wrong plan is worse than a thin honest one.
