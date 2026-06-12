---
name: beads-sdd-orchestrator
description: Orchestrate Beads as the canonical work graph while allowing any spec-driven or structured development workflow—Spec Kit, Superpowers, BMad, GSD, custom SDD, or plain markdown—to produce specs, plans, and execution artifacts. Use when creating features, turning designs into beads, planning implementation, executing child tasks, syncing GitHub Issues, creating PR gates, or finishing work.
---

# Beads SDD Orchestrator

## Purpose

Use this skill to run a development cycle where:

- **Beads is canonical.** Beads owns state, dependencies, gates, ownership, readiness, acceptance criteria, and durable work memory.
- **GitHub Issues are for humans.** Issues mirror Beads for visibility. They are not the source of truth, the spec, or the implementation plan.
- **Any SDD workflow may produce artifacts.** Spec Kit, Superpowers, BMad, GSD, custom project workflows, or plain markdown can create specs/designs, implementation plans, tasks, and execution prompts.
- **Git/PR/CI prove work.** Branches, commits, PRs, tests, and CI are evidence. PR and CI state can be represented as Beads gates.

This skill is an **orchestrator**, not a replacement for SDD frameworks. It decides where outputs from those frameworks belong and keeps Beads authoritative.

## Core rule

> **Spec/design belongs in the parent feature bead. The detailed implementation plan is a temporary worksheet. The durable implementation plan lives in Beads as child beads, dependencies, gates, and acceptance criteria.**

## Definitions

### Spec / Design

A spec is the durable product and technical contract. It answers what and why.

The spec/design of record should live in the parent feature bead. It should include:

- Problem or goal
- User-facing behavior
- Scope
- Non-goals
- Acceptance criteria
- Durable product/technical decisions
- Risks and unknowns
- Open questions
- Links or references to deeper local artifacts if any

A workflow may produce a long spec file, but the approved durable contract must be copied or distilled into the parent bead so the bead remains portable and authoritative.

### Plan / Implementation Plan

A plan is the execution strategy. It answers how.

A full implementation plan may contain markdown code blocks, exact files, test commands, implementation steps, subtasks, and subagent instructions. It may live in a temporary, often gitignored markdown file, such as:

- `.plans/<bead-id>-implementation.md`
- `.agent/plans/<bead-id>-implementation.md`
- `.superpowers/plans/<bead-id>-implementation.md`
- `.specify/specs/<feature>/plan.md`
- `.bmad/<feature>/plan.md`
- `.gsd/<feature>/plan.md`

The plan file is a worksheet. It is not the canonical work graph.

The durable implementation plan of record should be represented in Beads as:

- Child task beads
- Dependencies
- Gates
- Per-task acceptance criteria
- Ownership/claim state
- PR/CI/migration/deployment blockers

### GitHub Issue

A GitHub Issue is a human-facing mirror of a bead. It may show a spec summary, acceptance criteria, child bead list, gate status, and PR links. It should not contain the full implementation plan unless explicitly requested.

Every mirrored GitHub Issue should make clear:

> Canonical state lives in Beads. This issue is a human-facing mirror.

## When to use this skill

Use this skill when the user asks to:

- Add a new feature using Beads
- Convert a design/spec into Beads
- Use Beads with Spec Kit, Superpowers, BMad, GSD, or another SDD workflow
- Plan the next ready issue
- Break a feature into implementation tasks
- Execute a child bead
- Create a PR and gate the bead on PR merge
- Sync Beads to GitHub Issues
- Triage GitHub Issue comments into Beads
- Finish/close beads after PR merge or CI success

Do not use this skill for general coding without Beads, or for pure documentation tasks that do not involve the Beads workflow.

## First actions on every invocation

1. **Inspect the repo and available tools.**
   - Check for Beads: `.beads/`, `bd --help`, or project instructions.
   - Check for SDD workflows: `.specify/`, `.bmad/`, `.gsd/`, `.superpowers/`, `docs/superpowers/`, project command files, or custom agent instructions.
   - Check project instructions: `CLAUDE.md`, `AGENTS.md`, `README.md`, `.cursor/rules`, `.github/copilot-instructions.md`, or similar.

