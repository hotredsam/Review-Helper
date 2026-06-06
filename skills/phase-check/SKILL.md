---
name: phase-check
description: "Before building a phase from a phased plan, verify the plan marks the right phase as current, confirm prior phases are actually done, and refuse to redo finished work. Can also update or close GitHub issues that should be complete but aren't marked so. Use before starting or resuming a build phase, or when the user mentions \"phase check\"."
---

Before any code gets written for a phase, check the plan against reality so nothing finished gets rebuilt.

Read `.planning/PLAN.md` — the index, with the "Current state" header and the phase status table. The current phase is the first one not marked `done`. Then:

1. **Confirm the current phase** from the table and header. If the user asked to start a phase that isn't the current one, say so and confirm before proceeding.
2. **Verify prior phases are actually done.** For each phase marked `done`, open its file in `.planning/phases/` and spot-check that its work exists in the repo (its tasks' "Done when" checks would still pass, its features/files are present). If a `done` phase looks incomplete, stop and tell the user — do not silently redo it; either the status or the code is wrong, and they decide which.
3. **State the target.** Name the phase and the first unchecked task you're about to start, and its "Done when" check.
4. **Never rebuild a done phase.** If a request would redo finished work, refuse and point to the status.

Then read only the current phase file plus `CLAUDE.md` and begin — don't load the other phase files.

## GitHub issue sync (optional)

If the repo has phase issues, reconcile them with real status:

- A phase whose work is actually complete but whose issue is still open/unlabeled-done → update or close that issue to match.
- Match issues to phases by their stable marker; never create duplicates.
- One-way only: the plan is the source of truth. Don't infer plan changes from issue edits.

Report what you changed (or that nothing needed changing) before building.
