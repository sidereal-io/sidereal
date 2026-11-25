//! Public plugin contracts for Sidereal.
//!
//! This crate holds the interface a domain pack codes against — the same
//! surface a third-party pack would use. The first-party astro pack depends on
//! this crate and never on `core` internals (ADR-002).
//!
//! The contracts here are intentionally minimal and not a frozen ABI. They
//! are expected to churn until M2 freezes them.
//!
//! A [`Source`] runs through an [`AssetContext`]: the context is the only way
//! it reaches the candidate's bytes, and it can propose one Asset. The host
//! keeps identity, hashing and provenance; a Source never sets them.

use std::fmt;

/// Version of the plugin ABI surface. Bumped when the contracts change.
///
/// - `0`: the M0 skeleton, with id-only stubs.
/// - `1`: a [`Source`] reports its version and runs through an
///   [`AssetContext`]. Every Source must implement [`Source::run`].
pub const ABI_VERSION: u32 = 1;

/// Brings files into the library as Assets (e.g. manual upload).
pub trait Source: Send + Sync {
    /// Stable, namespaced identifier, e.g. `"astro.fits"`.
    fn id(&self) -> &str;

    /// The implementation's version, recorded in each Asset version's
    /// provenance.
    fn version(&self) -> &str;

    /// Look at one candidate through `context` and propose at most one
    /// Asset, or refuse it.
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), SourceError>;
}

/// How a [`Source`] run ends when it does not accept the candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceError {
    /// The candidate is not something this Source accepts.
    Refused(String),
    /// This Source cannot run yet.
    Unsupported,
    /// The Source could not finish.
    Failed(String),
}

/// The bytes of one candidate, as the host exposes them to a run.
pub trait Candidate {
    /// The candidate's size in bytes.
    fn size(&self) -> u64;

    /// Read bytes starting at `offset` into `buf`, returning how many were
    /// read. Zero means the end of the candidate.
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize>;
}

/// What a Source proposes about a candidate: only its kind and its
/// recognized media type. The host supplies the name, ids and hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetProposal {
    pub kind: String,
    pub media_type: String,
}

impl AssetProposal {
    pub fn new(kind: impl Into<String>, media_type: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            media_type: media_type.into(),
        }
    }

    /// A kind is one lowercase token; a media type is `type/subtype` with no
    /// parameters, so the host can send it as a header unchanged.
    fn is_well_formed(&self) -> bool {
        fn token(text: &str) -> bool {
            !text.is_empty()
                && text
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"+-.".contains(&b))
        }
        token(&self.kind)
            && self
                .media_type
                .split_once('/')
                .is_some_and(|(kind, subtype)| token(kind) && token(subtype))
    }
}

/// Which Source produced an Asset version, taken from the registered Source
/// that ran, never from what the Source proposed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    pub source_id: String,
    pub source_version: String,
}

/// A run that ended with one valid proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accepted {
    pub proposal: AssetProposal,
    pub provenance: Provenance,
}

/// Why a Source run did not produce an accepted proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionError {
    /// No Source with that id is registered.
    UnknownSource,
    /// The Source refused the candidate.
    Refused(String),
    /// The Source cannot run yet.
    Unsupported,
    /// The Source failed, or broke the context's rules.
    Failed(String),
    /// The run ended without a proposal.
    NoProposal,
    /// The host cancelled the run.
    Cancelled,
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSource => f.write_str("no such source is registered"),
            Self::Refused(reason) => write!(f, "the source refused the candidate: {reason}"),
            Self::Unsupported => f.write_str("the source cannot run yet"),
            Self::Failed(reason) => write!(f, "the source failed: {reason}"),
            Self::NoProposal => f.write_str("the source proposed no asset"),
            Self::Cancelled => f.write_str("the run was cancelled"),
        }
    }
}

impl std::error::Error for ExecutionError {}

/// Why the context refused an operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextError {
    /// Reading the candidate failed.
    Read,
    /// The run already proposed an Asset.
    AlreadyProposed,
    /// The proposal's kind or media type is not well formed.
    InvalidProposal,
}

/// The run-scoped context a [`Source`] works through. It exposes no path,
/// database handle or identity setter.
pub struct AssetContext<'a> {
    candidate: &'a mut dyn Candidate,
    log: &'a mut dyn FnMut(&str),
    cancelled: &'a dyn Fn() -> bool,
    proposal: Option<AssetProposal>,
    broken: Option<String>,
}

