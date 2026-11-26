//! Proves the astro pack registers and is discoverable purely through
//! `plugin-abi` — no `core` involved (astro does not even depend on it).

use sidereal_pack_astro::AstroPack;
use sidereal_plugin_abi::{Output, Pack, Registry};

#[test]
fn astro_pack_registration_makes_it_discoverable() {
    let mut registry = Registry::new();
    registry.register_pack(&AstroPack).unwrap();

    assert_eq!(AstroPack.id(), "astro");
    assert!(registry.source_ids().contains(&"astro.fits"));
    assert!(registry.operator_ids().contains(&"astro.plate-solve"));
    assert!(registry.sink_ids().contains(&"astro.immich"));
}

struct Empty;
impl sidereal_plugin_abi::Candidate for Empty {
    fn size(&self) -> u64 {
        0
    }
    fn read_at(&mut self, _: u64, _: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

#[test]
fn the_fits_source_says_it_cannot_run_yet() {
    let mut registry = Registry::new();
    registry.register_pack(&AstroPack).unwrap();
    assert_eq!(
        registry.execute_source("astro.fits", &mut Empty, &mut |_| {}, &|| false),
        Err(sidereal_plugin_abi::ExecutionError::Unsupported)
    );
}

#[test]
fn operator_execution_of_the_plate_solve_stub_says_it_cannot_run_yet() {
    let mut registry = Registry::new();
    registry.register_pack(&AstroPack).unwrap();
    assert_eq!(
        registry.execute_operator(
            "astro.plate-solve",
            &mut Empty,
            "image/fits",
            &mut Discard,
            &mut |_| {},
            &|| false
        ),
        Err(sidereal_plugin_abi::ExecutionError::Unsupported)
    );
}

/// A host destination that drops every preview byte.
struct Discard;

impl Output for Discard {
    fn write(&mut self, _: &[u8]) -> std::io::Result<()> {
        Ok(())
    }
}
