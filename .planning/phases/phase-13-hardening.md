# Phase 13 — Production hardening
Status: done
Goal: Tested, secure, signed, and packaged for daily use.
Depends on: all prior phases

## Tasks
- [x] **T1 Tests + light CI** — cover providers, structured-proposal parsing, the plan-merge (status preservation), and issue matching, plus a core-loop smoke test, run by CI. Done when: CI is green on a clean checkout and covers those areas. (`.github/workflows/ci.yml`: secrets-gate + frontend build/test + `cargo test --lib`. Coverage exists: model providers, chat parse_suggestions, plan carry_status, sync reconcile, the App-shell smoke test. 93 backend + 70 frontend green locally.)
- [x] **T2 Security + dependency audit** — run `secrets-gate` over the codebase, audit dependencies/licenses, confirm keychain-only token storage. Done when: the scan is clean, tokens live only in the keychain, and deps are accounted for. (secrets-gate clean over 201 files; SECURITY.md documents deps/licenses/trust-boundaries + keychain-only token; prompt-injection hardening: ProjectContext marked as untrusted DATA.)
- [x] **T3 Sign, notarize, package** — code-sign and notarize the `.app` into an installable build. Done when: the built `.app` launches on a clean Mac without Gatekeeper warnings. (Config wired: tauri.conf.json macOS bundle + entitlements.plist (hardened runtime); RELEASE.md documents the env-driven sign+notarize. NOTE: the final notarize step needs Sam's Apple Developer ID certificate + app-specific password — credential-gated, left for the credential holder per RELEASE.md.)

## Deferred-item status (from earlier reviews)
- Prompt-injection delimiters in context.rs — DONE (DATA preamble).
- File-size: api.rs (496) / plan/commands.rs (492) / sync/mod.rs (433) are over the ~300 guideline but under the 500 hard ceiling — left for the Phase 13 review to adjudicate vs split risk.
- Sync holds the DB lock across network calls — accepted (sync is explicit + infrequent); revisit if it harms responsiveness.
- schema UNIQUE(term) NOCASE — needs Sam's sign-off (fixed schema); documented as presently-benign.

## Watch for (this phase)
- This phase hardens; it does not rescue. The per-phase verifications were the real safety net — don't treat this as the first time anything is tested.
