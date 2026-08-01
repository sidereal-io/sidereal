# Plugin Architecture

**Status:** Proposed · **Tracks:** [RFC #213](https://github.com/sidereal-io/sidereal/issues/213) · **Part of:** [Architecture](README.md)

Everything Sidereal v2 *does* to a user's files uses the plugin contract. This document defines what
a plugin receives, what it may request, what it returns, and how it proves conformance.

The contract is **transport-independent**. [ADR-001](../decisions/ADR-001-plugin-boundary.md) defines
three execution profiles — built-in Rust, embedded Rhai, and an external provider — over the same
semantics. Built-ins do not receive an unconstrained private API merely because they are compiled in.

## Contents

- [Why everything uses the contract](#why-everything-uses-the-contract)
- [Capabilities](#capabilities)
- [Manifest and registration](#manifest-and-registration)
- [What core owns](#what-core-owns)
- [AssetContext and execution model](#assetcontext-and-execution-model)
- [Side effects, retries, and cancellation](#side-effects-retries-and-cancellation)
- [Facet declaration](#facet-declaration)
- [Conformance suite](#conformance-suite)
- [Versioning](#versioning)

---

## Why everything uses the contract

An interface designed with zero real consumers is wrong in ways only discovered months later. The
countermeasure is a rule:

> **Built-in functionality uses the same semantic contract and conformance suite as third-party
> functionality. Execution transports may differ; behavior and authority do not.**

Plate solve, rename, move, and tag are the first four built-in Operators and the first consumers of
the Operator contract. At least two also ship through the embedded-script profile, so the public
script surface is exercised rather than inferred from a Rust implementation. If a built-in needs a
capability the interface lacks, the interface grows; it does not get a back door.

Source, Operator, and Sink are versioned and frozen **independently**. A capability reaches `v0.1`
only after at least one realistic implementation works through its public profile and passes that
capability's conformance suite. An Operator cannot validate the Source or Sink contract by proxy.

## Capabilities

Three capabilities, one registration mechanism. A plugin may declare more than one; Immich is both a
Source and a Sink.

### Source — produces assets

Given its configuration, a Source discovers files and hands them to core for ingestion. It reports
what it found; it does not write to the asset store itself.

A Source configuration may assign a default `kind` and configured facet values to each ingested
asset. A Source may propose detected facets only within its installation grants. These are
classification inputs to Core's Selectors; a Source never selects or invokes downstream Operators.
Mixed-kind Sources may leave the default `kind` unset and classify individual assets from their
ingested metadata.

Examples: watch folder, Immich import, NINA/SGP session output, manual upload.

A Source is responsible for **stable external identity**, scoped to the configured source instance.
The idempotency key is `(source_instance_id, external_id)`, where `external_id` may be an Immich UUID,
content hash, or URL hash. Two configured Immich servers can therefore expose the same upstream ID
without colliding.

### Operator — transforms assets

An **Operator** is an implementation that takes assets plus params and proposes mutations, new assets,
or both. An **Operation Run** is one recorded invocation of an Operator.

Each Operator also declares an `accepts` Selector, the semantic Processing Goals it can satisfy, the
facet, artifact, or receipt prerequisites for those goals, and which prior outcomes its mutations may
invalidate. Core's reconciler intersects policy selection, provided outcomes, `accepts`, grants, and
prerequisites to select eligible work. The Operator neither scans for assets nor chooses what runs
before or after it. See
[ADR-006](../decisions/ADR-006-rule-engine-deferral.md).

Examples today: plate solve, rename, move, tag, and extract metadata. Later: Siril invocation, AI
detection, and dedup.

An Operator may produce zero, one, or many new assets — thumbnails, master calibration frames, stacks,
and exports are all ordinary outputs. Operators never rewrite an existing content revision. Byte
changes produce a new immutable `AssetVersion`; path-only moves remain events on the stable `Asset`.
See [ADR-003](../decisions/ADR-003-storage-layout-and-asset-identity.md).

### Sink — publishes assets

A Sink takes assets and publishes them outside Sidereal, then reports what it published.

Examples: Immich, static gallery, Astrobin, S3.

A Sink never deletes a Sidereal asset. This is a direct lesson from v0.10.x, where an asset
disappearing from Immich deletes the Sidereal record — acceptable when Immich is the source of truth,
unacceptable once Sidereal owns the tree.

## Manifest and registration

A plugin ships a manifest declaring:

| Field | Purpose |
|---|---|
| `name`, `version` | Plugin identity. Version is semver. |
| `api_versions` | Independent Source, Operator, and Sink contract versions used by this plugin. Core refuses incompatible capabilities at load time. |
| `capabilities` | Which of `source` / `operator` / `sink` it implements, and the named Operators it exposes. |
| `execution` | `built_in`, `script`, or `external`, plus the entry point required by that profile. |
| `config_schema` | JSON Schema for configuration. Core renders admin UI and validates before delivery. |
| `facets` | Facet schemas declared and/or write grants requested, with types and index hints. |
| `processing` | `accepts` Selector, goal outcomes provided, prerequisite predicates, and invalidations declared by each Operator. |
| `capability_grants` | Requested host functions, allowlisted network destinations, byte-access mode, and secret names. Installation requires explicit approval. |
| `requires` | Declared external dependencies or provider endpoints so unmet requirements surface before a run. |

Built-in Rust plugins ship with Sidereal. Script plugins are manifest-plus-Rhai bundles loaded by
core. External providers are separately installed services or agents; Sidereal v0.1 validates and
connects to their endpoints but does not orchestrate their containers or language environments.

`config_schema` driving the admin UI is a deliberate improvement on v0.10.x, where the settings
document, defaults, masking rules, and React form are separate places that must agree.

## What core owns

The division of responsibility matters more than any individual signature.

**Core owns:**

- **The filesystem.** Plugins never write to the asset store directly. A requested rename, move, or
  import is validated and performed by core.
- **Asset identity, the metadata envelope, immutable versions, and lineage.** Plugins reference
  handles and propose authorised envelope mutations, inputs, and outputs; core validates and mints
  records and edges.
- **Facets and Selectors** — storage, indexing, deterministic evaluation, provenance, change events,
  and human-readable match explanations.
- **The job queue** — scheduling, concurrency, durable events, retries, cancellation, and progress
  fan-out.
- **Processing Goals and reconciliation** — policy evaluation, provider selection, prerequisite and
  cycle checks, missed-event recovery, and durable satisfaction evidence.
- **Config and capability validation** — against the manifest and installation grants.
- **Secrets.** A plugin receives only declared, run-scoped credentials.

**Plugins own:** domain logic, external protocols, declared vocabulary, and the facet values they are
authorised to propose. Core owns validation and persistence.

## AssetContext and execution model

Every invocation receives a run-scoped `AssetContext`. It exposes only approved capabilities:

- **Asset and immutable-version handles**, plus approved metadata and facets.
- **Explicit byte access** as a read-only descriptor/mount, stream, or disposable copy — never a
  normal writable asset-store path.
- **Validated params** conforming to the plugin schema.
- **Core capabilities** for proposed file intents, output import, authorised facet writes, allowlisted
  HTTP, scoped secrets, logging, progress, and cancellation.

An Operator returns status, proposed core-managed mutations, zero or more new assets, lineage
declarations, external receipts, goal-satisfaction evidence, and log output. Core validates the
complete result before committing core-managed effects.

Every run is recorded as an [Operation Run](README.md#operation-run): Operator and version, addressed
Processing Goals, inputs and input versions, params, outputs, status, side-effect state, and logs.

The execution profiles differ only in how requests and results cross the adapter. Built-in Rust uses
an in-process trait, Rhai uses registered host functions, and external providers use an authenticated
protocol. Shared fixtures and conformance tests verify the same semantics.

If a legacy external tool requires a path, core provides a read-only mount or disposable workspace
and verifies input hashes after execution. Produced files are imported and hashed before becoming
`AssetVersion` records.

**Progress is first-class.** v0.10.x demonstrates why: its separate plate-solving worker sends
progress to the server as a WebSocket client, but the server does not re-broadcast those frames.
Progress must flow through core, which owns fan-out.

## Side effects, retries, and cancellation

Every run receives a durable `run_id`, a run-scoped `idempotency_key`, and a declared side-effect
class:

| Class | Examples | Core behavior |
|---|---|---|
| **Pure** | Read metadata, calculate facets | Freely retryable |
| **Core-managed** | Import output, rename, move, tag | Validate first; commit transactionally where possible |
| **Idempotent external** | Upsert to an API with an idempotency key | Retry according to the provider contract |
| **Non-idempotent external** | CLI invocation or remote publish without idempotency support | No automatic retry after ambiguous failure |

Cancellation is cooperative. For pure and uncommitted core-managed work, cancellation leaves no
partial core mutation. Core cannot promise that an arbitrary external system rolled back. A cancelled
or disconnected external run records one of `not_started`, `not_committed`, `completed`, or
`may_have_completed_externally`, plus any provider-supplied compensation action.

Core records external request identifiers and checks provider status before retrying an ambiguous
run. The conformance suite verifies the behavior declared by each side-effect class instead of
requiring the impossible guarantee that every cancelled external action has no effect.

Host functions must cooperate with cancellation and resource limits. Script instruction limits alone
do not constrain a blocking native host call, so every host capability has its own timeout and
cancellation behavior.

## Facet declaration

A schema registry — typically contributed by a domain pack — declares canonical facet definitions:

```
astro.fits.*     schema owned by the astro pack
astro.solve.*    schema owned by the astro pack
photo.exif.*     schema owned by a general-photo pack
ai.faces.*       schema owned by an AI pack
```

Each facet has a name, type, index hint, schema owner, and allowed producer set. Core stores, indexes,
and queries facets without knowing their domain meaning.

Schema ownership is exclusive, but write authority is not. The astro pack can define
`astro.solve.ra`, while Astrometry.net and ASTAP Operators are separately granted permission to emit
that canonical facet. Every value records producer plugin and version provenance. Conflicting schema
declarations are a load error; unauthorised writes are rejected.

This separation preserves interoperable queries: calibration matching uses canonical `astro.fits.*`
values regardless of which compatible reader produced them. The namespace, compatibility, evolution,
and grant rules are [ADR-008](../decisions/ADR-008-facet-schema-and-write-authority.md).

## Conformance suite

Published per capability version. It asserts, at minimum:

- Manifest, configuration, and grants validate.
- Declared capabilities are implemented.
- Pure and declared-idempotent Operators are idempotent under re-run with identical inputs, params,
  and key.
- Cancellation and ambiguous external completion follow the declared side-effect class.
- Failure is reported as failure rather than silent success.
- No direct writes to the asset store.
- Only schema-compatible, authorised facets are emitted, with producer provenance.
- Inputs are not mutated behind core's back.
- Declared prerequisites, outcomes, and invalidations match observed results.
- `accepts` Selectors are deterministic and every selected or rejected subject can be explained.
- A repeated reconciliation pass does not duplicate a satisfied external effect.

Built-ins and transport adapters are the suite's first subjects. A built-in that cannot pass the
semantic suite is a bug in the built-in or interface — never an exemption.

## Versioning

Each capability contract is versioned independently. Core refuses an incompatible capability at load
time rather than failing partway through a run.

Operator API v0.1 is targeted for M2 after four built-ins, including at least two Rhai
implementations, consume it unchanged. Source API v0.1 follows real watch-folder and Immich Sources.
Sink API v0.1 follows a real Immich or filesystem-gallery Sink. A `v0.2` of each is expected — a
freeze buys a stable target, not permanent immutability.

---

## Related decisions

- **[ADR-001](../decisions/ADR-001-plugin-boundary.md)** — execution profiles and installation.
- **[ADR-002](../decisions/ADR-002-core-domain-pack-split.md)** — the core/domain-pack seam.
- **[ADR-003](../decisions/ADR-003-storage-layout-and-asset-identity.md)** — stable Assets and immutable
  AssetVersions.
- **[ADR-006](../decisions/ADR-006-rule-engine-deferral.md)** — declarative Processing Goals,
  reconciliation, and policy deferral.
- **[ADR-007](../decisions/ADR-007-security-and-plugin-trust.md)** — authentication, capability
  grants, endpoint trust, and secret delivery.
- **[ADR-008](../decisions/ADR-008-facet-schema-and-write-authority.md)** — facet schema ownership,
  write grants, provenance, and evolution.
