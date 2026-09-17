---
id: adrs-adr006
date: 2026-08-18
status: accepted
title: 'ADR006: Declarative Processing, Selectors, and Policy Deferral'
description: Architecture Decision Record (ADR) for how v2 drives processing — declarative Processing Goals reconciled to convergence, not a workflow engine.
---

# ADR-006: Declarative Processing, Selectors, and Policy Deferral

## Context

The product must react to many triggers: an asset is ingested, a collection becomes ready, a button is clicked, an API request arrives, or a schedule fires. Several Operators may need to succeed before an asset is fully processed.

The user's intent is not "run A, then B, then C." It is "make all these outcomes true," with ordering that matters only where one outcome is a genuine prerequisite of another. A workflow makes the execution path the durable abstraction. That forces users to reason about where a cursor is stuck, and it bakes in incidental ordering.

This decision needs a model for the desired state above individual attempts — without becoming a general-purpose workflow engine. It also needs one answer to three applicability questions: which subjects a policy covers, which Operators can satisfy a subject, and which assets belong to a dynamic Collection.

Facet mechanics are owned separately by [ADR-008 — Metadata envelope, facets & write authority](ADR-008-facet-schema-and-write-authority.md).

## Decision

Adopt **shared Selectors, declarative Processing Goals, and reconciliation** — no general-purpose workflow engine, and no first-class Pipeline Run.

**Selectors** are a shared core primitive over `kind`, source, typed facets, and Collection membership. They are bounded to boolean composition plus existence, equality, set, and typed-comparison operators — never arbitrary plugin code. They evaluate against an index and produce a human-readable match explanation.

**Processing Policies** are versioned. A policy uses a Selector to declare the outcomes required for matching versions and snapshots. Each Operator declares an `accepts` Selector and the goals it provides. Core dispatches an Operator only when all of these hold: the policy selected the subject, the Operator provides a missing goal, `accepts` matches, grants permit it, and prerequisites are met.

**Goals** bind to immutable inputs — an AssetVersion or Collection snapshot, plus the policy revision.

**Reconciliation is level-triggered.** Events request prompt evaluation, but periodic sweeps and startup recovery recompute missing work from source-of-truth state. So only real data dependencies impose order, and independent goals run concurrently.

Success is durable evidence (facets, artifacts, lineage, receipts) that a restart never discards or blindly repeats. Failure is visible at the goal: `pending`, `running`, `blocked`, `satisfied`, or `needs_attention`. Sources classify but never orchestrate.

User-authored policy rules, conflict resolution, simulation, and a policy editor are deferred until after cutover. Built-in domain-pack policies provide pre-cutover behaviour.

## Consequences

- One execution model serves events, manual actions, schedules, and later user policies. Recovery is transparent, and there is no opaque stuck workflow.
- Operators must carry precise prerequisite, outcome, invalidation, and idempotency declarations. External effects need durable receipts and explicit handling for ambiguous completion.
- The level-triggered sweep is the recovery backstop, but its cost scales with total goal and asset count, not just with what changed. On a large library, sweep frequency trades recovery latency against load and needs a bound.
- When two Operators both provide the same outcome — for example, Astrometry.net and ASTAP for `astro.solve.*` — "dispatch any eligible Operator" is under-specified. Without a stated tie-break, selection can depend on registry order, and a re-run may pick a different provider, hurting reproducibility. A deterministic preference rule must be defined with the Operator contract.

## Alternatives Considered

### Alternative 1: Execute versioned workflows or pipelines
- **Pros:** familiar explicit control flow; natural where order itself is meaningful; established engines supply durable waits, retries, and graph scheduling.
- **Cons:** makes users reason about stuck execution instead of unsatisfied outcomes; encodes unnecessary ordering; adds a second operational state model and workflow-version migration; user-authored workflows are a much larger product surface.
- **Why not:** it optimises for the rare ordered case at the cost of the common "make these true" case, and it enlarges the debugging surface.

### Alternative 2: Dispatch individual Operators only
- **Pros:** smallest initial implementation.
- **Cons:** Sources, API handlers, and UI actions each grow duplicated sequencing logic; automatic processing cannot reliably recover from missed events or partial completion; adding policy later means replacing the core dispatch path after plugins already depend on it.
- **Why not:** it pushes coordination into every caller and forecloses convergent recovery.
