---
id: adrs-adr001
date: 2026-08-18
status: accepted
title: 'ADR001: Plugin Contract and Execution Profiles'
description: Architecture Decision Record (ADR) for how different classes of plugin run and install behind one semantic contract — a capability-oriented hybrid of three execution profiles.
---

# ADR-001: Plugin Contract and Execution Profiles

## Context

All pipeline actions—including built-in features—use a single unified contract (Source / Operator / Sink).

However, different tasks require distinct execution environments, and no single runtime satisfies all needs:

- **Hot paths** (FITS/XISF parsing, storage adapters, core astro operations) need low overhead and run as trusted first-party code.
- **User-authored extensions** (rename, tag, metadata) must install without an extra container or runtime, and must stay tightly capability-limited.
- **Heavyweight tools** (Python ML, Siril, PixInsight, ASTAP) may need large native runtimes, another OS, or process isolation.

An execution profile defines _how_ a plugin runs, whereas permissions separately dictate _what_ actions it can perform.

## Decision

Implement a** capability-focused hybrid approach featuring three distinct execution profiles, all governed by a single semantic contract and conformance test suite**. The transport adapter is the sole component that varies across profiles..

| Profile | Use | Packaging |
|---|---|---|
| **Built-in Rust** | Trusted, performance-sensitive first-party behaviour | Compiled into the binary as crates |
| **Embedded script** | Default public extension surface for lightweight Operators/Sources/Sinks | Manifest + script source in a plugin bundle |
| **External provider** | Python ML, Siril/PixInsight/ASTAP, hardware- or OS-specific tools | Separately installed service; manifest configures its endpoint |

Plugins interact with core exclusively through a run-scoped **`AssetContext`** interface. This context grants plugins controlled capabilities—such as reading approved metadata and facets, accessing mediated file bytes, generating new assets and proposed facets, triggering core-managed actions (rename, move, tag, publish), initiating allowlisted HTTP requests, retrieving declared run-scoped secrets, logging output, reporting progress, and listening for cancellation signals. Direct writable access to the store is strictly prohibited across all execution profiles, as is ambient access to assets, secrets, system processes, or host network interfaces. When a plugin generates output files, core ingests them directly and computes hashes to create corresponding AssetVersion records.

A **published Rust dynamic ABI will not be provided**. Both container orchestration and WASM support have been postponed, though both can be added incrementally at a later date.

Selecting the engine for the embedded-script profile remains an independent decision that hinges on completing the scripting engine spike. This profile should only be marked as stable once **a minimum of two built-in extensions have been successfully delivered using it**.

## Consequences

- **Deployment Efficiency**: Standard deployments remain lightweight, packing built-ins and script plugins into a single binary or container. Resource-heavy integrations retain language and OS flexibility without burdening standard plugins.
- **Maintenance Overhead**: The engineering team must manage multiple transport adapters to support the single contract and conformance suite.
- **Blast Radius & Trust**: Because panics in compiled built-ins can crash core, this profile is strictly reserved for trusted first-party code.
- **Expanded External Capabilities**: External tools requiring write access (e.g., Siril, PixInsight) operate via disposable workspaces imported back by core rather than read-only mounts. Consequently, the external-provider profile inherently possesses a broader capability footprint than in-process profiles despite identical nominal semantics.
- **Stability Certification Risks**: The requirement of two built-in implementations could be met using trivial cases (like API plate-solving or tagging) that fail to test asynchronous cancellation, streaming large batches, or memory bounds under hostile inputs. This creates a risk of declaring the profile stable on simple workloads only to encounter failures under demanding third-party usage.

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
