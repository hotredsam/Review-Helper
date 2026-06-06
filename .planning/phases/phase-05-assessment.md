# Phase 5 — Assessment engine & State pane
Status: done
Goal: Score a project 0–100 across the six dimensions plus a Production Readiness scorecard and hygiene.
Depends on: Phase 4

## Tasks
- [x] **T1 Assessment engine** — run `skills/big-picture/scan.py` for deterministic facts, then have the model score the six dimensions + production-readiness + hygiene from them → structured JSON in `assessments`. Done when: running on the sample repo returns all scores 0–100 and a cleanup list, grounded in the scan's numbers.
- [x] **T2 State pane** — per-dimension % with color tint, overall, top-3 fixes, the Production Readiness scorecard, the hygiene cleanup list. Done when: the pane renders a real assessment with numbers, colors, fixes, and cleanup.
- [x] **Tend Phase verification** — assess an imported repo end-to-end. Done when: six scores + overall + production readiness + hygiene all persist.

## Watch for (this phase)
- Ground every score in the scan output, not impressions.
- Numbers AND color in the app; the `big-picture` skill (terminal) stays numbers-only on the same rubric.
