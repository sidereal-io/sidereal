---
id: adrs-adr009
date: 2026-08-19
status: accepted
title: 'ADR009: Backend Language and Runtime'
description: Architecture Decision Record (ADR) for the v2 backend language — Rust, chosen for a filesystem-mutating, CPU-bound engine with an embedded untrusted-plugin surface.
---

# ADR-009: Backend Language and Runtime

## Context

v2 turns Sidereal from an Immich viewer into the system of record for files on disk, and it runs a plugin engine on the ingest hot path.

The backend language must serve four things the current TypeScript/Hono stack serves weakly:

- **Hot-path throughput** — parsing and hashing hundreds of FITS/XISF frames per session.
- **Memory safety** on a component whose rename, move, and delete bugs destroy irreplaceable data.
- **Concurrency under bursty ingest** without GC-pause tail latency.
- **A credible in-process plugin-isolation story** with no published dynamic ABI.

Reversing this later means a second rewrite, so we decide it explicitly and up front.

## Decision

Write the v2 backend in **Rust.**

Its core is a filesystem-mutating, CPU-bound engine with an embedded untrusted-plugin surface. That is precisely where Rust's compile-time memory and data-race safety, native throughput, fearless concurrency, and mature in-process scripting (Rhai) pay off — and where Node's strengths do not apply.

The language switch is the top project risk, and we mitigate it deliberately. The frontend stays TypeScript/React as an independent workstream, so existing contributors stay productive. Heavy or OS-specific work (Python ML, Siril/PixInsight/ASTAP) runs as external providers behind the plugin contract, rather than being forced into Rust.

This fixes the language, not the framework and crate stack, which is settled in code.

## Consequences

- The hot path, the data-safety guarantees, and concurrent ingest are all on their strongest footing. The backend ships as a single static binary, supporting one-command deployment.
- The existing TypeScript backend is discarded, and the backend contributor pool shrinks. The mitigation protects the *frontend* contributor's productivity, but not backend bus-factor: a Rust system-of-record that mutates user files concentrates maintenance knowledge in whoever holds Rust expertise — an org risk to staff against.
- Choosing Rust for core does not force every integration into Rust. The external-provider profile keeps language and OS freedom at the edges.

## Alternatives Considered

### Alternative 1: Stay on TypeScript / Node
- **Pros:** keeps the existing backend, its contributors, and one language across the stack; fastest path to a running backend.
- **Cons:** CPU-bound FITS parsing and hashing push real work into native addons or WASM anyway, eroding the one-language benefit exactly where it matters; GC pauses hurt tail latency under bursty ingest; the safest in-process capability story is weaker than a systems language on the very component that mutates user files.
- **Why not:** it keeps the parts that are cheap and gives up the guarantees a system of record most needs.

### Alternative 2: Go
- **Pros:** simple, fast to learn, strong concurrency, single binary.
- **Cons:** weaker for CPU-bound numeric and parsing work, and still GC-paused; a less expressive type system for the Asset/Version/Lineage/facet model; no in-process embedding of Rhai's calibre.
- **Why not:** it is closer than Node, but it still trades away the numeric performance and type expressiveness the domain model leans on.
