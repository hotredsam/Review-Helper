## Why

<!-- The problem this solves. Link the issue if one exists. -->

## What changed

<!-- Bullet the user-visible and structural changes. -->

## Checks

- [ ] Both suites green (`npm test` + `cargo test --lib`)
- [ ] New behavior has a regression test that fails on the old code
- [ ] Destructive actions confirm via Modal; no silent failures introduced
- [ ] No model call holds the DB lock; no web tools on grounded paths