2. **Sync and prime Beads before deciding what is ready.**
   - Prefer, after verifying local CLI syntax:
     - `bd dolt pull`
     - `bd prime`
   - If commands differ, inspect `bd --help` and use the repo’s actual command names.

3. **Never make GitHub the decision source.**
   - GitHub Issues can inform triage.
   - Beads determines ready/blocked/claimed/done.

4. **Do not continue if canonical state is ambiguous.**
   - If a GitHub Issue and bead disagree, prefer Beads.
   - If a human explicitly wants to import GitHub changes, triage them into Beads first.

## Adapter model

Different SDD workflows produce similar artifact classes. Map them into Beads using this table.

| Artifact role | Superpowers | Spec Kit | BMad | GSD / custom | Beads destination |
|---|---|---|---|---|---|
| Product/design spec | brainstorming/design output | spec.md | PRD/architecture/story design | requirements/spec doc | Parent feature bead spec/design |
| Implementation plan | writing-plans output | plan.md | dev story plan / implementation notes | plan doc | Temporary plan worksheet + child beads |
| Task breakdown | plan tasks | tasks.md | story tasks/subtasks | task list | Child task beads with dependencies |
| Execution | subagent-driven-development | implement command | dev agent workflow | execute/build prompt | Work one child bead at a time |
| Review/finish | code review / finishing branch | validation/checklists | QA/PO/Dev review | verification | PR/CI gates and bead closure |

When a workflow has its own spec/plan/tasks artifacts, do not fight it. Let it produce them, then translate durable state into Beads.

## Phase 1: Feature intake / design

### Use when

The user says things like:

- “Let’s add a feature”
- “Create a feature bead”
- “Use Spec Kit to specify this”
- “Use BMad to design this”
- “Brainstorm this feature”
- “Turn this into a spec”

### Goal

Create or update a parent feature bead whose spec/design is the source of truth.

### Steps

1. Sync/prime Beads.
2. Select the design/spec workflow.
   - If the user named a framework, use that framework.
   - If the repo already has an established workflow, follow it.
   - Otherwise use the available generic/spec/design workflow.
3. Produce the spec/design.
4. Create or update the parent feature bead.
5. Put the durable design/spec contract into the bead.
6. Add a human spec/design approval gate when appropriate.
7. Push/sync Beads.
8. Mirror to GitHub Issues using push-only/local-authoritative sync when supported.

### Parent feature bead contents

Use this shape:

```markdown
# Spec / Design

## Goal

## Problem / Why

## User-facing Behavior

## Scope

## Non-goals

## Acceptance Criteria

## Key Decisions

## Risks / Unknowns

## Open Questions

## References
```

### Commands to prefer, after verifying local syntax

```bash
bd dolt pull
bd prime
bd create "Feature: <name>" --type feature --labels "type:feature,state:needs-spec-review,source:human"
bd update <bead-id> --description "<durable spec/design>"
bd gate create --type=human --blocks <bead-id> --reason "Spec/design approval required"
bd dolt push
bd github push <bead-id>
# or: bd github sync --push-only --prefer-local
```

If GitHub sync is implemented by a workflow rather than `bd github`, use the repo’s documented sync path. If using GitHub CLI manually, the form is usually `gh workflow run <workflow-file>` rather than `gh run workflow <workflow-file>`.

## Phase 2: Implementation planning

### Use when

The user says things like:

- “Plan the next issue”
- “Write an implementation plan”
- “Break this feature into tasks”
- “Use writing-plans”
- “Generate tasks from this spec”
- “Create child beads from the plan”

### Goal

Create a code-heavy implementation worksheet if useful, then translate its durable structure into child beads.

### Steps

1. Sync/prime Beads.
2. Use `bd ready` or equivalent to find ready work.
3. Claim the selected parent bead if planning ownership matters.
4. Use the selected SDD planning workflow.
5. Save the full detailed implementation plan as a worksheet.
6. Translate the plan into Beads:
   - Child beads
   - Dependencies
   - Gates
   - Per-task acceptance criteria
   - Risk labels
