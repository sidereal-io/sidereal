//! Public plugin contracts for Sidereal.
//!
//! This crate holds the interface a domain pack codes against — the same
//! surface a third-party pack would use. The first-party astro pack depends on
//! this crate and never on `core` internals (ADR-002).
//!
//! The contracts here are intentionally minimal for M0 — trait stubs and a
//! registry that make the registration path *real*, not a frozen ABI. They are
//! expected to churn until M2 freezes them.

/// Version of the plugin ABI surface. Bumped when the contracts change; `0`
/// marks the pre-freeze M0 skeleton.
pub const ABI_VERSION: u32 = 0;

/// Produces assets into the system (e.g. a FITS reader).
pub trait Source: Send + Sync {
    /// Stable, namespaced identifier, e.g. `"astro.fits"`.
    fn id(&self) -> &str;
}

/// Transforms assets (e.g. plate solving).
pub trait Operator: Send + Sync {
    /// Stable, namespaced identifier, e.g. `"astro.plate-solve"`.
    fn id(&self) -> &str;
}

/// Exports assets to an external system (e.g. Immich).
pub trait Sink: Send + Sync {
    /// Stable, namespaced identifier, e.g. `"astro.immich"`.
    fn id(&self) -> &str;
}

/// The registry a pack contributes its components through — the same entry
/// point a third-party pack uses. Packs register here and never touch `core`
/// internals.
#[derive(Default)]
pub struct Registry {
    sources: Vec<Box<dyn Source>>,
    operators: Vec<Box<dyn Operator>>,
    sinks: Vec<Box<dyn Sink>>,
}

impl Registry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a [`Source`].
    pub fn register_source(&mut self, source: Box<dyn Source>) {
        self.sources.push(source);
    }

    /// Register an [`Operator`].
    pub fn register_operator(&mut self, operator: Box<dyn Operator>) {
        self.operators.push(operator);
    }

    /// Register a [`Sink`].
    pub fn register_sink(&mut self, sink: Box<dyn Sink>) {
        self.sinks.push(sink);
    }

    /// Ids of all registered sources, in registration order.
    pub fn source_ids(&self) -> Vec<&str> {
        self.sources.iter().map(|s| s.id()).collect()
    }

    /// Ids of all registered operators, in registration order.
    pub fn operator_ids(&self) -> Vec<&str> {
        self.operators.iter().map(|o| o.id()).collect()
    }

    /// Ids of all registered sinks, in registration order.
    pub fn sink_ids(&self) -> Vec<&str> {
        self.sinks.iter().map(|s| s.id()).collect()
    }
}

/// Implemented by a domain pack to contribute its components. The host builds a
/// [`Registry`] and calls [`Pack::register`] on each pack; this is the entire
/// contact surface between a pack and the host.
pub trait Pack {
    /// Stable pack identifier, e.g. `"astro"`.
    fn id(&self) -> &str;

    /// Register this pack's sources, operators, and sinks into `registry`.
    fn register(&self, registry: &mut Registry);
}
