# What changed from the first package, and why

I reviewed the first version against the vibecoding-failure research and found eight weaknesses. Each fix is below. Several were caught by holding the package to the *same* standard it's meant to enforce.

### 1. The plan was one 555-line file → split into an index + per-phase files
Claude Code re-reads the plan on every task. A 555-line file re-read each time is exactly the context-bloat the research warns about — the planning artifact was itself violating context hygiene. Now `PLAN.md` is a slim index with a phase status table, and each phase is its own ~20-line file in `phases/`. Claude loads only the phase it's working on.

### 2. All discipline was prose in CLAUDE.md → the riskiest rules are now enforced by a hook
The research is blunt that CLAUDE.md instructions get ignored even with MUST/caps, and that deterministic behavior belongs in settings/hooks, not prose — and that the model doesn't think adversarially about security. The v1 secrets rule was a prose hope. Now a `PreToolUse` hook runs a secrets scanner before every `git commit` and blocks the commit (exit 2) if a credential is staged. Tested end-to-end: blocks a staged secret, allows a clean commit, ignores non-commit commands.

### 3. The skills were model-judgment-only → backed by tested scripts
`secrets-gate` and `big-picture` relied entirely on the model "looking." Both now lead with a deterministic `scan.py` that produces facts (the secrets scanner; a repo-metrics collector that flags >300/500-line files, missing tests/CI, secret hits, git depth), and the model adds judgment on top. Both scripts are tested — the secrets scanner catches planted keys and ignores placeholders/env-vars; the metrics scanner correctly reads a near-empty repo and flags a 620-line file.

### 4. The SQLite schema was unverified prose → shipped as a tested `schema.sql`
A prose column-list invites a divergent, possibly-broken schema. `ARCHITECTURE.md` now points to `.planning/schema.sql`, which I compiled and exercised: 13 tables, `CHECK` enums reject bad values, FK cascades work. Claude Code uses it as-is.

### 5. Used the (outdated) build-plan task format → leaner per-phase format with checkboxes
Dropped the verbose What/Why/Scope/Verification/Commit-when scaffolding. Each task is now one line — `- [ ] **Tn Name** — what. Done when: <check>.` — with per-task checkboxes for precise resume and a phase-end verification.

### 6. Context hygiene, the unhappy path, and test-as-you-go were under-specified → baked into CLAUDE.md and every phase
The research hammers three things v1 glossed: `/clear` between tasks, prompting for the failure paths as you build (not "later"), and testing component-by-component (the "save button does nothing because a lower layer broke in step one" story — v1 deferred most testing to Phase 13). CLAUDE.md now has explicit sections for each, and every phase file carries a "Watch for" note encoding the relevant lesson.

### 7. Placeholder name "Keel", no repo wiring → renamed to Review Helper and wired to the repo
Renamed throughout and pointed at `hotredsam/Review-Helper`. The package mirrors the repo layout so it drops in. (I read the repo — currently just a README on `main` — but can't push without your credentials; push steps are in `README.md`.)

### 8. The decisions record was described but empty → seeded `DECISIONS.md`
Ten ADR-style entries capture the real decisions (Tauri, `claude -p` behind a provider, read-only model, 0–100 assessment, plan-as-source-of-truth, gated push, approve-don't-auto-write, the Understand-hub spine, hybrid grilling, enforcement-by-mechanism) — so the record starts populated and demonstrates the format.

## Tests run this round

- `schema.sql` compiled and exercised (13 tables; CHECK rejects bad enums; FK cascade verified).
- `scan_secrets.py` vs. a secrets-laden file (caught 4/4: OpenAI key, credentialed URL, hardcoded password, AWS key) and a clean file (passed, ignored placeholder + env usage).
- `scan_repo.py` vs. the real repo (near-empty, read correctly) and a messy fixture (flagged the 620-line file as hard, found the planted secret, saw no tests/CI).
- The commit-guard hook end-to-end: blocks a staged secret (exit 2), allows the clean version (exit 0), ignores `npm test` (exit 0).
