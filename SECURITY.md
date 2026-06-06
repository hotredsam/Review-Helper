# Security & dependency audit (Phase 13)

## Secrets
- The GitHub token lives **only in the macOS Keychain** (`keyring` crate,
  `apple-native`). It is never written to code, config, or the client bundle.
- `scripts/scan_secrets.py` (the secrets-gate) is enforced two ways: a
  `PreToolUse` commit hook and the CI job (`.github/workflows/ci.yml`). It runs
  clean over the whole tree.

## Trust boundaries
- **The model is read-only against any user source.** Planning / grill / chat /
  assessment calls pass only read/search tools; the app performs every write and
  commit itself.
- **The frontend never does privileged work.** Filesystem, GitHub API, and the
  `claude` subprocess all run in Rust behind named Tauri commands.
- **Nothing reaches the record or GitHub silently.** Chat-inferred changes are
  pending suggestions the user approves; GitHub closes/deletions happen only
  after a confirmed preview (`SyncPanel`), and the sync is idempotent.
- **Prompt-injection defense-in-depth.** Untrusted clones are scanned without
  following symlinks out of the clone; the `ProjectContext` bundle is marked as
  DATA (not instructions); generated cards/questions/suggestions are validated
  and length-bounded before they are persisted.

## Dependencies (all permissive licenses, justified)
Rust (`src-tauri/Cargo.toml`):
- `tauri`, `tauri-plugin-opener` — the app shell (MIT/Apache-2.0).
- `serde`, `serde_json` — (de)serialization (MIT/Apache-2.0).
- `rusqlite` (`bundled`) — embedded SQLite; bundled so there's no system dep (MIT).
- `keyring` (`apple-native`) — Keychain token storage (MIT/Apache-2.0).
- `reqwest` (`blocking`, `json`) — GitHub REST client (MIT/Apache-2.0).
- No base64 dependency — a 15-line encoder is inlined in `github/api.rs`.

Frontend (`package.json`):
- `react`, `react-dom` (MIT); `tailwindcss` + `@tailwindcss/vite` (MIT);
  `lucide-react` (ISC); `zustand` (MIT); `@tauri-apps/api` + plugins (MIT/Apache-2.0).

Run `cargo audit` and `npm audit` for advisory CVE checks before a release.
