//! Public plugin contracts for Sidereal.
//!
//! This crate holds the interface a domain pack codes against — the same
//! surface a third-party pack would use. The first-party astro pack depends on
//! this crate and never on `core` internals (ADR-002). Trait stubs
//! (`Source`/`Operator`/`Sink`, `AssetContext`, manifest/capability types) land
//! in a follow-up (#230); for now this is a placeholder that establishes the
//! seam so downstream crates can compile against it.

/// Version of the plugin ABI surface. Bumped when the contracts change; `0`
/// marks the pre-freeze M0 skeleton.
pub const ABI_VERSION: u32 = 0;