impl AssetContext<'_> {
    /// The candidate's size in bytes.
    pub fn candidate_size(&self) -> u64 {
        self.candidate.size()
    }

    /// Read candidate bytes starting at `offset`. One call reads at most
    /// [`AssetContext::MAX_READ`] bytes and never past the candidate's end.
    pub fn read_candidate(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize, ContextError> {
        let remaining = self.candidate.size().saturating_sub(offset);
        let len = buf.len().min(Self::MAX_READ).min(remaining as usize);
        if len == 0 {
            return Ok(0);
        }
        self.candidate
            .read_at(offset, &mut buf[..len])
            .map(|n| n.min(len))
            .map_err(|_| ContextError::Read)
    }

    /// Propose the one Asset this run produces.
    pub fn propose_asset(&mut self, proposal: AssetProposal) -> Result<(), ContextError> {
        if self.proposal.is_some() {
            self.broken = Some("proposed more than one asset".into());
            return Err(ContextError::AlreadyProposed);
        }
        if !proposal.is_well_formed() {
            return Err(ContextError::InvalidProposal);
        }
        self.proposal = Some(proposal);
        Ok(())
    }

    /// Write one log line through the host.
    pub fn log(&mut self, line: &str) {
        (self.log)(line);
    }

    /// Whether the host has cancelled this run.
    pub fn is_cancelled(&self) -> bool {
        (self.cancelled)()
    }
}

