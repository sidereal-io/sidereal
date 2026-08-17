//! Proves the astro pack registers and is discoverable purely through
//! `plugin-abi` — no `core` involved (astro does not even depend on it).

use sidereal_pack_astro::AstroPack;
use sidereal_plugin_abi::{Pack, Registry};

#[test]
fn astro_pack_registers_and_is_discoverable() {
    let mut registry = Registry::new();
    AstroPack.register(&mut registry);

    assert_eq!(AstroPack.id(), "astro");
    assert!(registry.source_ids().contains(&"astro.fits"));
    assert!(registry.operator_ids().contains(&"astro.plate-solve"));
    assert!(registry.sink_ids().contains(&"astro.immich"));
}
