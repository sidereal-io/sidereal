---
name: superpowers-beads-bridge
description: Use when running any superpowers workflow skill — brainstorming, writing-plans, executing-plans, subagent-driven-development, or finishing-a-development-branch — in the sidereal repo, where specs, plans, and work tracking must live in beads (bd) rather than TodoWrite or markdown files under docs/superpowers.
---

# Superpowers × Beads Bridge

## Overview

This is an **overlay** on the superpowers workflow. Follow the standard
`superpowers:brainstorming`, `superpowers:writing-plans`, `superpowers:executing-plans`
(or `subagent-driven-development`), and `superpowers:finishing-a-development-branch`
skills exactly as written, with the five overrides below.

In sidereal, **beads (bd) is the system of record** for specs, plans, and task tracking.
Where a superpowers skill says to use TodoWrite or write a spec/plan to
`docs/superpowers/`, translate it to the beads equivalent here instead.

**If an override here conflicts with a superpowers skill, this skill wins** — per
superpowers' own `using-superpowers` rule that user/project instructions (AGENTS.md)
take precedence over skills.

## Quick Reference

| Superpowers default | Use instead |
|---|---|
| "Create a TodoWrite todo per checklist item" | TodoWrite ONLY for a skill's own ephemeral process steps. Every deliverable work unit is a `bd` issue. |
| Spec → `docs/superpowers/specs/*.md` (committed) | Spec → `bd` issue `--design`. `docs/superpowers/` is gitignored scratch. |
| Plan → `docs/superpowers/plans/*.md` | Plan → `bd` epic + one child task issue per Task, ordered with `bd dep`. Scratch only: `.workspace/plans/`. |
| executing-plans / subagent: TodoWrite per task | `bd ready` → `bd update <id> --claim` → work → `bd close <id>`. |
| finishing-a-development-branch close | Close bd issues, then the session-close sequence in Override 5. |

## Override 1 — Spec lives on a bd issue

**Replaces** brainstorming Step 6 (write spec doc to `docs/superpowers/specs/`) and
Step 8 (user reviews the spec file). The spec — dialogue output, design, and final
form — lives in a bd issue's `--design`, not a local file.

```bash
# New spec (design body piped in on stdin):
bd create --type=feature --title "<title>" --labels spec/brainstorming --design-file -

# Request human review of the spec, then WAIT:
bd human <issue-id>            # flags the issue for a human (adds the 'human' label)
```

Do **not** proceed to implementation until a human applies the approval label:
`bd label add <issue-id> spec/ready`. The agent never sets `spec/ready` itself. The
brainstorming "user reviews the spec" gate becomes "a human applies `spec/ready`."

## Override 2 — Plan is a bd epic + child task issues

**Replaces** writing-plans' save location (`docs/superpowers/plans/`) and its execution
handoff. The plan becomes a bd epic whose children are the individual tasks.

```bash
# Epic holds the goal/architecture/tech-stack overview:
bd create --type=epic --parent <spec-issue-id> --title "<feature> plan" --design-file -

# One child task per plan Task (its steps + code go in --design):
bd create --type=task --parent <epic-id> --title "Task N: <name>" --design-file -

# Order the tasks (later depends on earlier):
bd dep add <task-N+1-id> <task-N-id>
```

A local `.workspace/plans/YYYY-MM-DD-<slug>.md` (gitignored) is allowed as scratch only;
the bd epic and its children are the source of truth.

## Override 3 — Execute via `bd ready`

**Replaces** the TodoWrite-per-task tracking in `executing-plans` and
`subagent-driven-development`.

```bash
bd ready                       # next unblocked child task
bd update <task-id> --claim    # sets assignee + status=in_progress
# ... TDD red → green → refactor → commit (as the superpowers skills define) ...
npm run check                  # after each task; also `npm run test` if packages/shared/ changed
bd close <task-id>
```

A skill's **own** internal step list may use TodoWrite as ephemeral process scaffolding.
The record of deliverable work is always the bd issue — never TodoWrite, never markdown.

## Override 4 — ADR before PR

**Adds** a step before `finishing-a-development-branch`. If a notable design decision was
made or changed during implementation — a trade-off where a reasonable person could argue
for a different approach — write an ADR from `docs/decisions/ADR-000-template.md`
(sequential `ADR-NNN-short-slug.md`, status Proposed unless already Accepted; flip status
in the same PR if it implements an Accepted ADR). Routine choices don't need one.

## Override 5 — PR body + session close

**Replaces** `finishing-a-development-branch`'s close flow. Use Conventional Commits in
the PR title; the PR body references the bd issue id for traceability (bd issues close via
`bd close`, not GitHub auto-close). Then run the session-close sequence:

```bash
bd close <ids>                 # close the finished issues
bd dolt push                   # if a Dolt remote is configured
git pull --rebase && git push
git status                     # MUST show up to date with origin
```

## What this skill does NOT change

Everything else stays as superpowers defines it: the brainstorming dialogue style and
approach proposals, the writing-plans header and bite-sized task structure, the
Red-Green-Refactor loop, frequent commits, and the finishing-a-development-branch options
menu. Only the five points above are overridden.
