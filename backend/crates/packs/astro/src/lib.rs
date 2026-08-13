//! First-party astrophotography domain pack.
//!
//! The astro pack is compiled into the v2.0 binary (ADR-002 Option A) but codes
//! against [`sidereal_plugin_abi`] — the same contract a third-party pack would
//! use — and never against `core` internals. That direction is enforced by
//! `backend/scripts/check-arch.sh`.
//!
//! M0 is a registration stub only: no FITS reading, no plate solving, no
//! ingest. Real registration through the plugin registry lands in #230.

/// The plugin-ABI version this pack was built against.
pub fn abi_version() -> u32 {
    sidereal_plugin_abi::ABI_VERSION
}
