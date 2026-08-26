# 012: Embedded Scripting Engine

**Status:** Proposed · **Date:** 2026-08-20

**Context:** Builds on [ADR-001 — Plugin contract & execution profiles](ADR-001-plugin-boundary.md),
which established an embedded-script execution profile as the default public extension surface and
deliberately left the engine choice open. This ADR picks that engine.

## Problem

The embedded-script profile runs untrusted, user-authored plugin code in-process, inside the Rust
backend. It needs an engine that is safe to embed and controllable by the host, because a plugin must
not be able to exhaust resources, block indefinitely, or reach past the capability-limited
`AssetContext`. The engine must support:

- **Cancellation** — the host can abort a running script mid-execution.
- **Operation and memory limits** — bounded work per call, so a script cannot hang or OOM the process.
- **Async host calls** — `AssetContext` operations are async (byte access, HTTP, emitting assets); the
  engine must call back into async host functions without blocking a runtime thread.
- **Manifest-driven loading** — a plugin bundle's declared entry points load and run deterministically.
- **A clean capability boundary** — the script sees only what the host injects, with no ambient
  filesystem, network, or process access.

The question is which embeddable engine best meets these under a Rust host, and how much confidence we
have before committing.

## Options

### Option A: Rhai

**Pros:**
- Purpose-built for embedding in Rust; native `rust`-side integration, no FFI.
- Built-in operation counting, and support for memory/depth limits and cancellation.
- Simple, familiar Rust-like syntax for plugin authors.

**Cons:**
- Interpreted and comparatively slow; unsuited to hot-path work (which is why hot paths stay built-in Rust).
- Async host calls need care — the engine is synchronous at its core.

### Option B: Rune

**Pros:**
- Designed for Rust embedding with first-class `async` support, easing async `AssetContext` calls.
- Modern language features and a bytecode VM.

**Cons:**
- Younger and less battle-tested than Rhai; smaller ecosystem.
- Resource-limit and sandboxing primitives less proven for hostile input.

### Option C: Lua (via mlua/rlua)

**Pros:**
- Mature, widely understood embedding language with a large author pool.
- Fast interpreter; long track record as an embedded scripting language.

**Cons:**
- Binds to a C library — adds a non-Rust build dependency and an FFI surface on the component that runs
  untrusted code.
- Async and fine-grained resource limits require extra host machinery rather than being native.

## Recommendation

**Option A — Rhai**, contingent on a spike. It is the most natural fit for an in-process Rust host and
ships the host-side controls the profile requires — operation limits and cancellation — without an FFI
dependency, which matters most on the component that runs untrusted code. Its weakness is performance,
but the profile is explicitly for lightweight Operators, Sources, and Sinks; anything performance-
sensitive stays in the built-in Rust profile.

Because this locks in the public extension surface, the choice is ratified only after a spike proves
Rhai against every requirement above — cancellation, operation/memory limits, async host calls, manifest
loading, and the `AssetContext` API — on a real plugin. **Rune and Lua remain fallbacks** if the spike
surfaces a blocker, Rune being the leading alternative for its native async.

## Decision

Pending the spike. On success, this ADR moves to Accepted with Rhai; on a blocker, it is re-decided in
favour of the surviving fallback.
