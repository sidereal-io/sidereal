//! First-party astrophotography domain pack.
//!
//! The astro pack is compiled into the v2.0 binary (ADR-002 Option A) but codes
//! against [`sidereal_plugin_abi`] — the same contract a third-party pack would
//! use — and never against `core` internals. That direction is enforced by
//! `backend/scripts/check-arch.sh`.
//!
//! M0 is registration only: the components below are id-bearing stubs with no
//! behaviour yet (no FITS reading, no plate solving, no ingest).

use sidereal_plugin_abi::{Operator, Pack, Registry, Sink, Source};

/// The astrophotography domain pack.
pub struct AstroPack;

impl Pack for AstroPack {
    fn id(&self) -> &str {
        "astro"
    }

    fn register(&self, registry: &mut Registry) {
        registry.register_source(Box::new(FitsSource));
        registry.register_operator(Box::new(PlateSolveOperator));
        registry.register_sink(Box::new(ImmichSink));
    }
}

/// Reads FITS/XISF frames into assets (M0 stub).
struct FitsSource;
impl Source for FitsSource {
    fn id(&self) -> &str {
        "astro.fits"
    }
}

/// Plate-solves frames against a catalog (M0 stub).
struct PlateSolveOperator;
impl Operator for PlateSolveOperator {
    fn id(&self) -> &str {
        "astro.plate-solve"
    }
}

/// Exports processed frames to Immich (M0 stub).
struct ImmichSink;
impl Sink for ImmichSink {
    fn id(&self) -> &str {
        "astro.immich"
    }
}

/// The plugin-ABI version this pack was built against.
pub fn abi_version() -> u32 {
    sidereal_plugin_abi::ABI_VERSION
}
