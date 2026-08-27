---
id: adrs-adr006
date: 2026-08-18
status: accepted
title: 'ADR006: Declarative Processing, Selectors, and Policy Deferral'
description: Architecture Decision Record (ADR) for how v2 drives processing — declarative Processing Goals reconciled to convergence, not a workflow engine.
---

# ADR-006: Declarative Processing, Selectors, and Policy Deferral

## Context

The product must react when an asset is ingested, a collection becomes ready, a button is clicked, an API request arrives, or a schedule fires, and several Operators may need to succeed before an asset is fully processed. The user intent is not "run A then B then C" — it is "make all these outcomes true," with ordering mattering only where one outcome is a genuine prerequisite of another. A workflow makes the execution path the durable abstraction, which forces users to reason about where a cursor is stuck and bakes in incidental ordering. This decision needs a model for the desired state above individual attempts without becoming a general-purpose workflow engine, and one answer to three applicability questions (which subjects a policy covers, which Operators can satisfy a subject, which assets belong to a dynamic Collection). Facet mechanics are owned separately by [ADR-008 — Metadata envelope, facets & write authority](ADR-008-facet-schema-and-write-authority.md).

## Decision

Adopt **shared Selectors, declarative Processing Goals, and reconciliation** — no general-purpose workflow engine, no first-class Pipeline Run. Selectors are a shared core primitive over `kind`, source, typed facets, and Collection membership, bounded to boolean composition plus existence/equality/set/typed-comparison (never arbitrary plugin code), with indexed evaluation and a human-readable match explanation. A versioned Processing Policy uses a Selector to declare the outcomes required for matching versions/snapshots; each Operator declares an `accepts` Selector and the goals it provides, and core dispatches it only when the policy selected the subject, the Operator provides a missing goal, `accepts` matches, grants permit, and prerequisites are met. Goals bind to immutable inputs (an AssetVersion or Collection snapshot plus the policy revision). Reconciliation is level-triggered: events request prompt evaluation, but periodic sweeps and startup recovery recompute missing work from source-of-truth state, so only real data dependencies impose order and independent goals run concurrently. Success is durable evidence (facets, artifacts, lineage, receipts) that a restart never discards or blindly repeats; failure is visible at the goal (`pending`/`running`/`blocked`/`satisfied`/`needs_attention`). Sources classify but never orchestrate. User-authored policy rules, conflict resolution, simulation, and a policy editor are deferred until after cutover; built-in domain-pack policies provide pre-cutover behaviour.

## Consequences

- One execution model serves events, manual actions, schedules, and later user policies; recovery is transparent and there is no opaque stuck workflow.
- Operators must carry precise prerequisite/outcome/invalidation/idempotency declarations, and external effects need durable receipts and explicit ambiguous-completion handling.
- The level-triggered sweep is the recovery backstop, but its cost scales with total goal/asset count, not just what changed — on a large library, sweep frequency trades recovery latency against load and needs a bound.
- When two Operators both provide the same outcome (e.g. Astrometry.net and ASTAP for `astro.solve.*`), "dispatch any eligible Operator" is under-specified: without a stated tie-break, selection can depend on registry order and a re-run may pick a different provider, hurting reproducibility. A deterministic preference rule must be defined with the Operator contract.

## Alternatives Considered

### Alternative 1: Execute versioned workflows or pipelines
- **Pros:** familiar explicit control flow; natural where order itself is meaningful; established engines supply durable waits, retries, and graph scheduling.
- **Cons:** makes users reason about stuck execution instead of unsatisfied outcomes; encodes unnecessary ordering; adds a second operational state model and workflow-version migration; user-authored workflows are a much larger product surface.
- **Why not:** it optimises for the rare ordered case at the cost of the common "make these true" case, and enlarges the debugging surface.

### Alternative 2: Dispatch individual Operators only
- **Pros:** smallest initial implementation.
- **Cons:** Sources, API handlers, and UI actions each grow duplicated sequencing logic; automatic processing cannot reliably recover from missed events or partial completion; adding policy later means replacing the core dispatch path after plugins depend on it.
- **Why not:** it pushes coordination into every caller and forecloses convergent recovery.
