---
id: adrs-adr001
date: 2026-08-18
status: accepted
title: 'ADR001: Plugin Contract and Execution Profiles'
description: Architecture Decision Record (ADR) for how different classes of plugin run and install behind one semantic contract — a capability-oriented hybrid of three execution profiles.
---

# ADR-001: Plugin Contract and Execution Profiles

## Context

In v2, every pipeline action is a plugin — the built-ins included. So one semantic contract (Source / Operator / Sink) must cover all of them.

But the workloads behind that contract need very different runtimes, and no single runtime serves all three:

- **Hot paths** (FITS/XISF parsing, storage adapters, core astro operations) need low overhead and run as trusted first-party code.
- **User-authored extensions** (rename, tag, metadata) must install without an extra container or runtime, and must stay tightly capability-limited.
- **Heavyweight tools** (Python ML, Siril, PixInsight, ASTAP) may need large native runtimes, another OS, or process isolation.

This decision is paired with [ADR-007 — Security & plugin trust](ADR-007-security-and-plugin-trust.md). An execution profile is *how* code runs; that ADR governs *what* it may do.

## Decision

Adopt a **capability-oriented hybrid: three execution profiles behind one semantic contract and conformance suite.** Only the transport adapter differs between profiles.

| Profile | Use | Packaging |
|---|---|---|
| **Built-in Rust** | Trusted, performance-sensitive first-party behaviour | Compiled into the binary as crates |
| **Embedded script** | Default public extension surface for lightweight Operators/Sources/Sinks | Manifest + script source in a plugin bundle |
| **External provider** | Python ML, Siril/PixInsight/ASTAP, hardware- or OS-specific tools | Separately installed service; manifest configures its endpoint |

A run-scoped **`AssetContext` is the only route from plugin code to core.** Through it, a plugin can read approved metadata and facets, access bytes under mediation, emit new assets and proposed facets, request core-managed rename/move/tag/publish intents, make allowlisted HTTP calls, use run-scoped declared secrets, report logs and progress, and observe cancellation. No profile gets a writable path into the store, and none gets ambient access to assets, secrets, processes, or the network. Core imports the files a plugin produces and hashes them into `AssetVersion` records.

There is **no published Rust dynamic ABI.** WASM and container orchestration are deferred; both are additive later.

The embedded-script profile's scripting engine is a separate decision (ADR-012), and the profile depends on that engine spike. Declare the profile stable only after **at least two built-ins also ship through it.**

## Consequences

- The default deployment stays one binary or container for built-ins and script plugins. Heavy integrations keep their language and OS freedom without becoming a tax on every plugin.
- The team maintains several transport adapters against one contract and conformance suite.
- A compiled built-in can still panic and take down core, so only trusted first-party code belongs in that profile.
- Some external tools must *write* — PixInsight and Siril, for example. They cannot use a read-only mount, so core gives them a disposable workspace and imports from it. The external-provider profile's real capability envelope is therefore broader than the in-process ones, even under "identical semantics."
- The two-built-in stability bar risks being cleared on trivial candidates — tag/rename, API plate-solve — that never exercise async cancellation, memory limits under hostile input, or large-batch streaming. The profile could be declared "stable" on toy workloads and then break on the first demanding third-party plugin.

## Alternatives Considered

### Alternative 1: A single runtime for every plugin
- **Pros:** simplest mental model; one conformance path.
- **Cons:** no single runtime fits all three workloads.
- **Why not:** an in-process runtime over-grants third-party code and cannot host another OS or a large native tool. An out-of-process runtime taxes every hot path with IPC. Either choice forces a complex default install or an unsafe capability surface.

### Alternative 2: A published Rust dynamic ABI (dylibs) for third-party plugins
- **Pros:** native performance for third-party code; one language.
- **Cons:** Rust has no stable compiler ABI, so dylibs break across toolchain versions. In-process loading also grants third-party code core's own authority.
- **Why not:** it is neither a durable third-party contract nor an isolation boundary.

### Alternative 3: A WASM-first extension surface
- **Pros:** strong sandbox; portable compiled artifacts; language-agnostic.
- **Cons:** heavy up-front investment (host bindings, component toolchain). It also does not address the heavyweight external-tool case, which still needs its own runtime and isolation.
- **Why not:** it is premature before any plugin needs portable compiled components. Kept as a reversible later addition.
