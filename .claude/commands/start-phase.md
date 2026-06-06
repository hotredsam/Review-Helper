Run a single build phase of Review Helper, safely.

1. Run the `phase-check` skill first: confirm `.planning/PLAN.md` marks the right phase as current and that every earlier phase is actually done. If a phase marked `done` looks incomplete, stop and tell me — do not redo it.
2. Open only the current phase file in `.planning/phases/` plus `CLAUDE.md`. Do not load the other phase files.
3. Work the phase's tasks one at a time. After each task: run its "Done when" check, then commit (the commit hook runs the secrets scan automatically), then tick the task's checkbox.
4. When the phase-end verification passes, set the phase `Status` to `done`. Do not start the next phase without me.

Phase to run (optional): $ARGUMENTS
