//! Learning mode (Phase G): a second top-level mode, separate from code review.
//! The user names a subject (describe or upload), gets grilled on scope, picks
//! from a generatively-proposed module manifest, then studies generated
//! materials (notes/flashcards/quiz/tutor). An adaptive *learner profile* —
//! per-skill mastery (Bayesian Knowledge Tracing) + flashcard spaced repetition
//! (FSRS) + pace signals — is updated from real interactions and fed back (as
//! bounded numbers only) into the proposal/material prompts. No "learning
//! styles": that framing is scientifically debunked; we adapt on evidence.

pub mod commands;
mod gen;
pub mod intake;
pub mod store;