7. Add a human plan approval gate when appropriate.
8. Push/sync Beads.
9. Mirror Beads to GitHub Issues.

### Rules

- Do not paste the entire code-heavy plan into the parent bead.
- Do not paste the entire code-heavy plan into the GitHub Issue.
- Do put a short plan summary and child bead list into the parent bead.
- Do put actionable local detail into each child bead.
- If the framework already creates a `tasks.md`, convert its tasks into child beads or link them to child beads.

### Plan worksheet header

Use or adapt this for the temporary plan file:

```markdown
# Implementation Plan Worksheet: <feature>

Canonical feature bead: <bead-id>
Parent spec/design: stored in <bead-id>
This file is an execution worksheet, not the source of truth.
The durable plan is represented by child beads, dependencies, gates, and acceptance criteria.

## Summary

## Task Breakdown

## Files Likely Touched

## Test Strategy

## Commands

## Subagent / Execution Notes

## Open Questions
```

### Child bead shape

Each child bead should contain enough context to execute independently:

```markdown
# Task

## Parent

## Goal

## Scope

## Acceptance Criteria

## Likely Files

## Tests / Validation

## Dependencies

## Notes
```

## Phase 3: Implementation

### Use when

The user says things like:

- “Implement it”
- “Start the next child bead”
- “Use subagent-driven-development”
- “Execute the plan”
- “Work the next ready task”

### Goal

Execute one approved child bead or one clear implementation slice at a time.

### Steps

1. Verify human plan/spec gates are resolved.
2. Sync/prime Beads.
3. Read the current child bead and parent feature bead.
4. Confirm the current bead is claimed.
5. Create a branch that includes the bead ID.
6. Use the selected execution workflow.
7. Pass curated context to subagents/execution agents.
8. Implement and test.
9. Commit with bead IDs where practical.
10. Push branch.
11. Create PR.
12. Create a PR/CI gate in Beads.
13. Mark bead state as in PR.
14. Push/sync Beads.
15. Mirror to GitHub Issues.

### Branch naming

Prefer:

```text
feat/<bead-id>-<short-slug>
fix/<bead-id>-<short-slug>
chore/<bead-id>-<short-slug>
```

### Commit messages

Prefer including bead IDs:

```text
Add repository search endpoint (<bead-id>)
Add saved filter tests (<child-bead-id>)
```

### PR title/body

Prefer:

```text
Title: Add saved repo filters (<bead-id>)
Body: Implements <bead-id>. Child beads: <ids>. Canonical state lives in Beads.
```

### PR gate

After PR creation, prefer, after verifying syntax:

```bash
bd gate create --type=gh:pr --blocks <bead-id> --await-id <pr-number> --reason "Wait for PR merge"
bd update <bead-id> --add-label state:in-pr --remove-label state:in-progress
bd dolt push
bd github push <bead-id>
```

## Phase 4: PR completion and closeout

### Use when

The user says things like:

- “Finish this PR”
- “PR merged”
- “Close the bead”
- “Check gates”
- “Sync final state”

### Goal

Resolve external gates and close beads when done.

### Steps

1. Sync/prime Beads.
2. Check PR/CI gates.
3. Confirm acceptance criteria are satisfied.
4. Close child bead when merged/done.
5. Update parent feature bead if all children are complete.
6. Push/sync Beads.
7. Mirror to GitHub Issues.

### Default close policy

Start simple:

> A bead closes when the code is merged and acceptance criteria are satisfied.

Do not close a bead merely because code was written unless the project explicitly uses a separate “work done but PR gate still open” model.

## GitHub Issue mirror policy

GitHub Issues are human-readable cards.

A mirrored issue should include:

```markdown
# <Title>

## Canonical Bead

`<bead-id>`

## Status

## Goal

## Spec / Design Summary

## Acceptance Criteria

## Child Beads

## Gates

## PRs

---

Canonical state lives in Beads. This issue is a human-facing mirror.
```

### GitHub comments

Treat GitHub comments as input, not state.

When a human comments on GitHub:

