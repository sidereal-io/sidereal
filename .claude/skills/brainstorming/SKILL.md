---
name: brainstorming
description: Turn a rough idea into a grilled, documented requirement artifact before any spec or plan exists. Use when the user brings a new feature idea, a "what if / how should we" question, or wants to explore a problem space.
---

# Brainstorming

The output is a requirements artifact — the problem or intent, the options weighed, the direction chosen and what was left open — which in turn will become a new **source of demand**. Not code, not a plan, not a spec.

## 1 — Ground, then diverge

Explore the codebase and durable docs first (`docs/architecture/README.md`, relevant ADRs in `docs/decisions/`) so options are grounded in what exists, not imagined. Then generate **2–4 genuinely different approaches** — including the "do less / do nothing" option — each with honest tradeoffs against the project's guarantees. Present them with a recommendation first. Divergence before convergence is the point of this stage; don't anchor on the first workable shape.

## 2 — Converge by grilling

Run the `grilling` discipline on the chosen direction: one question at a time, a recommended answer with every question, and questions answerable from the codebase answered by exploring the codebase instead of asking. Walk each branch of the design tree until decisions stop changing.

## 3 — Document as you go

Use `domain-modeling` throughout (this is the `grill-with-docs` behavior): new or sharpened terms go into `CONTEXT.md` the moment they crystallize; offer an ADR only when the decision is hard to reverse, surprising without context, AND a real tradeoff — see the Decision Records section of `CLAUDE.md`.

## 4 — Land the artifact

Write the brainstorm record to `.workspace/brainstorming/<date>-<topic>.md` capturing: the options considered, what was decided and why, and what was left open. This should read as a PRD or requirements document — it becomes the scratch input for the issue body, per `CLAUDE.md`'s Feature & Bug Workflow (issue bodies are the durable spec, never repo files).

## 5 — Route to ticket creation

Then route by size, per `CLAUDE.md`'s Feature & Bug Workflow:

- **Big enough for a spec** (new guarantees, cross-package, migrations, authz): check `docs/decisions/` first for a Proposed ADR this design would have to assume an answer to — if one exists, stop and resolve it before continuing. Otherwise open the parent issue (`gh issue create --label type/feature,status/design --body-file .workspace/design.md`, or attach to an existing milestone/feature issue) and route to `writing-specs` next, then `spec-critique` before any plan.
- **Small and crisp**: go straight to `writing-plans`, citing this doc as the source of demand — still needs exactly one issue named first.
