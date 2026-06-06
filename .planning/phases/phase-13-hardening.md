# Phase 13 — Production hardening
Status: not started
Goal: Tested, secure, signed, and packaged for daily use.
Depends on: all prior phases

## Tasks
- [ ] **T1 Tests + light CI** — cover providers, structured-proposal parsing, the plan-merge (status preservation), and issue matching, plus a core-loop smoke test, run by CI. Done when: CI is green on a clean checkout and covers those areas.
- [ ] **T2 Security + dependency audit** — run `secrets-gate` over the codebase, audit dependencies/licenses, confirm keychain-only token storage. Done when: the scan is clean, tokens live only in the keychain, and deps are accounted for.
- [ ] **T3 Sign, notarize, package** — code-sign and notarize the `.app` into an installable build. Done when: the built `.app` launches on a clean Mac without Gatekeeper warnings.

## Watch for (this phase)
- This phase hardens; it does not rescue. The per-phase verifications were the real safety net — don't treat this as the first time anything is tested.
