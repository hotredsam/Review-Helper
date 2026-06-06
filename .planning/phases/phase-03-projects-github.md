# Phase 3 — Projects & GitHub connect
Status: not started
Goal: Attach a project four ways and keep a local clone cache.
Depends on: Phase 1

## Tasks
- [ ] **T1 GitHub OAuth device flow** — device-flow auth, token in the OS keychain, repo list. Done when: device flow lists your repos, the token survives restart, and sign-out clears it.
- [ ] **T2 Add-project — four paths** — import from GitHub, new blank, link-by-URL, create-repo-from-app. Done when: each path creates a project and create-from-app makes a real empty GitHub repo.
- [ ] **T3 Shallow-clone cache + refresh** — clone attached repos to the data dir; refresh re-pulls. Done when: attaching populates the cache and refresh pulls a new commit, on a private repo.
- [ ] **Tend Phase verification** — import one, link one by URL, create one, start one blank. Done when: all four appear in nav and the cloned ones have caches.

## Watch for (this phase)
- The GitHub token is a secret — keychain only, never in code, config, or logs. The commit hook will catch leaks, but don't rely on it.
- The model reads only the clone, never the user's working tree.
