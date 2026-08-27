---
id: adrs-adr012
date: 2026-08-26
status: proposed
title: 'ADR012: Embedded Scripting Engine'
description: Architecture Decision Record (ADR) for which engine backs the embedded-script plugin profile — Rhai, pending a spike, with Rune and Lua as fallbacks.
---

# ADR-012: Embedded Scripting Engine

## Context

[ADR-001 — Plugin contract & execution profiles](ADR-001-plugin-boundary.md) established an embedded-script profile as the default public extension surface and deliberately left the engine open. This ADR picks it.

The profile runs untrusted, user-authored code in-process inside the Rust backend. So the engine must be safe to embed and controllable by the host. It needs:

- **Cancellation** — abort a running script mid-execution.
- **Operation and memory limits** — a script cannot hang or OOM the process.
- **Async host calls** — the `AssetContext` operations a script invokes are async.
- **Manifest-driven loading** — declared entry points load deterministically.
- **A clean capability boundary** — the script sees only what the host injects: no ambient filesystem, network, or process access.

Because this fixes the public extension surface, we commit only once it is proven.

## Decision

Use **Rhai, contingent on a spike** that proves it against every requirement above — cancellation, operation and memory limits, async host calls, manifest loading, and the `AssetContext` API — on a real plugin.

Rhai is purpose-built for embedding in Rust with no FFI. It ships host-side operation counting and cancellation, and it offers a familiar Rust-like syntax. Its weakness is interpreter performance, which does not matter here: anything performance-sensitive stays in the built-in Rust profile.

This ADR stays **Proposed** until the spike passes. On success, it moves to Accepted with Rhai. On a blocker, it is re-decided for the surviving fallback — Rune leading, for its native async.

## Consequences

- The public scripting surface is native to the Rust host, with no C or FFI dependency on the component that runs untrusted code. Host-side limits and cancellation come without extra machinery.
- Rhai is interpreted and comparatively slow, so the embedded profile is explicitly for lightweight Operators, Sources, and Sinks. Hot-path work belongs in built-in Rust regardless.
- Rhai's core is synchronous, so bridging it to async host calls needs care. This is the primary thing the spike must de-risk, and a failure here is the most likely reason to fall back.

## Alternatives Considered

### Alternative 1: Rune
- **Pros:** designed for Rust embedding with first-class `async`, easing async `AssetContext` calls; a modern bytecode VM.
- **Cons:** younger and less battle-tested than Rhai, with a smaller ecosystem and less proven resource-limit and sandboxing primitives for hostile input.
- **Why not:** it is the leading fallback, but its sandboxing maturity against untrusted code is unproven where Rhai's is established. Picked only if the Rhai spike surfaces an async blocker.

### Alternative 2: Lua (via mlua / rlua)
- **Pros:** a mature, widely understood embedding language with a large author pool and a fast interpreter.
- **Cons:** binds to a C library, adding a non-Rust build dependency and an FFI surface on the very component that runs untrusted code; fine-grained resource limits and async need extra host machinery rather than being native.
- **Why not:** an FFI boundary on the untrusted-code path is exactly the attack surface an in-process Rust engine avoids.
