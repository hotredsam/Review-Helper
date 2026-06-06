#!/usr/bin/env python3
"""big-picture metrics. Collects deterministic facts about a repo so scoring is
grounded in measurements, not impressions. Prints JSON. The model reads this and
assigns the 0-100 dimension scores.
Usage: python3 scan.py [REPO_DIR]   (defaults to cwd)
"""
import os, sys, json, subprocess, re

SRC_EXT = {".py",".rs",".ts",".tsx",".js",".jsx",".go",".java",".rb",".c",".cpp",".h",".hpp",".swift",".kt",".sql",".css",".scss",".sh"}
SKIP_DIR = {".git","node_modules","target","dist","build",".venv","venv","__pycache__",".next",".cache","vendor"}
SECRET = re.compile(r"\bsk-[A-Za-z0-9_-]{20,}|\bgh[posru]_[A-Za-z0-9]{30,}|\bAKIA[0-9A-Z]{16}\b|-----BEGIN .*PRIVATE KEY-----")
TODO = re.compile(r"\b(TODO|FIXME|HACK|XXX)\b")

def walk(root):
    # followlinks=False: don't descend into symlinked directories.
    for dp, dns, fns in os.walk(root, followlinks=False):
        dns[:] = [d for d in dns if d not in SKIP_DIR]
        for fn in fns:
            yield os.path.join(dp, fn)

def main(argv):
    root = argv[1] if len(argv) > 1 else "."
    files = list(walk(root))
    rel = lambda p: os.path.relpath(p, root)
    src = [f for f in files if os.path.splitext(f)[1] in SRC_EXT]
    big, total_src_lines, todos, secret_hits = [], 0, 0, []
    for f in src:
        # Never read symlinked files: in an untrusted clone a tracked symlink
        # could point outside the repo (e.g. ~/.ssh/id_rsa) and leak its content.
        if os.path.islink(f):
            continue
        try:
            lines = open(f, errors="ignore").read().splitlines()
        except Exception:
            continue
        n = len(lines); total_src_lines += n
        if n > 300: big.append({"file": rel(f), "lines": n, "severity": "hard" if n > 500 else "warn"})
        for ln in lines:
            if TODO.search(ln): todos += 1
            if SECRET.search(ln): secret_hits.append(rel(f))
    names = {rel(f).lower() for f in files}
    has = lambda pred: any(pred(n) for n in names)
    try:
        commits = int(subprocess.run(["git","-C",root,"rev-list","--count","HEAD"],
                      capture_output=True, text=True).stdout.strip() or 0)
    except Exception:
        commits = 0
    metrics = {
        "files_total": len(files),
        "source_files": len(src),
        "source_lines": total_src_lines,
        "files_over_300_lines": big,
        "todo_fixme_count": todos,
        "secret_pattern_hits": sorted(set(secret_hits)),
        "git_commits": commits,
        "has_readme": has(lambda n: n.startswith("readme")),
        "has_claude_md": "claude.md" in names,
        "has_planning_dir": any(n.startswith(".planning/") or n.startswith("planning/") for n in names),
        "has_tests": has(lambda n: "/test" in "/"+n or n.startswith("test") or "spec." in n or "_test." in n or ".test." in n),
        "has_ci": any(n.startswith(".github/workflows/") for n in names),
        "has_gitignore": ".gitignore" in names,
    }
    print(json.dumps(metrics, indent=2))
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))
