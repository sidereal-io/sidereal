# 001: Plugin Contract and Execution Profiles

**Status:** Accepted
**Date:** 2026-07-29
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). The plugin contract in
[plugins.md](../architecture/plugins.md) defines the common semantics; this ADR chooses how different
classes of plugin execute and are installed.

## Problem

Every v2 pipeline action uses the plugin contract, including built-ins. The original proposal treated
"plugin contract" and "process boundary" as one decision, but the workloads do not share one useful
boundary: FITS/XISF parsing and other hot paths need low overhead; user-authored rename/tag/metadata
work should be easy to install and tightly capability-limited; Python ML, Siril, PixInsight, and ASTAP
may need large native runtimes, another OS, or process isolation. Forcing all three through one runtime
either makes the default install complex or over-grants third-party code. The contract and its
execution transport therefore need separate decisions.

## Decision drivers

- **One semantic contract.** Source/Operator/Sink results mean the same thing under every profile and
  run through the same conformance suite.
- **Core retains authority.** Plugins request effects through a capability-limited `AssetContext`; no
  ambient asset-store, secret, process, or network access.
- **Simple default deployment.** Installing a normal plugin must not require another container or
  language runtime.
- **Isolation where it matters.** A third-party native runtime may fail without taking down core.
- **No published Rust dynamic ABI** — compiler ABI instability rules out dylibs as the third-party
  contract.

## Recommendation

A **capability-oriented hybrid** with three execution profiles behind one semantic contract:

| Profile | Intended use | Packaging |
|---|---|---|
| **Built-in Rust** | Trusted, performance-sensitive first-party behavior: FITS/XISF readers, storage adapters, core astro operations | Compiled into the binary as crates |
| **Embedded script** | Default public extension surface for lightweight Operators, Sources, Sinks | Manifest + Rhai source in a plugin bundle |
| **External provider** | Python ML, Siril/PixInsight/ASTAP, hardware- or OS-specific tools | Separately installed service; manifest configures its endpoint |

The initial scripting engine is **Rhai**, subject to an M0 spike proving cancellation, operation/memory
limits, async host calls, manifest loading, and the `AssetContext` API; Rune and Lua remain fallbacks.
WASM is deferred until a concrete plugin needs portable compiled components the script profile cannot
meet. This is **not** the rejected "hybrid" where built-ins get an unconstrained private API — all
profiles implement identical request/result semantics and capability-specific conformance tests; only
the transport adapter differs.

The interface is batch-oriented (an Operator receives an asset set, not one call per asset) and large
file bytes never cross a JSON/gRPC boundary — co-located providers get read-only handles or disposable
paths, remote providers use explicit streaming. An M0 benchmark on a 500+ frame session sizes batching
without reopening the semantic contract.

## Decision

Accepted 2026-08-18 (M0 of RFC #213). Adopt the **capability-oriented hybrid** above: the three
execution profiles behind one semantic contract, each passing the same capability-specific conformance
suite with only the transport differing.

- **`AssetContext` is the only route from plugin code to core** — approved metadata/facets, mediated
  byte access (read-only stream, descriptor, mount, or disposable copy), emitting new assets and
  proposed facets, core-managed rename/move/tag/publish intents, allowlisted HTTP, manifest-declared
  run-scoped secrets, and logs/progress/cancellation. No profile gets a writable path into the store;
  core imports and hashes produced files into `AssetVersion` records. Host capabilities each carry their
  own timeout and cancellation. The full enumeration lives in [plugins.md](../architecture/plugins.md).
- **Execution profile is not a trust level.** Built-in Rust is trusted first-party code; script bundles
  and external providers get explicit capabilities at install. Authentication, grants, and secret
  delivery are [ADR-007](ADR-007-security-and-plugin-trust.md).
- **No published Rust dynamic ABI; WASM deferred; Rhai contingent on the M0 spike** (Rune/Lua as
  fallbacks). v0.1 does **not** orchestrate plugin containers, provision Python environments, or manage
  external-provider upgrades — an external provider is a user-managed dependency.
- The embedded-script profile is declared stable only after **≥2 built-ins also ship through it**
  (initial candidates: tag/rename and API-based plate solving).

A compiled built-in can still panic with core, so only trusted first-party code belongs in that
profile; WASM and container orchestration remain reversible later additions, not M0 prerequisites.
