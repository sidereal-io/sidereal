---
name: critique
description: Use when a design, spec, plan, or skill is written or materially revised. Also use when a new ADR (decision record) is drafted, or a subagent returns claiming significant work is complete.
---

# Critique a Result

Attack the work before it is accepted. Argue as the engineer who inherits it with none of today's context, or who gets paged when it fails. Press on the decision; never rewrite it — resolution stays with whoever made it.

## When to apply

Fire on any of these:

- **Design or plan.** A design, spec, plan, or skill is written or materially revised.
- **New ADR.** A decision record is drafted.
- **Subagent claim.** A subagent returns claiming a significant body of work is complete.

## Choose the engine

Prefer an independent adversary. A self-review inherits your own blind spots.

- **Default — dispatch.** Hand the critique to a different model family with `choose-an-adversary`.
- **Always dispatch** for high stakes: a new ADR, a subagent's significant deliverable, or a spec or plan that spans multiple units, adds contracts or migrations, or changes the architecture.
- **Low stakes.** In-session is fine for a bounded, single-unit design or plan with no new contracts. Say plainly that the critique ran in-session, so the reader knows it is weaker.
- **No other family installed?** Critique in-session and say so.

## The method

Follow the same steps whether you run the critique or another model does.

1. **Name the decision** in one line, with the alternatives it forecloses. If you cannot name it crisply, you are not ready to attack it.
2. **Find the specific failure.** Name the input, the scale, the concurrent access, the partial failure. "This feels fragile" is not a concern; "this corrupts the tree when the move is interrupted mid-write" is.
3. **Split the assumptions.** Check the ones the repo can settle — go read the code. An assumption the code already answers is not a concern. Name the ones that bet on the future, and say what makes each bet lose.
4. **Weigh the opportunity cost.** What does this foreclose, what gets harder to change later, and what work does it commit someone to that nobody scheduled?
5. **Find the breakdown boundary.** The volume, the second user, the second plugin, the migration, the day the external service is down.

Raise at least one substantive concern, and at least one non-obvious failure mode — something the author would not have listed alone.

For a subagent's claimed-complete work, check the claim against the actual files and output. Do not reason about work you can go read.

## The output

Present concerns ranked, most severe first. For each:

- **The concern** — one line, stated as the defect, not a question.
- **The scenario** — the concrete conditions under which it bites.
- **What must be true** for it not to matter — this is what the other side must answer.

Then split what **blocks** acceptance from what is merely **noted**, and say which is which. A pile of undifferentiated objections is as useless as none.

When another model ran the critique, verify each finding before you accept it. Mark it CONFIRMED, REFUTED (name the contradicting evidence), or OUT-OF-SCOPE (say why). The critic has no product context, so some findings will miss.

Drop any concern that a response actually resolves. Manufacturing disagreement to stay in character defeats the point.
