# 001: Plugin Contract and Execution Profiles

**Status:** Proposed
**Date:** 2026-07-29
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). The plugin contract in [docs/architecture/plugins.md](../architecture/plugins.md) defines the common semantics; this ADR chooses how different classes of plugin execute and are installed.

## Problem

Every Sidereal v2 pipeline action uses the plugin contract, including built-ins. The original proposal
treated "plugin contract" and "process boundary" as one decision, but the workloads do not share one
useful boundary:

- FITS/XISF parsing and other hot paths need low overhead.
- User-authored rename, tag, metadata, and API orchestration should be easy to install and tightly
  capability-limited.
- Python ML, Siril, PixInsight, and ASTAP may require large native runtimes, another operating
  system, or process isolation.

Forcing all three through one runtime either makes the default install operationally complex or gives
third-party code more authority than it needs. The contract and its execution transport therefore
need separate decisions.

## Decision drivers

- **One semantic contract.** Source, Operator, and Sink results mean the same thing under every
  execution profile and run through the same conformance suite.
- **Core retains authority.** Plugins request effects through an `AssetContext`; they do not receive
  ambient write access to the asset store, arbitrary secrets, or an unrestricted process launcher.
- **Simple default deployment.** Installing a normal plugin must not require another container or
  language runtime.
- **Isolation where it matters.** A third-party native runtime may fail without taking down core.
- **No published Rust dynamic ABI.** Rust compiler ABI stability makes dylibs unsuitable as the
  third-party contract.

## Recommendation

Adopt a **capability-oriented hybrid** with three execution profiles.

| Profile | Intended use | Packaging and installation |
|---|---|---|
| **Built-in Rust** | Trusted, performance-sensitive first-party behavior: FITS/XISF readers, storage adapters, core astro operations | Compiled into the Sidereal binary as crates |
| **Embedded script** | Default public extension surface for lightweight Operators, Sources, and Sinks | Manifest plus Rhai source in a plugin bundle loaded by Sidereal |
| **External provider** | Python ML, Siril/PixInsight/ASTAP integration, hardware or OS-specific tools | Separately installed service or agent; the manifest configures its endpoint |

The initial scripting engine is **Rhai**, subject to an M0 spike proving cancellation, operation and
memory limits, async host calls, manifest loading, and the `AssetContext` API. Rune and Lua remain
alternatives if that spike fails. WASM is deferred until a concrete plugin demonstrates a need for
portable compiled components that the script profile cannot meet.

Sidereal v0.1 does **not** orchestrate arbitrary plugin containers, install Python environments, or
manage external-provider upgrades. An external provider is an explicit, user-managed dependency.
For a Windows-only PixInsight integration, for example, Sidereal calls a separately installed Windows
agent; from core's perspective it is an authenticated provider endpoint.

This is not the rejected form of "hybrid" where built-ins get an unconstrained private API. All
profiles implement the same request/result semantics and capability-specific conformance tests.
Transport adapters differ. At least two built-ins must also ship through the embedded-script profile
before that profile is declared stable; the initial candidates are tag/rename and API-based plate
solving.

## AssetContext

Every invocation receives a run-scoped `AssetContext`. It is the only route from plugin code to core
capabilities:

- read approved asset metadata and facets;
- request byte access as a read-only stream, descriptor, read-only mount, or disposable copy;
- emit new assets and proposed facet values;
- request core-managed rename, move, tag, and publish intents;
- perform allowlisted HTTP requests;
- access only manifest-declared, run-scoped secrets;
- report logs and progress and observe cancellation.

No profile receives a normal writable path into the asset store. If a legacy external tool requires a
path, core supplies a read-only mount or disposable workspace and verifies input hashes after the run.
Produced files are imported and hashed by core before becoming `AssetVersion` records.

Host functions must cooperate with cancellation and resource limits. Script operation limits alone do
not constrain a blocking native host call, so every host capability must have its own timeout and
cancellation behavior.

## Performance

The interface is batch-oriented: an Operator receives an asset set, not one IPC call per asset.
M0 benchmarks a representative 500+ frame session across the script and external-provider adapters.
The benchmark informs batch sizing and streaming; it does not reopen the semantic contract.

Large file bytes never cross a JSON or gRPC boundary by default. Co-located providers receive
read-only handles or disposable paths. Remote providers use explicit streaming or provider-managed
object transfer.

## Security and trust

Execution profile is not a trust level by itself. Built-in code is trusted first-party code; script
bundles and external providers receive explicit capabilities from their manifests and installation
approval. Authentication, endpoint trust, secret delivery, capability grants, and admin exposure are
defined by [ADR-007](ADR-007-security-and-plugin-trust.md).

## Consequences

- The default Sidereal deployment remains one binary/container for built-ins and script plugins.
- Lightweight extensions get a small, stable toolkit instead of host-language crate access.
- Heavy integrations retain language and operating-system freedom without becoming the universal
  plugin tax.
- The team maintains multiple transport adapters, but one semantic contract and conformance suite.
- A compiled built-in can still panic with core; only trusted first-party code belongs in that
  profile.
- WASM and container orchestration remain reversible additions rather than M0 prerequisites.

## Decision

[Filled in after review.]
