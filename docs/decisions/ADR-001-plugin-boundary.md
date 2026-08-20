# 001: Plugin Contract and Execution Profiles

**Status:** Accepted
**Date:** 2026-07-29
**Context:** This ADR chooses how different classes of plugin execute and are installed, behind one
common semantic contract. It is coupled with [ADR-007 — Security & plugin
trust](ADR-007-security-and-plugin-trust.md): execution profile says how code runs; that decision says
what it may do.

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

The initial scripting engine is **Rhai**, subject to a spike proving cancellation, operation/memory
limits, async host calls, manifest loading, and the `AssetContext` API; Rune and Lua remain fallbacks.
WASM is deferred until a concrete plugin needs portable compiled components the script profile cannot
meet. This is **not** the rejected "hybrid" where built-ins get an unconstrained private API — all
profiles implement identical request/result semantics and capability-specific conformance tests; only
the transport adapter differs.

The interface is batch-oriented (an Operator receives an asset set, not one call per asset) and large
file bytes never cross a JSON/gRPC boundary — co-located providers get read-only handles or disposable
paths, remote providers use explicit streaming. A benchmark on a 500+ frame session sizes batching
without reopening the semantic contract.

**`AssetContext` is the only route from plugin code to core** — approved metadata/facets, mediated byte
access (read-only stream, descriptor, mount, or disposable copy), emitting new assets and proposed
facets, core-managed rename/move/tag/publish intents, allowlisted HTTP, manifest-declared run-scoped
secrets, and logs/progress/cancellation. No profile gets a writable path into the store; core imports
and hashes produced files into `AssetVersion` records. Host capabilities each carry their own timeout
and cancellation. The full capability enumeration is part of the plugin contract.

Execution profile is not a trust level: built-in Rust is trusted first-party code, while script bundles
and external providers get explicit capabilities at install; authentication, grants, and secret delivery
are settled separately (the coupled trust decision). There is **no published Rust dynamic ABI**. v0.1 does not orchestrate plugin containers, provision Python environments, or manage
external-provider upgrades — an external provider is a user-managed dependency. The embedded-script
profile is declared stable only after **≥2 built-ins also ship through it** (initial candidates:
tag/rename and API-based plate solving). A compiled built-in can still panic with core, so only trusted
first-party code belongs in that profile; WASM and container orchestration remain reversible later
additions, not up-front prerequisites.

## Decision

Accepted 2026-08-18 — the **capability-oriented hybrid**, as recommended. The embedded scripting engine
(**Rhai**) is contingent on the spike, with Rune/Lua as fallbacks.
