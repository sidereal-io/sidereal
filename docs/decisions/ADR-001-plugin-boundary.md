# 001: Plugin Process Boundary

**Status:** Proposed
**Date:** 2026-07-29
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). The plugin contract in [docs/architecture/plugins.md](../architecture/plugins.md) is deliberately mechanism-independent; this ADR chooses the mechanism. Largest architectural call in M0.

## Problem

Every Sidereal v2 pipeline action is a plugin, including built-ins. How does a plugin execute relative to the core process?

The choice trades three things against each other:

- **Crash isolation** — a plugin invoking Siril or running Python ML code will eventually segfault, OOM, or hang. Does that take the server with it?
- **Per-asset latency** — operations run across thousands of frames per session. Boundary-crossing cost multiplies by asset count.
- **Authorship reach** — Rust-only plugin authorship is a much smaller contributor pool than "any language that can speak a protocol."

## Options

### Option A: Out-of-process (subprocess / gRPC)

**Pros:**
- Full crash and memory isolation; a dying plugin is a failed run, not an outage.
- Plugins authorable in any language — directly relevant, since AI/ML plugins will want Python and Siril orchestration is shelling out anyway.
- Resource limits and timeouts per plugin are straightforward.
- Natural fit for plugins that are already external processes.

**Cons:**
- Per-call IPC and serialization cost, multiplied across thousands of assets.
- Byte access needs a story that isn't "copy the FITS file across a socket" — shared filesystem paths or memory mapping.
- More operational surface: process supervision, lifecycle, zombie handling.

### Option B: In-process WASM

**Pros:**
- Strong sandboxing with much lower call overhead than IPC.
- Single-process deployment; no supervision.
- Capability-based security is natural.

**Cons:**
- Host-function plumbing needed for anything real (filesystem, network, subprocess).
- WASM cannot invoke Siril or a native ML runtime — the exact plugins the north star cares about.
- Language reach is nominally broad, practically narrow for numeric/imaging work.
- Large-file handling inside a WASM memory model is a research project.

### Option C: Native Rust dylib

**Pros:**
- Effectively zero call overhead; direct memory access to asset buffers.
- Simplest thing that could work for built-ins.

**Cons:**
- Zero isolation — a plugin bug is a server crash, and the north star explicitly anticipates third-party plugins.
- Rust-only authorship.
- Rust ABI stability across compiler versions makes a published third-party ABI painful.

### Option D: Hybrid — in-process for built-ins, out-of-process for third-party

**Pros:**
- Fast path for the operations that run per-asset at volume; isolation where untrusted code runs.

**Cons:**
- **Violates the rule that keeps the interface honest** — built-ins would consume a different path from third parties, which is precisely the drift approach C in the RFC exists to prevent.
- Two boundary implementations to maintain and test.

## Recommendation

**Leaning Option A (out-of-process).** A crashing Python AI plugin should not take down the server, and opening authorship beyond Rust is a stated goal. The RFC records this leaning without deciding it.

Two things to resolve before accepting:

1. **Latency.** Benchmark per-asset IPC across a realistic session (500+ frames). If per-call overhead dominates, the answer may be batching at the interface — pass asset *sets* rather than single assets — rather than changing the boundary.
2. **Byte access.** Decide how a plugin reads a 200 MB FITS file. Passing filesystem paths for co-located plugins, with copies only when a plugin is genuinely remote, is the obvious candidate and keeps [core's ownership of the filesystem](../architecture/plugins.md#what-core-owns) intact.

Option D should be rejected explicitly rather than left as a tempting later shortcut.

## Decision

[Filled in after review.]
