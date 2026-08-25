---
name: writing-plans
description: Use when turning a spec into an implementation plan, before writing any plan document.
---

# Writing Plans

- A plan is usually executed in a fresh context window with the plan and tools available to implement. The plan's job is to define the steps and sequence in which those steps are executed either sequentially or in parallel.
- The steps' purpose is to define contracts, interfaces, guarantees, and critical references and instructions to accomplish the task. It should not provide exhaustive instruction or code implementation.
- The plan file is the local **durable execution ledger** for its own steps: execution state, ticks, and evidence all live in it, so any session can resume from the file alone.
- If the spec covers multiple independent subsystems, suggest breaking the spec or the plan into multiple slices, one per subsystem.
- Each plan should produce working, testable software on its own.

The plan file is not meant to outlive the plan's execution and should exist as an ephemeral document in `.workspace`.

## Hard gates — verify before writing a word

1. **Critique recorded.** The spec (the issue body) has an independent cross-toolset critique and was revised against it. If not, stop and run the appropriate `spec-critique*` skill first — the reviewer's toolset must differ from the spec author's.
2. **Source of demand named.** Exactly one GitHub issue must first exist and be named — the parent issue if this plan covers a full feature, or a sub-issue if it covers one step of a larger one. No issue → create/update the source first via `writing-specs`; never write a free-floating plan.
3. If an **ADR-level decision** ever needs to be made during this stage, STOP immediately and describe the problem — this most likely means going back to the spec.

## Shape

- **5–12 steps**, each a checkbox item (`- [ ]`).
- If a plan is scoped larger than 5–12 steps, do a review and slicing recommendation.
- Design steps with clear boundaries and well-defined interfaces.
- If in an existing codebase, follow established patterns.
- It is okay to include a reasonable amount of refactoring to enhance code readability and hygiene.
- Properly order steps and label those that can/should be run in parallel.
- Provide a short difficulty or complexity rating for each step. This is to help the orchestrating agent choose which applicable model to use if using sub-agents.
- When a step is substantial enough to warrant its own tracked lifecycle (comments, a linked PR, independent close-out), materialize it as a sub-issue (`gh issue create --parent <parent#>`) per `CLAUDE.md`'s Feature & Bug Workflow — the sub-issue's open/closed state then tracks that step, and the plan step can reference it by number. Smaller steps can stay as plan-file checkboxes executed within a single sub-issue or branch.

## Step Sizing

- A step is nearly the smallest unit with only a task below it.
- Tasks are only used to help parallelize execution and coordinate.

Steps are composed of:
- "write failing tests depicting the intent"
- "run the test to ensure it fails"
- "implement the minimal code to make it work"
- "run the test to make sure it passes"
- "continue"

## Self-Review

- After writing the plan, use a fresh sub-agent (fresh eyes) to ensure the plan covers all of the spec, doesn't contain any ADR-worthy decisions being made inline, and is aligned to the overall goal of both the spec and the project.
- If issues are found, fix them inline — no need to re-review.

When the plan is written, hand off via the `executing-plans` skill.