impl AssetContext<'_> {
    /// The most bytes one [`AssetContext::read_candidate`] call returns.
    pub const MAX_READ: usize = 64 * 1024;
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

    /// The registered Source with this id.
    pub fn source(&self, id: &str) -> Option<&dyn Source> {
        self.sources.iter().find(|s| s.id() == id).map(|s| &**s)
    }

    /// Run the Source registered as `id` over one candidate. The result's
    /// provenance names the Source that ran.
    pub fn execute_source(
        &self,
        id: &str,
        candidate: &mut dyn Candidate,
        log: &mut dyn FnMut(&str),
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Accepted, ExecutionError> {
        let source = self.source(id).ok_or(ExecutionError::UnknownSource)?;
        let mut context = AssetContext {
            candidate,
            log,
            cancelled,
            proposal: None,
            broken: None,
        };
        let outcome = source.run(&mut context);
        if context.is_cancelled() {
            return Err(ExecutionError::Cancelled);
        }
        match outcome {
            Err(SourceError::Refused(reason)) => return Err(ExecutionError::Refused(reason)),
            Err(SourceError::Unsupported) => return Err(ExecutionError::Unsupported),
            Err(SourceError::Failed(reason)) => return Err(ExecutionError::Failed(reason)),
            Ok(()) => {}
        }
        if let Some(reason) = context.broken {
            return Err(ExecutionError::Failed(reason));
        }
        let proposal = context.proposal.ok_or(ExecutionError::NoProposal)?;
        Ok(Accepted {
            proposal,
            provenance: Provenance {
                source_id: source.id().to_owned(),
                source_version: source.version().to_owned(),
            },
        })
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

#[cfg(test)]
mod source_execution {
    use super::*;

    /// Candidate bytes held in memory, as a host would expose a staged file.
    struct Bytes(Vec<u8>);
    impl Candidate for Bytes {
        fn size(&self) -> u64 {
            self.0.len() as u64
        }
        fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
            let start = (offset as usize).min(self.0.len());
            let n = buf.len().min(self.0.len() - start);
            buf[..n].copy_from_slice(&self.0[start..start + n]);
            Ok(n)
        }
    }

    /// A Source that proposes an image when the first byte is `I`.
    struct FirstByte;
    impl Source for FirstByte {
        fn id(&self) -> &str {
            "test.first-byte"
        }
        fn version(&self) -> &str {
            "1.2.3"
        }
        fn run(&self, context: &mut AssetContext<'_>) -> Result<(), SourceError> {
            let mut first = [0; 1];
            context
                .read_candidate(0, &mut first)
                .map_err(|_| SourceError::Failed("read".into()))?;
            if first != *b"I" {
                return Err(SourceError::Refused("not an I".into()));
            }
            context
                .propose_asset(AssetProposal::new("image", "image/x-test"))
                .map_err(|_| SourceError::Failed("proposal".into()))
        }
    }

    fn registry_with(source: impl Source + 'static) -> Registry {
        let mut registry = Registry::new();
        registry.register_source(Box::new(source));
        registry
    }

    fn execute(registry: &Registry, id: &str, bytes: &[u8]) -> Result<Accepted, ExecutionError> {
        registry.execute_source(id, &mut Bytes(bytes.to_vec()), &mut |_| {}, &|| false)
    }

    #[test]
    fn source_execution_returns_the_proposal_with_host_provenance() {
        let registry = registry_with(FirstByte);
        let accepted = execute(&registry, "test.first-byte", b"Image").unwrap();
        assert_eq!(
            accepted.proposal,
            AssetProposal::new("image", "image/x-test")
        );
        assert_eq!(
            accepted.provenance,
            Provenance {
                source_id: "test.first-byte".into(),
                source_version: "1.2.3".into()
            }
        );
    }

    /// A Source whose run is a closure, for one-off behaviors.
    struct Scripted(fn(&mut AssetContext<'_>) -> Result<(), SourceError>);
    impl Source for Scripted {
        fn id(&self) -> &str {
            "test.scripted"
        }
        fn version(&self) -> &str {
            "0.0.1"
        }
        fn run(&self, context: &mut AssetContext<'_>) -> Result<(), SourceError> {
            (self.0)(context)
        }
    }

    fn run_scripted(
        script: fn(&mut AssetContext<'_>) -> Result<(), SourceError>,
        bytes: &[u8],
    ) -> Result<Accepted, ExecutionError> {
        execute(&registry_with(Scripted(script)), "test.scripted", bytes)
    }

    #[test]
    fn source_execution_reports_a_refusal() {
        let registry = registry_with(FirstByte);
        assert_eq!(
            execute(&registry, "test.first-byte", b"text"),
            Err(ExecutionError::Refused("not an I".into()))
        );
    }

    #[test]
    fn source_execution_fails_a_run_that_proposes_twice() {
        let outcome = run_scripted(
            |context| {
                context
                    .propose_asset(AssetProposal::new("image", "image/png"))
                    .unwrap();
                assert_eq!(
                    context.propose_asset(AssetProposal::new("image", "image/jpeg")),
                    Err(ContextError::AlreadyProposed)
                );
                Ok(())
            },
            b"I",
        );
        assert!(
            matches!(outcome, Err(ExecutionError::Failed(_))),
            "{outcome:?}"
        );
    }

    #[test]
    fn source_execution_fails_a_run_without_a_proposal() {
        assert_eq!(
            run_scripted(|_| Ok(()), b"I"),
            Err(ExecutionError::NoProposal)
        );
    }

    #[test]
    fn source_execution_refuses_a_malformed_proposal() {
        let outcome = run_scripted(
            |context| {
                assert_eq!(
                    context.propose_asset(AssetProposal::new("", "image/png")),
                    Err(ContextError::InvalidProposal)
                );
                assert_eq!(
                    context.propose_asset(AssetProposal::new("image", "png; charset=x")),
                    Err(ContextError::InvalidProposal)
                );
                Ok(())
            },
            b"I",
        );
        assert_eq!(outcome, Err(ExecutionError::NoProposal));
    }

    #[test]
    fn source_execution_bounds_one_read_to_max_read() {
        run_scripted(
            |context| {
                let mut buf = vec![0; AssetContext::MAX_READ * 2];
                assert_eq!(
                    context.read_candidate(0, &mut buf),
                    Ok(AssetContext::MAX_READ)
                );
                context
                    .propose_asset(AssetProposal::new("image", "image/png"))
                    .map_err(|_| SourceError::Failed("p".into()))
            },
            &vec![7; AssetContext::MAX_READ * 3],
        )
        .unwrap();
    }

    #[test]
    fn source_execution_reads_nothing_past_the_candidate() {
        run_scripted(
            |context| {
                let mut buf = [0; 8];
                assert_eq!(context.candidate_size(), 3);
                assert_eq!(context.read_candidate(1, &mut buf), Ok(2));
                assert_eq!(&buf[..2], b"BC");
                assert_eq!(context.read_candidate(3, &mut buf), Ok(0));
                assert_eq!(context.read_candidate(u64::MAX, &mut buf), Ok(0));
                context
                    .propose_asset(AssetProposal::new("image", "image/png"))
                    .map_err(|_| SourceError::Failed("p".into()))
            },
            b"ABC",
        )
        .unwrap();
    }

    #[test]
    fn source_execution_refuses_an_unknown_source() {
        assert_eq!(
            execute(&registry_with(FirstByte), "test.missing", b"I"),
            Err(ExecutionError::UnknownSource)
        );
    }

    #[test]
    fn source_execution_reports_a_cancelled_run() {
        let registry = registry_with(Scripted(|context| {
            assert!(context.is_cancelled());
            context
                .propose_asset(AssetProposal::new("image", "image/png"))
                .map_err(|_| SourceError::Failed("p".into()))
        }));
        let outcome = registry.execute_source(
            "test.scripted",
            &mut Bytes(b"I".to_vec()),
            &mut |_| {},
            &|| true,
        );
        assert_eq!(outcome, Err(ExecutionError::Cancelled));
    }

    #[test]
    fn source_execution_writes_log_lines_through_the_host() {
        let registry = registry_with(Scripted(|context| {
            context.log("looked at the candidate");
            context
                .propose_asset(AssetProposal::new("image", "image/png"))
                .map_err(|_| SourceError::Failed("p".into()))
        }));
        let mut lines = Vec::new();
        registry
            .execute_source(
                "test.scripted",
                &mut Bytes(b"I".to_vec()),
                &mut |line| lines.push(line.to_owned()),
                &|| false,
            )
            .unwrap();
        assert_eq!(lines, ["looked at the candidate"]);
    }

    #[test]
    fn source_execution_reports_an_unsupported_source() {
        assert_eq!(
            run_scripted(|_| Err(SourceError::Unsupported), b"I"),
            Err(ExecutionError::Unsupported)
        );
    }
}
