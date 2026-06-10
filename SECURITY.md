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
- **Prompt-injection defense-in-depth.** The real controls are structural: the
  model is read-only (no write/commit), untrusted clones are scanned without
  following symlinks out of the clone, and generated cards/questions/suggestions
  are structurally validated + length-bounded before they are persisted. On top
  of that, the `ProjectContext` bundle marks recorded state as untrusted DATA and
  backtick-delimits each value — an advisory behavioral hint, not a hard control.

## Trust model for imported content (deliberate decision, 2026-06-09)

Imported repositories and uploaded documents are treated as **trusted input**.
Model calls that read them keep WebSearch/WebFetch enabled by design — the
owner accepted the prompt-injection/exfiltration tradeoff for a local,
single-user tool whose normal inputs are his own repos and study materials.
The exposed surface to be aware of: **Assess** is the one feature where
third-party repositories are the expected input. If you import a repo you do
not trust, know that its files are read by a model that can fetch URLs; a
malicious README could exfiltrate anything else in that call's context.
Revisit this decision before any multi-user or hosted distribution.

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
