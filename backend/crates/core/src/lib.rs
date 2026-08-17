//! Domain-agnostic core for Sidereal.
//!
//! Core owns the generic engine — Asset, Collection, Lineage, OperationRun,
//! Selector, facet storage/index, and the plugin registry — and knows nothing
//! about astronomy (ADR-002). It builds on the contracts in
//! [`sidereal_plugin_abi`]. This is an M0 placeholder; the real spine lands in
//! M1.

/// The plugin-ABI version this build of core was compiled against.
pub fn abi_version() -> u32 {
    sidereal_plugin_abi::ABI_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_compiled_abi_version() {
        assert_eq!(abi_version(), sidereal_plugin_abi::ABI_VERSION);
    }
}
