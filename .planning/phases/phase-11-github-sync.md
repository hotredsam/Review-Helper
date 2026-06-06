# Phase 11 — GitHub sync out
Status: done
Goal: Push the planning package and phased issues, with consolidation and reconciliation behind a confirm gate.
Depends on: Phase 3, Phase 10

## Tasks
- [x] **T1 Generate the package** — render `.planning/` docs + a per-project CLAUDE.md from the plan/decisions/stack, with per-phase status markers and the resume header. Done when: generated docs reflect the current plan and carry status markers. (`sync::render_package`: PLAN.md w/ status-marker table + resume header, per-phase files, CLAUDE.md.)
- [x] **T2 Push to planning branch** — write the package to a `planning` branch via the GitHub API. Done when: pushing creates/updates the `planning` branch with the package. (`push_planning_branch`: ensure_branch from default head + Contents-API put per file, idempotent.)
- [x] **T3 Issue reconciliation (import side)** — adopt/relabel/close existing issues, applied only after a confirmed preview. Done when: a repo with old issues yields a preview and applying relabels/closes exactly what was shown. (`issues::reconcile` adopts by title, relabels by status, closes owned orphans, leaves foreign issues; `sync_issue_preview` → confirm → `sync_issue_apply`.)
- [x] **T4 Push to main + issue sync** — full change preview + confirm, one issue per phase matched by a stable marker (no dupes), status→issue state+label, remove legacy planning files only after confirm. Done when: push-to-main previews changes, writes docs + CLAUDE.md, opens/updates one issue per phase without dupes, removes legacy files only after confirmation, and is idempotent on re-push. (`push_main` puts docs to default branch + prunes stale phase files; issue sync matched by hidden `rh-phase` marker, records github_issue_number; SyncPanel gates apply behind Preview→Confirm.)

## Watch for (this phase)
- NEVER delete or close anything on GitHub without the confirmed preview.
- Keep the push idempotent — re-running must update, not duplicate, issues.
