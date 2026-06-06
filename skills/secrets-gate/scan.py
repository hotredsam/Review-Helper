#!/usr/bin/env python3
"""secrets-gate scanner. Deterministic pre-commit scan for leaked credentials.
Usage:
  python3 scan.py                 # scans `git diff --cached` (staged); falls back to working diff
  python3 scan.py FILE [FILE...]  # scans specific files
Exit code 0 = clean, 1 = findings (so a git hook can block).
"""
import re, sys, subprocess, os

PATTERNS = [
    ("OpenAI key",        re.compile(r"\bsk-[A-Za-z0-9_-]{20,}")),
    ("Anthropic key",     re.compile(r"\bsk-ant-[A-Za-z0-9_-]{20,}")),
    ("GitHub token",      re.compile(r"\bgh[posru]_[A-Za-z0-9]{30,}")),
    ("AWS access key id", re.compile(r"\bAKIA[0-9A-Z]{16}\b")),
    ("Google API key",    re.compile(r"\bAIza[0-9A-Za-z_-]{35}\b")),
    ("Slack token",       re.compile(r"\bxox[baprs]-[A-Za-z0-9-]{10,}")),
    ("Private key block", re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----")),
    ("Credentialed URL",  re.compile(r"[a-z][a-z0-9+.-]*://[^/\s:@]+:[^/\s:@]+@")),
    ("Hardcoded secret assignment",
        re.compile(r"(?i)\b(?:password|passwd|secret|api[_-]?key|access[_-]?key|client[_-]?secret|auth[_-]?token)\b"
                   r"\s*[:=]\s*[\"'][^\"'\s]{8,}[\"']")),
]
# Reduce false positives: obvious placeholders are ignored.
PLACEHOLDER = re.compile(r"(?i)(your[_-]?|example|placeholder|dummy|xxx+|<[^>]+>|changeme|redacted|\.\.\.)")

def scan_text(label, text):
    findings = []
    for i, line in enumerate(text.splitlines(), 1):
        for name, pat in PATTERNS:
            m = pat.search(line)
            if m and not PLACEHOLDER.search(m.group(0)):
                findings.append((label, i, name, line.strip()[:80]))
    return findings

def staged_files():
    try:
        out = subprocess.run(["git","diff","--cached","--name-only","--diff-filter=ACM"],
                             capture_output=True, text=True)
        files = [f for f in out.stdout.split("\n") if f.strip()]
        if files: return files, "staged"
        out = subprocess.run(["git","diff","--name-only","--diff-filter=ACM"],
                             capture_output=True, text=True)
        return [f for f in out.stdout.split("\n") if f.strip()], "working"
    except Exception:
        return [], "none"

def main(argv):
    if len(argv) > 1:
        files, mode = argv[1:], "explicit"
    else:
        files, mode = staged_files()
    if not files:
        print("secrets-gate: nothing to scan (%s)." % mode); return 0
    # .env / credential files committed
    risky_names = re.compile(r"(^|/)(\.env(\..+)?|credentials|id_rsa|.+\.pem|.+\.key)$")
    all_findings = []
    for f in files:
        if risky_names.search(f):
            all_findings.append((f, 0, "Credential file committed", f))
        try:
            with open(f, "r", errors="ignore") as fh:
                all_findings += scan_text(f, fh.read())
        except (IsADirectoryError, FileNotFoundError):
            pass
    if not all_findings:
        print("secrets-gate: clean (%s, %d file(s)). Safe to commit." % (mode, len(files)))
        return 0
    print("secrets-gate: BLOCKED — %d finding(s):" % len(all_findings))
    for fname, line, name, snippet in all_findings:
        loc = "%s:%d" % (fname, line) if line else fname
        print("  %-40s %-26s %s" % (loc, name, snippet))
    print("Resolve before committing: move secrets to the keychain/env, git-ignore the file, or use a server-side call.")
    return 1

if __name__ == "__main__":
    sys.exit(main(sys.argv))
