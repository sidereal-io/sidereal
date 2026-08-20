# 009: Backend Language and Runtime

**Status:** Accepted
**Date:** 2026-08-19
**Context:** The v2 rewrite needs a backend language; this ADR records *why it is Rust* rather than
leaving the single most expensive-to-reverse decision undocumented.

## Problem

v2 turns Sidereal from an Immich viewer into the **system of record for files on disk** and runs a
plugin engine on the ingest hot path. The backend language must serve four things the current
TypeScript/Hono stack serves weakly:

- **Hot-path throughput** — parsing and hashing hundreds of FITS/XISF frames per session.
- **Memory safety on a filesystem-mutating system of record** — a rename/move/delete bug here destroys
  irreplaceable user data.
- **Concurrency under bursty ingest** without GC-pause tail latency.
- **A credible plugin-isolation story** — an embeddable script engine and capability limits, with no
  published dynamic ABI.

Reversing this later means a second rewrite, so it is decided explicitly and up front.

## Options

### Option A: Stay on TypeScript / Node

**Pros:**
- Keeps the existing backend, its contributors, and one language across the whole stack.
- Fastest path to a running backend.

**Cons:**
- CPU-bound FITS parsing and hashing push the real work into native addons or WASM anyway, so the
  "one language" benefit erodes exactly where performance matters.
- GC pauses under bursty ingest hurt tail latency.
- The safest embedding/capability story (in-process script engine, no ambient FS) is weaker than a
  systems language offers, on the very component that mutates user files.

### Option B: Rust

**Pros:**
- Native hot-path performance for FITS/XISF parsing and hashing; mature image/numeric crates.
- Compile-time memory and data-race safety on the component that renames, moves, and deletes originals.
- Fearless concurrency for parallel ingest with no GC-pause tail.
- Mature embeddable scripting (Rhai) and a clean capability-limited `AssetContext`, matching the plugin execution model.
- Single static binary, supporting the one-command deployment goal.

**Cons:**
- A real language switch: steeper learning curve, a smaller contributor pool, and the existing TS
  backend is discarded. This is the RFC's named top risk.

### Option C: Go

**Pros:**
- Simple, fast to learn, strong concurrency, single binary.

**Cons:**
- Weaker than Rust for CPU-bound numeric/parsing work, and still GC-paused.
- A less expressive type system for the Asset/Version/Lineage/facet model, and no
  Rhai-in-process-caliber embedding story.

## Recommendation

**Option B — Rust.** The decisive factor is that v2's core is a filesystem-mutating, CPU-bound engine
with an embedded untrusted-plugin surface — precisely where Rust's safety, throughput, and embedding
maturity pay off and where Node's strengths do not apply. Go is closer than Node but gives up numeric
performance and type expressiveness for a domain model that leans on both.

The language switch is the RFC's top risk, mitigated deliberately: **the frontend stays TypeScript/React**
and moves as an independent workstream, so existing contributors stay productive through the switch;
and **heavy or OS-specific work stays out of the Rust core** — Python ML, Siril/PixInsight/ASTAP run as
external providers behind the plugin contract, so choosing Rust for core does not force every
integration into Rust. This decision fixes the language, not the framework/crate stack (web server, async runtime, image
libraries); those are ordinary implementation choices settled in code.

## Decision

Accepted 2026-08-19 — **Rust**, as recommended.
