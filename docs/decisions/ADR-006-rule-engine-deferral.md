# 006: Rule-Engine Deferral

**Status:** Proposed
**Date:** 2026-07-29
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). Confirms that user-defined per-kind pipelines land post-cutover (M7+), and fixes what M2's Operator engine must provide now so that remains true.

## Problem

The north star includes per-kind pipelines: `kind = light` routes to one chain of Operators, `kind = dark` to another, driven by user-defined rules. The RFC scopes this to M7+, after cutover — in v2.0 Operators are invoked individually.

Deferring is only safe if the engine built at M2 does not have to be *rebuilt* to accept rules later. So the real question is not "defer or not" but **which hooks must exist at M2** for the deferral to be free.

The architecture argues this is naturally cheap: a routing rule matches on kind and facets, and both are core concepts being built anyway, so routing is a query plus a dispatch — not new infrastructure.

## Options

### Option A: Defer, with matching and dispatch built as separable pieces

Ship individual invocation in v2.0, but build M2's engine so dispatch takes a resolved set of
`(operator, params)` rather than being hard-wired to a user action.

**Pros:**
- No user-facing rule surface to design, document, or support before cutover.
- The hook is small: dispatch already needs to accept a list of runs; a rule engine just becomes another producer of that list.
- Facet queries are being built for calibration-master matching regardless, so the matching half already exists.

**Cons:**
- Requires discipline at M2 — an engine wired directly to "user clicked solve" is cheaper short-term and would need rework.

### Option B: Defer entirely, accept rework later

**Pros:**
- Simplest M2.

**Cons:**
- Rule support then touches the engine's core dispatch path after third-party plugins depend on its behaviour — a worse time to change it.

### Option C: Build rules in v2.0

**Pros:**
- Delivers a north-star capability at cutover.

**Cons:**
- Adds a substantial design surface (rule language, conflict resolution, dry-run, debugging why a rule didn't fire) to a milestone set already carrying a language switch and a data-model change.
- Rules over an unproven plugin interface will encode assumptions that ABI v0.2 breaks.
- Cutover is already gated on eleven non-negotiables; this is not one of them.

## Recommendation

**Option A.** Confirm the user-facing rule-language and editor deferral, and record the execution
primitives M2 owes it:

1. **Dispatch takes a resolved run graph**, not a single user-triggered Operator.
2. **Facet and kind matching is a queryable core capability**, not logic embedded in the
   calibration-matching path.
3. **State changes publish through a durable event/outbox model** in the same transaction as the
   change they describe.
4. **Every derived run carries causal metadata**: root event, parent run, depth, triggering rule
   revision, and a deterministic evaluation/idempotency key.
5. **Evaluation is once-only per event and rule revision**, with cycle/depth protection and explicit
   ordering for competing dispatches.
6. **Operation Run records retain the resolved inputs and params**, so later rule edits do not alter
   the meaning of historical runs.

These are not the deferred rule product. They are the durable causality and idempotency primitives
also needed for bulk work, retries, progress, and reliable external side effects in v2.0. The rule
language, conflict-resolution UI, dry-run experience, and user-authored routing remain M7 work.

Worth confirming during review: does anything on the [non-negotiable cutover list](../architecture/README.md#non-negotiable-before-cutover) implicitly need rules? Bulk plate solving is the closest, and it needs bulk dispatch rather than rule evaluation — but it should be checked rather than assumed.

## Decision

[Filled in after review.]
