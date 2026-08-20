# 006: Declarative Processing, Selectors, and Policy Deferral

**Status:** Accepted
**Date:** 2026-07-31
**Context:** Establishes declarative, convergent processing while deferring user-authored policy rules
until after cutover. Facet mechanics are owned separately by [ADR-008 — Metadata envelope, facets &
write authority](ADR-008-facet-schema-and-write-authority.md).

## Problem

The product needs to react when an asset is ingested, a collection becomes ready, a user clicks a
button, an API request arrives, or a schedule fires. Several Operators may need to succeed before the
asset or collection is fully processed.

The user intent is not normally “run A, then B, then C.” It is “make all of these outcomes true.” For
example, a stacked image may need extracted metadata, a plate solution, a canonical name, a
thumbnail, and an Immich publication receipt. Ordering matters only where one outcome is a genuine
prerequisite for another.

A workflow or pipeline makes the execution path the durable abstraction. That creates a second
problem: when execution stops, the user must diagnose where a workflow cursor is stuck and decide
how to move it. It also bakes an ordering into policy even when the order is incidental.

An `Operation Run` remains the exact historical record of one Operator attempt. This decision needs a
model for the desired state above those attempts without becoming a general-purpose workflow engine. It also
needs one answer to three applicability questions: which subjects receive a policy, which Operators
can process a subject, and which assets belong to a dynamic Collection.

## Options

### Option A: Reconcile declarative Processing Goals

A versioned Processing Policy uses a shared Selector to declare outcomes required for matching
AssetVersions or immutable Collection snapshots. The reconciler materializes durable Processing
Goals, compares them with recorded state, and dispatches eligible Operators until every applicable
goal is satisfied.

Operators declare an `accepts` Selector, semantic prerequisites, and outcomes. The scheduler chooses
any suitable Operator whose selector matches and imposes only the ordering implied by unmet
prerequisites. Events prompt reconciliation, while a periodic sweep guarantees convergence after
missed events or process crashes.

**Pros:**

- Models the user's desired result instead of an incidental execution sequence.
- Independent work can run concurrently and alternative Operators can satisfy the same outcome.
- Recovery is transparent: an unsatisfied goal records why it remains unsatisfied.
- Events, manual actions, schedules, and later user policies share one execution model.
- Requires no separate workflow runtime or opaque workflow state.

**Cons:**

- Operators need precise prerequisite, outcome, invalidation, and idempotency declarations.
- Selector changes can fan out widely and require indexed evaluation plus clear match explanations.
- Planning must detect missing providers and dependency cycles.
- External effects require durable receipts and explicit ambiguous-completion handling.
- Some inherently ordered processes may still need a purpose-built coordinating Operator.

### Option B: Execute versioned workflows or pipelines

Triggers start a durable dependency graph of Operator steps. A Pipeline Run records the selected
definition, current position, child runs, and aggregate status.

**Pros:**

- Familiar model with explicit control flow.
- Natural fit for processes where order itself is meaningful.
- Established workflow engines can supply durable waits, signals, retries, and graph scheduling.

**Cons:**

- Makes users reason about stuck execution rather than unsatisfied outcomes.
- Encodes unnecessary ordering and requires workflow-version migration semantics.
- Adds a second operational state model and potentially another self-hosted runtime.
- User-authored workflows create a substantially larger product and debugging surface.

### Option C: Dispatch individual Operators only

Every trigger or button directly invokes one Operator; compound processing is coordinated by callers.

**Pros:**

- Smallest initial implementation.

**Cons:**

- Sources, API handlers, and UI actions acquire duplicated sequencing logic.
- Automatic processing cannot recover reliably from missed events or partial completion.
- Adding policy later requires replacing the core dispatch path after plugins depend on it.

## Decision

**Accepted Option A.** It implements shared Selectors, declarative Processing Goals, and
reconciliation. It does not introduce a general-purpose workflow engine or a first-class Pipeline
Run.

The following semantics are part of the decision:

1. **Selectors are a shared core primitive.** The deterministic selector vocabulary covers `kind`,
   source instance, typed facets, and Collection membership. Core owns indexed
   evaluation and records a human-readable match explanation. The language is bounded to boolean
   composition plus existence, equality, set, and typed comparison predicates; it cannot invoke
   arbitrary plugin code.
2. **Facets are the extensible selection metadata.** Facet schemas declare type, scope, mutability,
   indexing, ownership, and provenance. Built-in policies reference canonical core or domain-pack
   schemas rather than producer-specific fields; the facet contract is owned separately.
3. **Sources classify; they do not orchestrate.** Source configuration may assign an initial `kind`
   and configured facets. A Source may propose detected facets within its grants, but never selects
   or invokes Operators. Mixed-kind Sources may classify per asset rather than declaring one fixed
   kind.
4. **Policies declare outcomes, not steps.** A versioned Processing Policy uses a Selector to match
   asset or Collection state and declares the goals that must be satisfied.
5. **Operators declare applicability.** Each Operator declares an `accepts` Selector plus the goals
   it provides. Core dispatches it only when the policy selected the subject, the Operator provides
   the missing goal, `accepts` matches, its grants permit the run, and prerequisites are satisfied.
6. **Collections support explicit or selected membership.** A selector-backed Collection is a
   dynamic view. Processing binds an immutable membership snapshot so later selector results cannot
   alter an in-flight or historical input set.
7. **Goals bind to immutable inputs.** Each Processing Goal targets an AssetVersion or an immutable
   Collection snapshot, plus the policy revision that required it.
8. **Reconciliation is level-triggered.** Durable events request prompt evaluation, but periodic
   sweeps and startup recovery recompute missing work from source-of-truth state.
9. **Only data dependencies impose order.** Eligible goals may run concurrently. The planner rejects
   dependency cycles and reports missing providers rather than waiting indefinitely.
10. **Operation Runs are attempts.** Each attempt records the goals it addresses, resolved inputs and
   params, outputs, side-effect state, idempotency key, status, and logs.
11. **Success is durable evidence.** Facets, artifacts, lineage, and external publication receipts
   satisfy goals. A process restart never discards that evidence or repeats a completed external
   effect blindly.
12. **Invalidation is explicit.** Operators declare which outcomes a new AssetVersion or mutation may
   invalidate. Reconciliation then creates or reopens goals for the new immutable input.
13. **Failure is visible at the goal.** Goals progress through `pending`, `running`, `blocked`,
   `satisfied`, and `needs_attention`. Bounded retry policy, the missing prerequisite, active
   attempt, or ambiguous external effect is always inspectable.
14. **Manual actions add goals.** Button clicks and API requests use the same machinery rather than
    bypassing reconciliation with a separate execution path.

Bringing up the astro pack must prove convergence with configured Source facets and a built-in astro
policy covering a realistic ingest flow. The system must explain policy and Operator matches, recover after a
deliberately dropped event and a process crash, avoid duplicating a successful external effect, and
identify an impossible goal without leaving opaque “stuck” work.

User-authored policy matching, conflict resolution, simulation, explanation UI, and policy editing
remain deferred until after cutover. Built-in domain-pack policies provide the required pre-cutover behaviour.
