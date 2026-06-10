# Contributing

This is a personal tool built in public; issues and PRs are welcome but scope is owner-driven.

- **Bugs**: use the bug template; the fastest path to a fix is a failing reproduction.
- **PRs**: keep them phase-sized and atomic. Both test suites must pass, and new behavior needs a regression test that fails on the old code. The PR template's checklist is the house style in four lines.
- **Architecture**: read `HANDOFF.md` first — it explains why the model is read-only, why nothing writes silently, and how the phased-plan workflow works. `AGENTS.md` is the condensed version for coding agents.
