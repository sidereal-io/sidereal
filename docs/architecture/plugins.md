# Plugin Architecture

**Status:** Proposed · **Tracks:** [RFC #213](https://github.com/sidereal-io/sidereal/issues/213) · **Part of:** [Architecture](README.md)

Everything Sidereal v2 *does* to a user's files happens in a plugin. This document is the contract:
what a plugin is, what it receives, what it may and may not do, and how it proves it conforms.

Deliberately **mechanism-independent**. Whether a plugin is a WASM module, a subprocess speaking gRPC,
or a native Rust dylib is [ADR-001](../decisions/ADR-001-plugin-boundary.md) and is not decided. Every
statement below holds under all three, which is what lets M3 and M5 be designed while ADR-001 is
still being argued.

## Contents

- [Why everything is a plugin](#why-everything-is-a-plugin)
- [Capabilities](#capabilities)
- [Manifest and registration](#manifest-and-registration)
- [What core owns](#what-core-owns)
- [Execution model](#execution-model)
- [Facet declaration](#facet-declaration)
- [Conformance suite](#conformance-suite)
- [Versioning](#versioning)

---

## Why everything is a plugin

An interface designed with zero real consumers is wrong in ways only discovered months later, after
people have built against it. The countermeasure is a rule, not good intentions:

> **Built-in functionality ships as a plugin over the public interface. No privileged internal path
> exists.**

Plate solve, rename, move, and tag are the first four built-ins, and they are the first four
consumers. If the interface can't express one of them, we find out immediately — in M2, not M6. If a
built-in needs a capability the interface lacks, the interface grows; it does not get a back door.

The interface is frozen as **ABI v0.1** only once all four exist and work through it unchanged. It is
versioned, and a `v0.2` is expected.

## Capabilities

Three capabilities, one registration mechanism. A plugin declares one or more. Immich implements two —
it is both a Source and a Sink.

### Source — produces assets

Given its configuration, a Source discovers files and hands them to core for ingestion. It reports
what it found; it does not write to the asset store itself.

Examples: watch folder, Immich import, NINA/SGP session output, manual upload.

A Source is responsible for **stable external identity** — the value that makes re-running it
idempotent. In v0.10.x this is already the shape that works: `(sourceType, sourceId)` where `sourceId`
is an Immich UUID, a content hash, or a URL hash depending on origin. v2 keeps that idea.

### Operation — transforms assets

Takes a set of assets plus params. Produces mutations to those assets, new assets, or both, plus a
status and log output.

Examples today: plate solve, rename, move, tag, extract metadata. Later: Siril invocation, AI
detection, dedup.

Operations are where the [identity tension](README.md#lineage) bites. An operation that rewrites bytes
must either produce a *new* asset with a lineage edge, or mutate an asset whose identity is
independent of its content — and which of those is legal is ADR-003.

### Sink — publishes assets outward

Takes assets and publishes them to somewhere outside Sidereal. Reports what it published.

Examples: Immich, static gallery, Astrobin, S3.

A Sink never deletes a Sidereal asset. This is a direct lesson from v0.10.x, where an asset
disappearing from Immich deletes the Sidereal record — acceptable when Immich is the source of truth,
unacceptable once Sidereal owns the tree.

## Manifest and registration

A plugin ships a manifest declaring:

| Field | Purpose |
|---|---|
| `name`, `version` | Identity. Version is semver. |
| `abi` | Which ABI version it targets. Core refuses to load an incompatible plugin rather than failing at runtime. |
| `capabilities` | Which of `source` / `operation` / `sink` it implements, and the named operations it exposes. |
| `config_schema` | JSON Schema for its configuration. Core renders admin UI and validates from this — the plugin never hand-parses config, and never receives config it hasn't declared. |
| `facets` | Namespaces this plugin owns and the facets within them, with types and index hints. See [Facet declaration](#facet-declaration). |
| `requires` | Declared external dependencies (a binary on PATH, a network host, an API key) so core can surface "plugin X is installed but Siril isn't on PATH" instead of failing mid-run. |

Discovery, and whether packs are compiled in or loaded at runtime, is
[ADR-002](../decisions/ADR-002-core-domain-pack-split.md).

`config_schema` driving the admin UI is a deliberate improvement on v0.10.x, where the settings
document, its defaults, its masking rules, and the React form that renders it are four separate places
that must agree — and don't (`metadataSyncEnabled`, `syncDescription`, `syncCoordinates`, and
`syncTags` are still rendered by the frontend with no code path consuming them).

## What core owns

The division of responsibility matters more than any individual interface signature.

**Core owns:**

- **The filesystem.** Plugins never write to the asset store directly. An operation that wants a file
  moved returns that intent; core performs it and records it. This is what makes the storage layout
  changeable without touching plugins, and what makes "Sidereal is the system of record for files on
  disk" safe rather than terrifying.
- **Asset identity and lineage.** Plugins reference assets by handle. They do not mint identities or
  write edges directly; core records lineage from what a run declares it consumed and produced.
- **The job queue** — scheduling, concurrency limits, retries, cancellation, progress fan-out.
- **Config validation** — against the plugin's own declared schema.
- **Secrets.** A plugin receives the credentials its schema declares, not the settings document.

**Plugins own:** their domain logic, their external protocol, their vocabulary, and their facets.

## Execution model

An Operation run receives:

- **Asset handles**, not paths — plus the metadata and facets it declared interest in. Byte access is
  requested explicitly, so core can decide whether that means a path, a stream, or a copy. This is
  what keeps large-FITS handling a core concern rather than a per-plugin one.
- **A validated params object** conforming to its schema.
- **A cancellation signal** and a **progress channel**.

It returns a result: status, mutations, new assets, lineage declarations, and log output.

Every run is recorded as an [Operation Run](README.md#operation-run) — which plugin, which inputs,
which params, what it produced. Re-runnability and history come from replaying that record, not from
reconstructing intent out of side effects.

**Progress is a first-class part of the contract, not a courtesy.** v0.10.x has a concrete failure here
worth not repeating: the plate-solving worker runs as a separate process and sends
`plate-solving-update` frames to the server *as a WebSocket client*, but the server never re-broadcasts
them — so in the production split-process topology, worker-applied results may reach browsers only on
manual refetch. Progress must flow through core, which owns the fan-out.

## Facet declaration

A plugin — typically a domain pack — declares the namespaces it owns:

```
astro.fits.*     owned by the astro pack (FITS reader)
astro.solve.*    owned by the astro pack (plate solve)
photo.exif.*     owned by a general-photo pack
ai.faces.*       owned by an AI pack
```

Each facet has a name, a type, and an index hint. Core stores, indexes, and queries facets. **Core does
not know what any of them mean.**

Two consequences worth stating:

- **Calibration-master matching is a facet query.** "Find a master dark for this camera at −10 °C, gain
  100, 300 s, bin 1×1" is a lookup over `astro.fits.*`, not bespoke schema. The astro pack contributes
  the matching *rule*; core contributes the query.
- **Per-kind pipelines fall out for free later.** A routing rule matches on kind and facets, so
  `kind = light` routes one way and `kind = dark` another, with no core changes. This is why the rule
  engine can safely be deferred to M7 ([ADR-006](../decisions/ADR-006-rule-engine-deferral.md)) — the
  mechanism it needs is being built anyway.

Namespace ownership is exclusive: two plugins cannot both declare `astro.solve.ra`. Conflicts are a
load error, not a runtime surprise.

## Conformance suite

Published alongside the ABI. Every plugin must pass it, and it is what makes third-party plugins safe
to install.

It asserts, at minimum:

- Manifest validates; declared capabilities are actually implemented.
- Config validation rejects what the schema forbids and accepts what it permits.
- Operations are **idempotent under re-run** with identical inputs and params.
- Cancellation is honoured, and a cancelled run leaves no partial mutation.
- Failure is reported as failure — a plugin that cannot do its job returns an error rather than
  silently succeeding. (v0.10.x has silent-failure bugs in exactly this shape.)
- No direct writes to the asset store.
- Declared facets are the only facets emitted.

The four built-ins are the suite's first subjects. A built-in that can't pass it is a bug in the
built-in or in the interface — never an exemption.

## Versioning

The ABI is versioned and plugins declare the version they target. Core refuses to load an incompatible
plugin instead of failing partway through a run.

ABI v0.1 is frozen at M2, after four built-in consumers exist. A `v0.2` is expected — the freeze buys
a stable target for third-party authors, not permanent immutability.

---

## Open

- **[ADR-001](../decisions/ADR-001-plugin-boundary.md) — the process boundary.** In-process WASM ·
  out-of-process subprocess/gRPC · native Rust dylib. Affects crash isolation, per-asset latency at
  scale, and whether plugins can be authored outside Rust. Leaning out-of-process; the open question is
  whether per-asset IPC holds up across thousands of frames.
- **[ADR-002](../decisions/ADR-002-core-domain-pack-split.md)** — where the core/pack seam falls, and
  whether packs are compiled in or loaded.
- **[ADR-003](../decisions/ADR-003-storage-layout-and-asset-identity.md)** — determines whether an
  Operation may mutate bytes in place at all.