1. Read the comment.
2. Decide whether it changes scope, acceptance criteria, plan, priority, or creates a new task.
3. Update or create beads accordingly.
4. Push/sync Beads.
5. Mirror back to GitHub.

Never implement directly from a GitHub comment unless the requested change has been reflected into Beads.

## Labels and tags

Use labels for filtering and display. Use dependencies and gates for actual readiness/blocking logic.

Suggested labels:

```text
type:feature
type:bug
type:task
type:epic
type:decision
area:api
area:web
area:db
area:infra
area:docs
area:test
state:needs-spec-review
state:needs-plan
state:needs-plan-review
state:ready
state:planning
state:in-progress
state:in-pr
state:blocked
state:done
source:human
source:github
source:agent
source:discovered
risk:migration
risk:security
risk:breaking-change
risk:unknowns
size:s
size:m
size:l
```

Important:

- Labels are not blockers.
- A bead is blocked by dependencies or gates.
- `state:blocked` may mirror reality, but it should not be the only blocker.

## Failure modes to avoid

### GitHub becomes accidental truth

Bad:

```text
Agent reads GitHub issue comments and implements directly.
```

Good:

```text
Agent triages GitHub comments into Beads, then implements from Beads.
```

### Parent bead becomes a giant implementation plan

Bad:

```text
Parent feature bead contains 500 lines of code-heavy markdown plan.
```

Good:

```text
Parent feature bead contains the design/spec and child bead list. The code-heavy plan is a worksheet. Child beads contain actionable task details.
```

### Hidden local files become secret truth

Bad:

```text
Bead says only: "See .plans/feature.md"
```

Good:

```text
Bead contains the approved spec/design contract and references the local file only as deep context.
```

### Over-gating

Bad:

```text
Every subtask requires human approval.
```

Good:

```text
Human gates are used for design approval, plan approval, risky migrations, release decisions, or other meaningful boundaries.
```

### Framework lock-in

Bad:

```text
The workflow assumes every repo uses one specific SDD tool.
```

Good:

```text
The workflow detects the repo’s SDD method and maps artifacts into Beads consistently.
```

## Checklists

### Intake checklist

- [ ] Beads synced/primed
- [ ] SDD/design workflow selected
- [ ] Spec/design produced
- [ ] Parent feature bead created/updated
- [ ] Spec/design of record stored in parent bead
- [ ] Acceptance criteria present
- [ ] Human spec gate added if needed
- [ ] Beads pushed
- [ ] GitHub mirror updated

### Planning checklist

- [ ] Beads synced/primed
- [ ] Ready bead selected from Beads
- [ ] Plan worksheet produced if useful
- [ ] Child beads created
- [ ] Dependencies added
- [ ] Gates added for external blockers
- [ ] Per-child acceptance criteria added
- [ ] Human plan gate added if needed
- [ ] Beads pushed
- [ ] GitHub mirror updated

### Implementation checklist

- [ ] Relevant gates resolved
- [ ] Beads synced/primed
- [ ] Parent spec/design read
- [ ] Child bead read and claimed
- [ ] Branch includes bead ID
- [ ] Execution workflow selected
- [ ] Tests run
- [ ] Commits include bead IDs where practical
- [ ] PR created
- [ ] PR/CI gate created in Beads
- [ ] Beads pushed
- [ ] GitHub mirror updated

### Closeout checklist

- [ ] PR merged or explicit close condition met
- [ ] CI/status gates checked
- [ ] Acceptance criteria verified
- [ ] Child bead closed
- [ ] Parent bead updated/closed if complete
- [ ] Beads pushed
- [ ] GitHub mirror updated

## User-facing summaries

When explaining state to the user, use concise terms:

- “The design/spec is now in the parent bead.”
- “The detailed implementation plan is a worksheet; I translated its durable tasks into child beads.”
- “GitHub Issues have been updated as a human mirror.”
- “This child bead is blocked by the PR gate until merge.”

Do not say or imply that GitHub Issues are canonical.

## Final principle

> **Beads decides what work exists and what is ready. SDD workflows create useful artifacts. GitHub shows humans the state. Git and PRs prove the implementation.**
