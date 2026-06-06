# Phase 11 — GitHub sync out
Status: not started
Goal: Push the planning package and phased issues, with consolidation and reconciliation behind a confirm gate.
Depends on: Phase 3, Phase 10

## Tasks
- [ ] **T1 Generate the package** — render `.planning/` docs + a per-project CLAUDE.md from the plan/decisions/stack, with per-phase status markers and the resume header. Done when: generated docs reflect the current plan and carry status markers.
- [ ] **T2 Push to planning branch** — write the package to a `planning` branch via the GitHub API. Done when: pushing creates/updates the `planning` branch with the package.
- [ ] **T3 Issue reconciliation (import side)** — adopt/relabel/close existing issues, applied only after a confirmed preview. Done when: a repo with old issues yields a preview and applying relabels/closes exactly what was shown.
- [ ] **T4 Push to main + issue sync** — full change preview + confirm, one issue per phase matched by a stable marker (no dupes), status→issue state+label, remove legacy planning files only after confirm. Done when: push-to-main previews changes, writes docs + CLAUDE.md, opens/updates one issue per phase without dupes, removes legacy files only after confirmation, and is idempotent on re-push.

## Watch for (this phase)
- NEVER delete or close anything on GitHub without the confirmed preview.
- Keep the push idempotent — re-running must update, not duplicate, issues.
