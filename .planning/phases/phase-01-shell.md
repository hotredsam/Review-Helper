# Phase 1 — Project scaffold & app shell
Status: not started
Goal: A Tauri app that boots to a themed, navigable shell with the database wired and clean empty states.
Depends on: nothing

## Tasks
- [x] **T1 Scaffold app** — stand up Tauri + React + TS + Tailwind with one frontend→Rust command round-trip. Done when: `npm run tauri dev` opens a native window showing a value fetched from a Rust command.
- [x] **T2 Theme tokens + 4 themes** — design tokens as CSS variables, four themes, persisted. Done when: switching theme restyles the whole window with no hardcoded colors and survives a restart.
- [ ] **T3 SQLite + migrations** — wire SQLite, run migrations from `.planning/schema.sql`, add `projects` CRUD. Done when: migrations are idempotent and creating/listing a project persists across restart.
- [ ] **T4 Hamburger nav + empty states** — left project switcher and polished empty-state screens. Done when: with one empty project every pane region shows a clean (not ugly) empty state and nav switches.
- [ ] **Tend Phase verification** — create two projects, switch, change theme, restart. Done when: state and theme persist and empty states render.

## Watch for (this phase)
- Wire the DB layer before any pane that saves to it. A pane whose "save" silently no-ops because the DB was never connected is the classic vibecoding trap.
- No hardcoded colors anywhere; keep files small and single-responsibility from the start.
