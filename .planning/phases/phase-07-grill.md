# Phase 7 — Grill-me cards & detail coverage
Status: not started
Goal: Repo-specific questions with recommended answers, five at a time, with a depth slider and a "Done grilling" signal.
Depends on: Phase 4

## Tasks
- [x] **T1 Question generation** — bank supplies the topic/dimension; the model writes repo-specific question text + a recommended answer. Done when: generation yields repo-specific questions, each with a recommended answer, tagged by dimension. (`grill/bank.json` topics → `grill_generate` (bg thread + events) → `save_questions`; verified by `real_question_generation`. Pane wired UI→DB→UI.)
- [ ] **T2 Card UI + actions** — five at a time, with Submit, Not relevant, I don't know, Let's chat about this, Delete. Done when: each action behaves and "Let's chat" writes the chat resolution back into the card.
- [ ] **T3 Depth slider + coverage meter** — slider scales depth (~1–5h); Detail Coverage meter flips to "Done grilling" at saturation. Done when: raising the slider yields more questions, answering enough flips to "Done grilling", and adding a feature re-opens it.

## Watch for (this phase)
- The slider bounds scope — don't generate an unbounded flood of questions.
- Answered and dismissed both count as addressed.
