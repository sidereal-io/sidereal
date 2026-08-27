---
id: adrs-adr012
date: 2026-08-26
status: proposed
title: 'ADR012: Embedded Scripting Engine'
description: Architecture Decision Record (ADR) for which engine backs the embedded-script plugin profile — Rhai, pending a spike, with Rune and Lua as fallbacks.
---

# ADR-012: Embedded Scripting Engine

## Context

[ADR-001 — Plugin contract & execution profiles](ADR-001-plugin-boundary.md) established an embedded-script profile as the default public extension surface and deliberately left the engine open; this ADR picks it. The profile runs untrusted, user-authored code in-process inside the Rust backend, so the engine must be safe to embed and controllable by the host: **cancellation** (abort a running script mid-execution), **operation and memory limits** (a script cannot hang or OOM the process), **async host calls** (the `AssetContext` operations a script invokes are async), **manifest-driven loading** (declared entry points load deterministically), and **a clean capability boundary** (the script sees only what the host injects — no ambient filesystem, network, or process access). Because this fixes the public extension surface, it is committed only once proven.

## Decision

Use **Rhai, contingent on a spike** that proves it against every requirement above — cancellation, operation/memory limits, async host calls, manifest loading, and the `AssetContext` API — on a real plugin. Rhai is purpose-built for embedding in Rust with no FFI, ships host-side operation counting and cancellation, and offers a familiar Rust-like syntax; its weakness is interpreter performance, which does not matter because anything performance-sensitive stays in the built-in Rust profile. This ADR stays **Proposed** until the spike passes: on success it moves to Accepted with Rhai; on a blocker it is re-decided for the surviving fallback, Rune leading for its native async.

## Consequences

- The public scripting surface is native to the Rust host with no C/FFI dependency on the component that runs untrusted code, and host-side limits and cancellation come without extra machinery.
- Rhai is interpreted and comparatively slow, so the embedded profile is explicitly for lightweight Operators, Sources, and Sinks; hot-path work belongs in built-in Rust regardless.
- Rhai's core is synchronous, so bridging it to async host calls needs care and is the primary thing the spike must de-risk; a failure there is the most likely reason to fall back.

## Alternatives Considered

### Alternative 1: Rune
- **Pros:** designed for Rust embedding with first-class `async`, easing async `AssetContext` calls; a modern bytecode VM.
- **Cons:** younger and less battle-tested than Rhai, with a smaller ecosystem and less proven resource-limit and sandboxing primitives for hostile input.
- **Why not:** it is the leading fallback, but its sandboxing maturity against untrusted code is unproven where Rhai's is established; picked only if the Rhai spike surfaces an async blocker.

### Alternative 2: Lua (via mlua / rlua)
- **Pros:** a mature, widely understood embedding language with a large author pool and a fast interpreter.
- **Cons:** binds to a C library, adding a non-Rust build dependency and an FFI surface on the very component that runs untrusted code; fine-grained resource limits and async need extra host machinery rather than being native.
- **Why not:** an FFI boundary on the untrusted-code path is exactly the attack surface an in-process Rust engine avoids.
