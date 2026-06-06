# Review Helper — Proposal

*Repo: github.com/hotredsam/Review-Helper. (Name still informal — change anytime.)*

## The problem

Vibe coding fails for a predictable reason: the model writes code with no persistent system design. Given a vague prompt, it invents an architecture confidently — and quietly cuts corners on edge cases, security, and structure. The fix the community keeps re-learning the hard way is to impose the structure the model can't hold on its own: a written plan, small modular files, a rules file, context hygiene, and Git checkpoints — *before* you start prompting. Specifying requirements up front, not the model's coding ability, is the binding constraint.

There is a second problem specific to this builder: **not knowing enough about software (or product) to know what to specify.** Picking a stack, judging an architecture decision, or reasoning about monetization and user behavior all require understanding the app doesn't currently have. So the tool can't just enforce discipline — it has to *teach while you plan.*

## The solution

Review Helper is a planning, learning, and assessment cockpit that sits in front of every project. You bring a repo (or a blank idea); Review Helper analyzes the current state, scores how well-structured it is, and grills you — relentlessly, with recommended answers — until the project is specified well enough to build. As you go, a standing **Understand hub** lets you ask about anything (technical or product) and saves what you learn. When the plan is solid, Review Helper pushes a consistent planning package and phased GitHub issues, and you hand the plan to Claude Code to build phase by phase.

The defining bet: **understanding is the main activity, not a side feature.** Most of your time in Review Helper is spent figuring out what a decision means and why — across architecture, frontend, backend, the glue between them, deployment, business model, graphic design, and user behavior. Everything else (grilling, the stack panes, the decisions record) orbits that.

## Who it's for

A builder who ships real software with AI assistance but is newer to software engineering and product — and who wants a full understanding of what they're doing before Claude Code does it, with requirements hammered out so little is left to chance.

## Core principles

1. **Structure before prompting.** A plan, architecture, and decisions record exist before code does.
2. **Understand while you plan.** Any decision is one click from an explanation; what you learn is saved.
3. **You stay in control.** Nothing edits the record or your GitHub silently — Review Helper proposes, you approve. The AI reads your code but never writes it; Review Helper owns every file write and commit.
4. **One consistent plan across every repo.** Review Helper normalizes the varying plan quality across your projects into a single shape.
5. **Resume, never restart.** The plan always knows what's done, so Claude Code never rebuilds Phase 1 when Phase 4 is finished.

## What success looks like

- You can open any repo, see an honest 0–100 read of how well it's structured, and know the top three things to fix.
- You can get a project from "rough idea" to "specified well enough to build" through a grilling session whose depth you control.
- You finish a session understanding the decisions that were made — not just that they were made.
- Claude Code builds from the resulting plan without stalling on ambiguity or redoing finished work.
- Polished, fast, daily-driver quality — roughly 90% of the way to App-Store-grade feel.

## Scope

**In scope (v1):** the full vibecoding workflow described in `REQUIREMENTS.md` — projects from GitHub or scratch, repo analysis, the assessment, the Understand hub, grilling, the stack panes, the decisions record, the feature inbox, plan generation, and GitHub sync.

**Later / coming soon:** a separate learning mode beyond vibecoding (e.g. CPA study, general learning). Stubbed and labeled "coming soon" in v1; on the back burner until the vibecoding workflow is solid.

**Explicit non-goals:** see `REQUIREMENTS.md` § Non-goals. Notably: no runtime LLM-generated UI, no App Store submission commitment in v1, and no two-way GitHub-issue editing.
