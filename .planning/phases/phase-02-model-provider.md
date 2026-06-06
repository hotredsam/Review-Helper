# Phase 2 — Model provider layer & Claude availability
Status: done
Goal: A ModelProvider interface with a working `claude -p` adapter that streams to the UI and degrades gracefully.
Depends on: Phase 1

## Tasks
- [x] **T1 Define ModelProvider** — the trait, request/event types, and the read-only tool allow-list type; plus a fake provider for tests. Done when: a unit test drives the fake provider and receives ordered streamed events.
- [x] **T2 Claude Code adapter** — spawn `claude -p` with stream-json, the allow-list, append-system-prompt, and resume. Done when: a prompt streams tokens into a temp panel and a second turn resumes the session.
- [x] **T3 Settings provider config** — Claude default; off-by-default toggles for API-credit/overflow and a local-endpoint stub. Done when: selecting local yields the stub notice, Claude routes real calls, and settings persist.
- [x] **T4 Unavailability + debug panel** — "Claude not available" banner + retry, app stays read-only, debug panel shows last command/exit/stderr. Done when: with `claude` unavailable the banner shows, retry works once restored, and the debug panel reports it.

## Watch for (this phase)
- Build the unavailable/offline path now, not "later" — degraded paths are where vibecoded apps crash.
- Route ALL model use through this one interface. No ad-hoc `claude` calls scattered around the codebase.
