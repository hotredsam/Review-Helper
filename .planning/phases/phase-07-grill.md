# Phase 7 — Grill-me cards & detail coverage
Status: done
Goal: Repo-specific questions with recommended answers, five at a time, with a depth slider and a "Done grilling" signal.
Depends on: Phase 4

## Tasks
- [x] **T1 Question generation** — bank supplies the topic/dimension; the model writes repo-specific question text + a recommended answer. Done when: generation yields repo-specific questions, each with a recommended answer, tagged by dimension. (`grill/bank.json` topics → `grill_generate` (bg thread + events) → `save_questions`; verified by `real_question_generation`. Pane wired UI→DB→UI.)
- [x] **T2 Card UI + actions** — five at a time, with Submit, Not relevant, I don't know, Let's chat about this, Delete. Done when: each action behaves and "Let's chat" writes the chat resolution back into the card. (`QuestionCard` 5 actions → `grill_answer`/`grill_set_status`/`grill_delete`/`grill_chat_resolve`; chat resolution stored as a chat-sourced answer + marks answered. Full chat UI lands in Phase 8.)
- [x] **T3 Depth slider + coverage meter** — slider scales depth (~1–5h); Detail Coverage meter flips to "Done grilling" at saturation. Done when: raising the slider yields more questions, answering enough flips to "Done grilling", and adding a feature re-opens it. (`DepthSlider` → depth → `select_topics` fills to `target_for_depth`; `CoverageMeter` + `computeCoverage` (done = open===0 && total>0; answered/not_relevant/unknown all addressed; new open question re-opens).)

## Watch for (this phase)
- The slider bounds scope — don't generate an unbounded flood of questions.
- Answered and dismissed both count as addressed.
