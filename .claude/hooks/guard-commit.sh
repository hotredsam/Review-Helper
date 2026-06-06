#!/usr/bin/env bash
# PreToolUse hook (matcher: Bash). If the command is a git commit, run the
# secrets scanner over staged files. Exit 2 blocks the commit; exit 0 allows.
input="$(cat)"
cmd="$(printf '%s' "$input" | python3 -c 'import sys,json
try:
    print(json.load(sys.stdin).get("tool_input",{}).get("command",""))
except Exception:
    print("")' 2>/dev/null)"
case "$cmd" in
  *"git commit"*)
    ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
    if ! python3 "$ROOT/scripts/scan_secrets.py" >&2; then
      echo "secrets-gate hook: commit BLOCKED — resolve the findings above before committing." >&2
      exit 2
    fi
    ;;
esac
exit 0
