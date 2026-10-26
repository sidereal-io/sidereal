---
name: lets-brainstorm
description: Use when an OpenSpec change's brainstorm step questions the user about a story and records the decisions in a ledger. Says how to think, what to ask, how to fill the ledger and when to split the story. The schema says which files to write.
---

# Brainstorming

Reach one shared understanding of a story with the user before anyone plans
it. The schema that calls this skill says which files to write and when to
commit. This skill says how to question, what to record and when to split.

## Read and maintain domain language

Read [Domain language](../lets-shape/references/domain-language.md) before
using domain terms. Resolve project vocabulary from `planningHome.root` in
OpenSpec's artifact instructions, including a selected planning store. Read
the context-selected shared glossary plus existing ancestor and local
glossaries for all affected full capability paths. Do not assume an earlier
session loaded them. Missing optional files need no empty replacements;
a missing declared pointer needs discussion before dependent work proceeds.

Question ambiguous or conflicting meanings before relying on them. Confirm
the meaning and narrowest scope against ancestors and actual sibling uses.
Write each agreed definition immediately to its authoritative glossary and
commit those edits on the current change branch as their own `docs:` unit,
protecting preexisting edits. For a separate store, commit in that planning
repository. Keep candidate meanings explicitly tentative in the brainstorm
ledger until the user agrees. Never put glossary deltas in change `specs/`.
A split story uses shaping's glossary ownership handoff rather than copying
shared definitions into another PR.

## The stance

Be a thinking partner, not a form to fill in.

- **Grounded.** Read the code, tests and specs the story touches before you ask anything. Never ask the user for a fact you can check.
- **Patient.** Let the shape of the problem emerge. Do not rush to a conclusion.
- **Skeptical.** Question the story's assumptions, the user's and your own. Say so if the story solves the wrong problem, if a smaller story would deliver the outcome, or if the code hides complexity the story misses.
- **Honest.** If something is unclear, dig deeper. Never fake understanding, and never invent the user's intent, priorities or constraints: ask.
- **Visual.** When a flow, a set of states or a choice is hard to hold in words, draw it.

## Questioning

### Read the relevant specs first

Run `openspec list --specs` to find existing capability IDs. Read the affected
specs with `openspec show <spec-id> --type spec`, using each full ID, such as
`identity/user-auth`. Use the planning home OpenSpec resolves, including a
selected store, rather than assuming the specs live beside the code.

A domain groups related behavior specs. It can describe an application,
service or feature, with flat capability paths or optional grouping such as
`identity/user-auth`. Follow the existing layout. Use the request, project
context and code to identify the affected capabilities; domain names do not
automatically map to source directories.

Read shared behavior the story depends on too, such as authentication used by
both an API and a frontend. A shared spec is context unless its requirements
change. One story can affect several domains. When the affected area is clear,
continue without asking the user to select or confirm a domain.

### Settle the story

- Ask one question at a time, with your recommended answer and why it matters: the decision it settles.
- Settle first the questions that others depend on, such as scope before behavior. If an answer changes an earlier one, go back to it.
- Skip a question whose answer would not change the plan.
- Where a question has real alternatives, give two or three with their trade-offs, and lead with the one you recommend.
- Settle scope, behavior and acceptance: the what and the why. Leave the how to the design step, when the flow has one.

## The ledger

One row per resolved question, in the order settled. Each row names its source.

| Source | When |
|---|---|
| `User` | The user decided. |
| `Evidence: <where>` | Code or a document settled it. Name the file. |
| `Assumption` | Neither did. |

Silence is not agreement: a recommendation the user did not accept is an `Assumption`. Keep each row to one line; the design explains the why and the how.

## Splitting the story

A story is the right size when it would take about five to twelve tasks to build. If the answers show it is bigger, split it now, before any plan exists. Keep the thinnest slice that still delivers the outcome, and file the rest as a new issue.

1. Show the user the split: what this story keeps, and what moves to the new issue. Get a yes.
2. File the rest with the `lets-shape` skill: run its single-issue pass on the rest. Tell it that the rest is split from #<n>, this story. Its parent is this story's parent, if it has one. It is blocked by #<n>, unless the rest can ship before this slice. `lets-shape` writes the new issue's packet and sets its fields on the Project.
3. Record the split under "Changes from the story", with the new issue's number.

## Confirming

Show the user the ledger and the changes from the story, and ask them to confirm. Only an explicit yes counts. Until then the brainstorm is not finished, whatever the file says.
