---
id: adrs-adr001
date: 2026-08-18
status: accepted
title: 'ADR001: Plugin Contract and Execution Profiles'
description: Architecture Decision Record (ADR) for how different classes of plugin execute and install behind one semantic contract — a capability-oriented hybrid of three execution profiles.
---

# ADR-001: Plugin Contract and Execution Profiles

## Context

Every v2 pipeline action is a plugin, built-ins included, so one semantic contract (Source / Operator / Sink) must cover all of them. But the workloads behind that contract have irreconcilable execution needs, and no single runtime serves all three: hot paths (FITS/XISF parsing, storage adapters, core astro ops) need low overhead and run as trusted first-party code; user-authored extensions (rename, tag, metadata) must install without an extra container or runtime and stay tightly capability-limited; heavyweight tools (Python ML, Siril, PixInsight, ASTAP) may need large native runtimes, another OS, or process isolation. This decision is coupled with [ADR-007 — Security & plugin trust](ADR-007-security-and-plugin-trust.md): execution profile is *how* code runs, that ADR is *what* it may do.

## Decision

Adopt a **capability-oriented hybrid: three execution profiles behind one semantic contract and conformance suite**, where only the transport adapter differs:

| Profile | Use | Packaging |
|---|---|---|
| **Built-in Rust** | Trusted, performance-sensitive first-party behaviour | Compiled into the binary as crates |
| **Embedded script** | Default public extension surface for lightweight Operators/Sources/Sinks | Manifest + script source in a plugin bundle |
| **External provider** | Python ML, Siril/PixInsight/ASTAP, hardware- or OS-specific tools | Separately installed service; manifest configures its endpoint |

A run-scoped **`AssetContext` is the only route from plugin code to core** — approved metadata/facets, mediated byte access, emitting new assets and proposed facets, core-managed rename/move/tag/publish intents, allowlisted HTTP, run-scoped declared secrets, and logs/progress/cancellation. No profile gets a writable path into the store or ambient asset/secret/process/network access; core imports and hashes produced files into `AssetVersion` records. There is **no published Rust dynamic ABI**, and WASM and container orchestration are deferred, additive later. The embedded-script profile's scripting engine is a separate decision (ADR-012), and the profile's viability is contingent on that engine spike; the profile is declared stable only after **≥2 built-ins also ship through it**.

## Consequences

- The default deployment stays one binary/container for built-ins and script plugins; heavy integrations keep language and OS freedom without becoming a universal plugin tax.
- The team maintains several transport adapters against one contract and conformance suite.
- A compiled built-in can still panic with core, so only trusted first-party code belongs in that profile.
- External tools that must *write* (PixInsight, Siril) cannot use a read-only mount; they get a disposable workspace core imports from — so the external-provider profile's real capability envelope is broader than the in-process ones even under "identical semantics."
- The ≥2-built-in stability bar risks being cleared on trivial candidates (tag/rename, API plate-solve) that never exercise async cancellation, memory limits under hostile input, or large-batch streaming — the profile could be declared "stable" on toy workloads and break on the first demanding third-party plugin.

## Alternatives Considered

### Alternative 1: A single runtime for every plugin
- **Pros:** simplest mental model; one conformance path.
- **Cons:** no single runtime fits all three workloads.
- **Why not:** an in-process runtime over-grants third-party code and cannot host another OS or a large native tool; an out-of-process runtime taxes every hot path with IPC. Either forces a complex default install or an unsafe capability surface.

### Alternative 2: A published Rust dynamic ABI (dylibs) for third-party plugins
- **Pros:** native performance for third-party code; one language.
- **Cons:** Rust has no stable compiler ABI, so dylibs break across toolchain versions; in-process loading grants third-party code core's own authority.
- **Why not:** it is neither a durable third-party contract nor an isolation boundary.

### Alternative 3: A WASM-first extension surface
- **Pros:** strong sandbox, portable compiled artifacts, language-agnostic.
- **Cons:** heavy up-front investment (host bindings, component toolchain); does not address the heavyweight-external-tool case, which still needs its own runtime and isolation.
- **Why not:** premature before any plugin needs portable compiled components; kept as a reversible later addition.
