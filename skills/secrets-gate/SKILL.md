---
name: secrets-gate
description: "Pre-commit scan of a diff for hardcoded keys, credentials, tokens, and client-side secret exposure before anything is committed. Use before committing, when reviewing a change for leaked secrets, or when the user mentions \"secrets gate\". Block the commit if anything is found."
---

Stop secrets from being committed. The model doesn't think adversarially by default — so this gate leads with a deterministic scan, then adds judgment.

## Step 1 — run the scanner

```
python3 ~/.claude/skills/secrets-gate/scan.py            # scans staged files (git diff --cached); falls back to the working diff
python3 ~/.claude/skills/secrets-gate/scan.py FILE ...   # or scan specific files
```
(System-wide install path; per-repo it's `.claude/skills/secrets-gate/scan.py`.)

It flags known key formats (OpenAI/Anthropic `sk-…`, GitHub `ghp_…`, AWS `AKIA…`, Google `AIza…`, Slack `xox…`), private-key blocks, credentialed URLs (`proto://<user>:<pass>@host`), hardcoded `password/secret/api_key = "…"` assignments, and committed `.env` / `*.pem` / `*.key` / credential files. It ignores obvious placeholders (`your-token`, `<...>`, `example`, `changeme`) and env-var usage. Exit code 0 = clean, 1 = findings. (This is the same scanner the build's commit hook runs.)

## Step 2 — add judgment

The scanner catches known patterns; you catch the rest. Also check for:

- High-entropy strings that don't match a known prefix but look like a secret.
- Secrets headed for a **frontend/client bundle** or a public env var — anything shipped to the browser/app binary that should be server-side only.
- A newly added dependency that handles auth/crypto/network — worth a second look.

## How to report

- **Clean:** one line — "secrets-gate: clean, safe to commit."
- **Findings:** list each as `file:line — what it is — why it's a problem`, then **block** — tell the user not to commit until it's resolved, and give the fix (move to keychain/env, git-ignore the file, switch to a server-side call). Do not commit over an unresolved finding.

Be precise about location. A finding with a file and line is actionable; "there might be secrets somewhere" is not.
