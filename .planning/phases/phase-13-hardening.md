# Phase 13 — Production hardening
Status: done
Goal: Tested, secure, signed, and packaged for daily use.
Depends on: all prior phases

## Tasks
- [x] **T1 Tests + light CI** — cover providers, structured-proposal parsing, the plan-merge (status preservation), and issue matching, plus a core-loop smoke test, run by CI. Done when: CI is green on a clean checkout and covers those areas. (`.github/workflows/ci.yml`: secrets-gate + frontend build/test + `cargo test --lib` + a `cargo build` config-validation step. Coverage exists: model providers, chat parse_suggestions, plan carry_status, sync reconcile. The only app-level test is the `app_info()` build-health unit test — NOT a UI/bridge smoke test; a full app-shell/core-loop integration smoke was deferred (would need the full Tauri app in CI). 93 backend + 72 frontend green locally.)
- [x] **T2 Security + dependency audit** — run `secrets-gate` over the codebase, audit dependencies/licenses, confirm keychain-only token storage. Done when: the scan is clean, tokens live only in the keychain, and deps are accounted for. (secrets-gate clean over all tracked files; SECURITY.md documents deps/licenses/trust-boundaries + keychain-only token; prompt-injection hardening: ProjectContext marked as untrusted DATA with backtick-delimited values.)
- [x] **T3a Signing config + docs** — tauri.conf.json macOS bundle + entitlements.plist (hardened runtime) + RELEASE.md documenting the two-step code-sign (Tauri) then manual `notarytool` notarize + staple. CI compiles the Tauri backend.
- [ ] **T3b Build, notarize, verify on a clean Mac** — BLOCKED, release prerequisite owned by the credential holder: run RELEASE.md with Sam's Apple Developer ID cert + app-specific password, then verify `spctl -a -vvv` says "accepted, source=Notarized". Could not be run here (no Apple credentials). Verify minimal entitlements during this step.

## Deferred-item status (from earlier reviews)
- Prompt-injection — DONE: DATA preamble + backtick-delimited values + an injection test in context.rs.
- File-size: api.rs (496) / plan/commands.rs (492) / sync/mod.rs (433) — reviewed and ACCEPTED as-is. Each is single-responsibility (GitHub API client / plan-generation commands / package render + sync) and under the 500-line hard ceiling; splitting now would fragment cohesive domains. Revisit only if refactor boundaries emerge.
- Sync holds the DB lock across network calls — accepted (sync is explicit + infrequent); revisit if it harms responsiveness.
- schema UNIQUE(term) NOCASE — needs Sam's sign-off (fixed schema); documented as presently-benign.

## Watch for (this phase)
- This phase hardens; it does not rescue. The per-phase verifications were the real safety net — don't treat this as the first time anything is tested.
