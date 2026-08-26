# 001: Plugin Contract and Execution Profiles

**Status:** Accepted · **Date:** 2026-07-29

**Context:** This ADR chooses how different classes of plugin execute and are installed, behind one
common semantic contract. It is coupled with [ADR-007 — Security & plugin
trust](ADR-007-security-and-plugin-trust.md): execution profile says how code runs; that decision says
what it may do.

## Problem

Every v2 pipeline action is a plugin, built-ins included, so a single semantic contract
(Source/Operator/Sink) must cover all of them. But the workloads behind that contract have
irreconcilable execution needs, and no single runtime serves all three:

- **Hot paths** — FITS/XISF parsing, storage adapters, core astro operations — need low overhead and
  run as trusted first-party code.
- **User-authored extensions** — rename/tag/metadata work — must be easy to install (no extra container
  or language runtime) and tightly capability-limited, because they are third-party.
- **Heavyweight external tools** — Python ML, Siril, PixInsight, ASTAP — may need large native runtimes,
  another OS, or process isolation so a failure cannot take down core.

Forcing all three through one runtime either makes the default install complex or over-grants
third-party code. So the question is: **how many execution transports, and which, sit behind the one
semantic contract** — while keeping that contract identical across every profile so a result means the
same thing however it ran.

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

## Options

### Option A: Single runtime for everything
One transport behind the contract — every plugin, built-in or third-party, runs the same way.
**Pros:**
- Simplest mental model; one conformance path.

**Cons:**
- No single choice fits all three workloads: an in-process runtime over-grants third-party code and
  can't host another OS/large native tool; an out-of-process runtime taxes every hot path with IPC.
- Forces either a complex default install or an unsafe capability surface.

### Option B: Published Rust dynamic ABI (dylibs) for third-party plugins
Third parties compile against a stable Rust ABI and ship dynamic libraries loaded into core.
**Pros:**
- Native performance for third-party code; one language.

**Cons:**
- Rust has no stable compiler ABI — dylibs break across toolchain versions, so this is not a durable
  third-party contract.
- In-process loading gives third-party code the same authority as core; no isolation.

### Option C: WASM-first extension surface
Third-party plugins ship as portable WASM components sandboxed by the host.
**Pros:**
- Strong sandbox, portable compiled artifacts, language-agnostic.

**Cons:**
- Heavier up-front investment (host bindings, the component toolchain) before any plugin needs portable
  compiled components.
- Doesn't address the heavyweight-external-tool case (Python/Siril/PixInsight still need their own
  runtimes and isolation).

### Option D: Capability-oriented hybrid — three profiles, one contract *(recommended)*
Three execution profiles — **built-in Rust**, **embedded script**, **external provider** — behind one
semantic contract and conformance suite; each workload uses the profile that fits, and only the
transport adapter differs.
**Pros:**
- Each workload gets the right transport: low-overhead built-ins, an easy-to-install capability-limited
  script surface, and isolated external providers.
- Simple default install (no container/runtime for a normal plugin); isolation only where it's needed.
- WASM and container orchestration stay reversible later additions, not prerequisites.

**Cons:**
- More than one transport adapter to build and keep conformant against the shared contract.
- Requires discipline that built-ins get **no** privileged private API — otherwise it degrades into a
  two-tier system where built-ins and plugins diverge.

## Recommendation

**Option D — the capability-oriented hybrid.** It is the only option that serves all three workloads
without over-granting or over-building. Three profiles sit behind one contract; only the transport
adapter differs, and every profile runs the identical conformance suite:

| Profile | Intended use | Packaging |
|---|---|---|
| **Built-in Rust** | Trusted, performance-sensitive first-party behavior: FITS/XISF readers, storage adapters, core astro operations | Compiled into the binary as crates |
| **Embedded script** | Default public extension surface for lightweight Operators, Sources, Sinks | Manifest + script source in a plugin bundle |
| **External provider** | Python ML, Siril/PixInsight/ASTAP, hardware- or OS-specific tools | Separately installed service; manifest configures its endpoint |

Two invariants are non-negotiable, or the hybrid degrades into a two-tier system:

- **No privileged built-in API.** Built-ins get no back door — same request/result semantics and
  conformance tests as any plugin.
- **`AssetContext` is the only route from plugin code to core.** No profile gets a writable path into the
  store or ambient asset/secret/process/network access; core imports and hashes produced files into
  `AssetVersion` records. The full capability enumeration belongs to the plugin contract.

The engine for the embedded-script profile is chosen downstream, not here. A portable compiled-component
surface (WASM, Option C) is deferred until a plugin needs one the script profile cannot serve, as is
container orchestration and external-provider lifecycle management — an external provider is a
user-managed dependency. Both remain additive later, not prerequisites. The interface is batch-oriented
(an Operator receives an asset set) and large file bytes never cross the request/response boundary;
exact batch sizing is tuning, not part of this decision. The embedded-script profile is declared stable
only after **≥2 built-ins also ship through it**, forcing it to prove itself on first-party work before
third parties depend on it.

## Decision

Accepted 2026-08-18 — the **capability-oriented hybrid**, as recommended. The choice of embedded
scripting engine is left to a downstream decision, to be proven against the profile's requirements before
it is committed.
