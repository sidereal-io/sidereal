//! Public plugin contracts for Sidereal.
//!
//! This crate holds the interface a domain pack codes against — the same
//! surface a third-party pack would use. The first-party astro pack depends on
//! this crate and never on `core` internals (ADR-002).
//!
//! The contracts here are intentionally minimal and not a frozen ABI. They
//! are expected to churn until M2 freezes them.
//!
//! A [`Source`] and an [`Operator`] run through an [`AssetContext`]: the
//! context is the only way they reach their input's bytes. A Source can
//! propose one Asset; an Operator can propose facet values and one preview,
//! whose bytes it writes through the context to the host's [`Output`]. The
//! host keeps identity, hashing, provenance and lineage; a plugin never sets
//! them.

use std::fmt;

/// Version of the plugin ABI surface. Bumped when the contracts change.
///
/// - `0`: the M0 skeleton, with id-only stubs.
/// - `1`: a [`Source`] reports its version and runs through an
///   [`AssetContext`]. Every Source must implement [`Source::run`].
/// - `2`: an [`Operator`] reports its version and the facets it writes, and
///   runs through an [`AssetContext`]; a pack declares its facet schemas, and
///   hosts register packs through [`Registry::register_pack`].
/// - `3`: an [`Operator`] declares the media types it accepts and reads its
///   version's media type through its [`AssetContext`], and it can propose a
///   preview and stream its bytes to the host's [`Output`].
pub const ABI_VERSION: u32 = 3;

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
        token(&self.kind) && is_media_type(&self.media_type)
    }
}

/// One lowercase token: letters, digits, `+`, `-` and `.`.
fn token(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"+-.".contains(&b))
}

/// Whether `text` is one lowercase `type/subtype` with no parameters.
fn is_media_type(text: &str) -> bool {
    text.split_once('/')
        .is_some_and(|(kind, subtype)| token(kind) && token(subtype))
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
    /// No Operator with that id is registered.
    UnknownOperator,
    /// The Source refused the candidate.
    Refused(String),
    /// The Source cannot run yet.
    Unsupported,
    /// The Source failed, or broke the context's rules.
    Failed(String),
    /// The run ended without a proposal.
    NoProposal,
    /// The Operator stopped because the host refused a preview write.
    Output,
    /// The host cancelled the run.
    Cancelled,
    /// The Operator does not accept the version's media type, so it did not
    /// run.
    NotAccepted,
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSource => f.write_str("no such source is registered"),
            Self::UnknownOperator => f.write_str("no such operator is registered"),
            Self::Refused(reason) => write!(f, "the source refused the candidate: {reason}"),
            Self::Unsupported => f.write_str("the source cannot run yet"),
            Self::Failed(reason) => write!(f, "the source failed: {reason}"),
            Self::NoProposal => f.write_str("the source proposed no asset"),
            Self::Output => f.write_str("the preview could not be written"),
            Self::Cancelled => f.write_str("the run was cancelled"),
            Self::NotAccepted => f.write_str("the operator does not accept this media type"),
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
    /// This kind of run may not make this proposal or write: a Source run
    /// proposes no facet and no preview, an Operator run proposes no Asset,
    /// and preview bytes need a proposed preview first.
    NotAllowed,
    /// The host refused a preview write.
    Write,
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Read => "the candidate could not be read",
            Self::AlreadyProposed => "the run already made this proposal",
            Self::InvalidProposal => "the proposal is not well formed",
            Self::NotAllowed => "the context does not allow this",
            Self::Write => "the preview could not be written",
        })
    }
}

impl std::error::Error for ContextError {}

/// Where the host keeps the bytes of a preview an Operator writes. The host
/// supplies it; the Operator reaches it only through
/// [`AssetContext::write_preview`].
pub trait Output {
    /// Keep `bytes`, after the bytes of every earlier write.
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<()>;
}

/// Which kind of plugin a context serves.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RunKind {
    Source,
    Operator,
}

/// The run-scoped context a [`Source`] or [`Operator`] works through. It
/// exposes no path, database handle or identity setter.
///
/// The candidate is the run's input: for a Source, the file it looks at; for
/// an Operator, the stored bytes of the Asset version it reads.
pub struct AssetContext<'a> {
    kind: RunKind,
    candidate: &'a mut dyn Candidate,
    media_type: Option<&'a str>,
    /// Where preview bytes go: only an Operator run has one.
    output: Option<&'a mut dyn Output>,
    log: &'a mut dyn FnMut(&str),
    cancelled: &'a dyn Fn() -> bool,
    proposal: Option<AssetProposal>,
    facets: Vec<FacetProposal>,
    preview: Option<AssetProposal>,
    preview_bytes: u64,
    broken: Option<String>,
}

impl<'a> AssetContext<'a> {
    fn new(
        kind: RunKind,
        candidate: &'a mut dyn Candidate,
        media_type: Option<&'a str>,
        output: Option<&'a mut dyn Output>,
        log: &'a mut dyn FnMut(&str),
        cancelled: &'a dyn Fn() -> bool,
    ) -> Self {
        Self {
            kind,
            candidate,
            media_type,
            output,
            log,
            cancelled,
            proposal: None,
            facets: Vec::new(),
            preview: None,
            preview_bytes: 0,
            broken: None,
        }
    }
}

impl AssetContext<'_> {
    /// The candidate's size in bytes.
    pub fn candidate_size(&self) -> u64 {
        self.candidate.size()
    }

    /// The media type the host recorded for the version an Operator reads.
    /// A Source run has none: recognizing the candidate is the Source's job.
    pub fn media_type(&self) -> Option<&str> {
        self.media_type
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
        if self.kind != RunKind::Source {
            self.broken = Some("an operator run proposed an asset".into());
            return Err(ContextError::NotAllowed);
        }
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

    /// Propose a value for the facet `name`. The host checks every proposal
    /// against the facet's schema and this Operator's grants before it
    /// stores any.
    pub fn propose_facet(
        &mut self,
        name: impl Into<String>,
        value: FacetValue,
    ) -> Result<(), ContextError> {
        if self.kind != RunKind::Operator {
            self.broken = Some("a source run proposed a facet".into());
            return Err(ContextError::NotAllowed);
        }
        self.facets.push(FacetProposal {
            name: name.into(),
            value,
        });
        Ok(())
    }

    /// Propose the one preview of the version this Operator run reads: a
    /// browser-ready image the host keeps as a new, derived Asset. The host
    /// mints its ids, hashes its bytes and links it to the version.
    pub fn propose_preview(&mut self, proposal: AssetProposal) -> Result<(), ContextError> {
        if self.kind != RunKind::Operator {
            self.broken = Some("a source run proposed a preview".into());
            return Err(ContextError::NotAllowed);
        }
        if self.preview.is_some() {
            self.broken = Some("proposed more than one preview".into());
            return Err(ContextError::AlreadyProposed);
        }
        if !proposal.is_well_formed() {
            return Err(ContextError::InvalidProposal);
        }
        self.preview = Some(proposal);
        Ok(())
    }

    /// Write the next bytes of the proposed preview to the host.
    pub fn write_preview(&mut self, bytes: &[u8]) -> Result<(), ContextError> {
        let Some(output) = self.output.as_deref_mut() else {
            self.broken = Some("a source run wrote preview bytes".into());
            return Err(ContextError::NotAllowed);
        };
        if self.preview.is_none() {
            self.broken
                .get_or_insert_with(|| "wrote preview bytes before proposing a preview".into());
            return Err(ContextError::NotAllowed);
        }
        output.write(bytes).map_err(|_| ContextError::Write)?;
        self.preview_bytes += bytes.len() as u64;
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

/// Takes an Asset version and proposes facts about it (e.g. plate solving).
///
/// The Registry asks for [`Operator::id`], [`Operator::version`],
/// [`Operator::writes`] and [`Operator::accepts`] once, when the Operator
/// registers, and keeps the answers: later answers change nothing.
pub trait Operator: Send + Sync {
    /// Stable, namespaced identifier, e.g. `"astro.plate-solve"`.
    fn id(&self) -> &str;

    /// The implementation's version, recorded with each value it produces.
    fn version(&self) -> &str;

    /// The facets this Operator writes: its grants. Core refuses a proposal
    /// for any other facet.
    fn writes(&self) -> &[&str];

    /// The media types of originals' versions this Operator reads, such as
    /// `image/png`: each one lowercase `type/subtype` with no parameters.
    /// The host runs it only over a version with one of these media types.
    fn accepts(&self) -> &[&str];

    /// Whether this Operator makes previews through
    /// [`AssetContext::propose_preview`]. The host refuses a preview from an
    /// Operator that did not say so when it registered.
    fn makes_previews(&self) -> bool {
        false
    }

    /// Read one Asset version through `context` and propose facet values.
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError>;
}

/// How an [`Operator`] run ends when it does not finish.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperatorError {
    /// This Operator cannot run yet.
    Unsupported,
    /// The Operator could not finish.
    Failed(String),
    /// The Operator stopped because the host refused a preview write.
    Output,
}

/// `?` on a context call reports a refused preview write as
/// [`OperatorError::Output`], and any other refusal as a failure with its
/// text.
impl From<ContextError> for OperatorError {
    fn from(error: ContextError) -> Self {
        match error {
            ContextError::Write => Self::Output,
            other => Self::Failed(other.to_string()),
        }
    }
}

/// A facet value an Operator proposes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FacetValue {
    /// A whole number. A [`FacetType::Count`] accepts 0 to [`MAX_COUNT`];
    /// the host refuses any other.
    Integer(i64),
}

/// The largest count: the largest whole number a JavaScript client reads
/// exactly (`Number.MAX_SAFE_INTEGER`).
pub const MAX_COUNT: i64 = 9_007_199_254_740_991;

/// One proposed value for one facet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FacetProposal {
    pub name: String,
    pub value: FacetValue,
}

/// An Operator run that finished, with what it proposed. The producer, the
/// grants and `makes_previews` come from the Operator's registration, never
/// from the run. Nothing here is checked yet: the host checks it before it
/// stores it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperatorProposals {
    pub producer_id: String,
    pub producer_version: String,
    pub grants: Vec<String>,
    pub proposals: Vec<FacetProposal>,
    /// Whether the Operator registered as one that makes previews.
    pub makes_previews: bool,
    /// The preview the run proposed, if it proposed one.
    pub preview: Option<AssetProposal>,
    /// How many preview bytes the context passed on to the host's
    /// [`Output`].
    pub preview_bytes: u64,
}

/// An Operator as the Registry recorded it when it registered. Core takes
/// the producer and the grants from here, never from the Operator again.
pub struct RegisteredOperator {
    id: String,
    version: String,
    writes: Vec<String>,
    accepts: Vec<String>,
    makes_previews: bool,
    operator: Box<dyn Operator>,
}

impl RegisteredOperator {
    /// The id the Operator registered with.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The version the Operator registered with.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The facets the Operator listed when it registered: its grants.
    pub fn writes(&self) -> &[String] {
        &self.writes
    }

    /// The media types the Operator listed when it registered.
    pub fn accepts(&self) -> &[String] {
        &self.accepts
    }

    /// Whether the Operator registered as one that makes previews.
    pub fn makes_previews(&self) -> bool {
        self.makes_previews
    }

    /// Whether the Operator registered to accept exactly `media_type`.
    pub fn accepts_media_type(&self, media_type: &str) -> bool {
        self.accepts.iter().any(|accepted| accepted == media_type)
    }
}

/// The type of a facet's value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FacetType {
    /// A whole number from 0 to [`MAX_COUNT`].
    Count,
}

/// What a facet's value describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FacetScope {
    /// The Asset as a whole, across its versions.
    Asset,
    /// One Asset version: one exact set of bytes.
    AssetVersion,
}

/// The declaration of one facet, which the pack that declares it owns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FacetSchema {
    /// The facet's name: the owning pack's id, a dot, then the rest, such as
    /// `"image.file-size"`.
    pub name: String,
    /// What a person reads beside the value, such as `"File size"`.
    pub label: String,
    pub value_type: FacetType,
    /// The value's unit, such as `"bytes"`. It may be empty.
    pub unit: String,
    /// Raised whenever the label, unit, type or scope changes.
    pub version: u32,
    pub scope: FacetScope,
}

/// A facet schema the Registry keeps, with the pack that declared it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredFacet {
    pub schema: FacetSchema,
    /// The id of the pack that declared the schema.
    pub owner: String,
}

/// Why the Registry refused a pack. Each error names what it refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    /// Another declaration already uses this facet name.
    FacetAlreadyDeclared(String),
    /// The facet's name does not start with its pack's id and a dot.
    FacetOutsideNamespace { facet: String, pack: String },
    /// Another registered Operator already has this id.
    OperatorAlreadyRegistered(String),
    /// The facet schema holds something core could not store.
    InvalidFacet { facet: String, reason: &'static str },
    /// The Operator's id or version is something core could not record.
    InvalidOperator {
        operator: String,
        reason: &'static str,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FacetAlreadyDeclared(facet) => {
                write!(f, "facet {} is already declared", facet.escape_debug())
            }
            Self::FacetOutsideNamespace { facet, pack } => write!(
                f,
                "facet {} is not in the namespace of pack {}",
                facet.escape_debug(),
                pack.escape_debug()
            ),
            Self::OperatorAlreadyRegistered(id) => {
                write!(f, "operator {} is already registered", id.escape_debug())
            }
            Self::InvalidFacet { facet, reason } => {
                write!(
                    f,
                    "facet {} cannot be stored: {reason}",
                    facet.escape_debug()
                )
            }
            Self::InvalidOperator { operator, reason } => write!(
                f,
                "operator {} cannot be recorded: {reason}",
                operator.escape_debug()
            ),
        }
    }
}

impl std::error::Error for RegistryError {}

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
    operators: Vec<RegisteredOperator>,
    sinks: Vec<Box<dyn Sink>>,
    facets: Vec<DeclaredFacet>,
    /// The pack being registered, on the staging Registry that
    /// [`Registry::register_pack`] hands it.
    pack: Option<String>,
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

    /// Register an [`Operator`]. Its id, version, grants and accepted media
    /// types are recorded now, once.
    pub fn register_operator(&mut self, operator: Box<dyn Operator>) {
        self.operators.push(RegisteredOperator {
            id: operator.id().to_owned(),
            version: operator.version().to_owned(),
            writes: operator.writes().iter().map(|&w| w.to_owned()).collect(),
            accepts: operator.accepts().iter().map(|&m| m.to_owned()).collect(),
            makes_previews: operator.makes_previews(),
            operator,
        });
    }

    /// The registered Operator with this id.
    pub fn operator(&self, id: &str) -> Option<&RegisteredOperator> {
        self.operators.iter().find(|o| o.id == id)
    }

    /// Register a [`Sink`].
    pub fn register_sink(&mut self, sink: Box<dyn Sink>) {
        self.sinks.push(sink);
    }

    /// Declare a facet schema that the pack being registered owns.
    pub fn declare_facet(&mut self, schema: FacetSchema) {
        let owner = self.pack.clone().unwrap_or_default();
        self.facets.push(DeclaredFacet { schema, owner });
    }

    /// The kept facet schema with this name.
    pub fn facet_schema(&self, name: &str) -> Option<&DeclaredFacet> {
        self.facets.iter().find(|f| f.schema.name == name)
    }

    /// Register one pack. The pack registers into a staging Registry; only
    /// when everything it declared and registered passes the checks does it
    /// join this Registry, so a refused pack adds nothing.
    pub fn register_pack(&mut self, pack: &dyn Pack) -> Result<(), RegistryError> {
        // Read the id once, so the checks use the owner the schemas record.
        let pack_id = pack.id().to_owned();
        let mut staged = Registry {
            pack: Some(pack_id.clone()),
            ..Registry::default()
        };
        pack.register(&mut staged);
        // A schema declared after the pack replaced its staging Registry has
        // no owner yet; it is the pack's. A nested pack's schemas keep theirs.
        for declared in &mut staged.facets {
            if declared.owner.is_empty() {
                declared.owner.clone_from(&pack_id);
            }
        }
        let pack_id = pack_id.as_str();
        for (index, declared) in staged.facets.iter().enumerate() {
            let name = &declared.schema.name;
            if let Some(reason) = unstorable_facet(&declared.schema) {
                return Err(RegistryError::InvalidFacet {
                    facet: name.clone(),
                    reason,
                });
            }
            if !name
                .strip_prefix(pack_id)
                .is_some_and(|rest| rest.starts_with('.'))
            {
                return Err(RegistryError::FacetOutsideNamespace {
                    facet: name.clone(),
                    pack: pack_id.to_owned(),
                });
            }
            if self.facet_schema(name).is_some()
                || staged.facets[..index]
                    .iter()
                    .any(|f| f.schema.name == *name)
            {
                return Err(RegistryError::FacetAlreadyDeclared(name.clone()));
            }
        }
        for (index, entry) in staged.operators.iter().enumerate() {
            if let Some(reason) = unrecordable_operator(entry) {
                return Err(RegistryError::InvalidOperator {
                    operator: entry.id.clone(),
                    reason,
                });
            }
            if self.operator(&entry.id).is_some()
                || staged.operators[..index].iter().any(|o| o.id == entry.id)
            {
                return Err(RegistryError::OperatorAlreadyRegistered(entry.id.clone()));
            }
        }
        self.sources.append(&mut staged.sources);
        self.operators.append(&mut staged.operators);
        self.sinks.append(&mut staged.sinks);
        self.facets.append(&mut staged.facets);
        Ok(())
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
        let mut context = AssetContext::new(RunKind::Source, candidate, None, None, log, cancelled);
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

    /// Run the Operator registered as `id` over one Asset version's bytes,
    /// whose recorded media type is `media_type`. An Operator that did not
    /// register to accept that media type does not run. Preview bytes the
    /// Operator writes go to `output`. The result names the producer and the
    /// grants from the Operator's registration. It does not check the
    /// proposals; the host does.
    pub fn execute_operator(
        &self,
        id: &str,
        candidate: &mut dyn Candidate,
        media_type: &str,
        output: &mut dyn Output,
        log: &mut dyn FnMut(&str),
        cancelled: &dyn Fn() -> bool,
    ) -> Result<OperatorProposals, ExecutionError> {
        let entry = self.operator(id).ok_or(ExecutionError::UnknownOperator)?;
        if !entry.accepts_media_type(media_type) {
            return Err(ExecutionError::NotAccepted);
        }
        let mut context = AssetContext::new(
            RunKind::Operator,
            candidate,
            Some(media_type),
            Some(output),
            log,
            cancelled,
        );
        let outcome = entry.operator.run(&mut context);
        if context.is_cancelled() {
            return Err(ExecutionError::Cancelled);
        }
        match outcome {
            Err(OperatorError::Unsupported) => return Err(ExecutionError::Unsupported),
            Err(OperatorError::Failed(reason)) => return Err(ExecutionError::Failed(reason)),
            Err(OperatorError::Output) => return Err(ExecutionError::Output),
            Ok(()) => {}
        }
        if let Some(reason) = context.broken {
            return Err(ExecutionError::Failed(reason));
        }
        Ok(OperatorProposals {
            producer_id: entry.id.clone(),
            producer_version: entry.version.clone(),
            grants: entry.writes.clone(),
            proposals: context.facets,
            makes_previews: entry.makes_previews,
            preview: context.preview,
            preview_bytes: context.preview_bytes,
        })
    }

    /// Ids of all registered sources, in registration order.
    pub fn source_ids(&self) -> Vec<&str> {
        self.sources.iter().map(|s| s.id()).collect()
    }

    /// Ids of all registered operators, in registration order.
    pub fn operator_ids(&self) -> Vec<&str> {
        self.operators.iter().map(|o| o.id.as_str()).collect()
    }

    /// Ids of all registered sinks, in registration order.
    pub fn sink_ids(&self) -> Vec<&str> {
        self.sinks.iter().map(|s| s.id()).collect()
    }
}

/// Why core could not store this schema, if it could not. These checks match
/// the columns core stores a schema in, so a run never fails on a healthy
/// database because of what its pack declared.
fn unstorable_facet(schema: &FacetSchema) -> Option<&'static str> {
    let part = |p: &str| {
        !p.is_empty()
            && p.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    };
    let mut parts = schema.name.split('.');
    let well_formed = schema.name.contains('.') && parts.all(part);
    if !well_formed {
        Some("its name must be two or more parts of lowercase letters, digits and hyphens, joined by dots")
    } else if !(1..=i32::MAX as u32).contains(&schema.version) {
        Some("its version must be from 1 to 2147483647")
    } else if schema.label.is_empty() {
        Some("its label is empty")
    } else if schema.label.contains('\0') {
        Some("its label holds a zero character")
    } else if schema.unit.contains('\0') {
        Some("its unit holds a zero character")
    } else {
        None
    }
}

/// Why core could not record this Operator's identity, if it could not.
fn unrecordable_operator(entry: &RegisteredOperator) -> Option<&'static str> {
    if entry.id.is_empty() {
        Some("it has an empty id")
    } else if entry.id.contains('\0') {
        Some("its id holds a zero character")
    } else if entry.version.is_empty() {
        Some("its version is empty")
    } else if entry.version.contains('\0') {
        Some("its version holds a zero character")
    } else if entry.accepts.is_empty() {
        Some("it accepts no media type")
    } else if !entry.accepts.iter().all(|m| is_media_type(m)) {
        Some("it accepts a media type that is not well formed")
    } else {
        None
    }
}

/// Implemented by a domain pack to contribute its components. The host builds a
/// [`Registry`] and registers each pack with [`Registry::register_pack`]; this
/// is the entire contact surface between a pack and the host.
pub trait Pack {
    /// Stable pack identifier, e.g. `"astro"`. It is the namespace of the
    /// pack's facets.
    fn id(&self) -> &str;

    /// Register this pack's sources, operators and sinks, and declare its
    /// facet schemas, into `registry`.
    ///
    /// Hosts call [`Registry::register_pack`], never this method directly:
    /// `register_pack` hands the pack a staging Registry and checks what it
    /// registered before any of it joins the live one.
    fn register(&self, registry: &mut Registry);
}

/// A host destination that drops every preview byte, for tests.
#[cfg(test)]
struct Discard;
#[cfg(test)]
impl Output for Discard {
    fn write(&mut self, _: &[u8]) -> std::io::Result<()> {
        Ok(())
    }
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

#[cfg(test)]
mod facet_declaration {
    use super::*;

    /// A pack whose registration is a closure, for one-off declarations.
    struct TestPack(&'static str, fn(&mut Registry));
    impl Pack for TestPack {
        fn id(&self) -> &str {
            self.0
        }
        fn register(&self, registry: &mut Registry) {
            (self.1)(registry)
        }
    }

    fn size(name: &str) -> FacetSchema {
        FacetSchema {
            name: name.into(),
            label: "Size".into(),
            value_type: FacetType::Count,
            unit: "bytes".into(),
            version: 1,
            scope: FacetScope::AssetVersion,
        }
    }

    #[test]
    fn facet_declaration_keeps_every_schema_a_pack_declares() {
        let mut registry = Registry::new();
        registry
            .register_pack(&TestPack("test", |r| {
                r.declare_facet(size("test.size"));
                r.declare_facet(FacetSchema {
                    scope: FacetScope::Asset,
                    ..size("test.width")
                });
            }))
            .unwrap();
        let declared = registry.facet_schema("test.size").unwrap();
        assert_eq!(declared.schema, size("test.size"));
        assert_eq!(declared.owner, "test");
        assert_eq!(
            registry.facet_schema("test.width").unwrap().schema.scope,
            FacetScope::Asset
        );
        assert!(registry.facet_schema("test.missing").is_none());
    }

    #[test]
    fn facet_declaration_refuses_a_name_another_pack_declared() {
        let mut registry = Registry::new();
        registry
            .register_pack(&TestPack("test", |r| r.declare_facet(size("test.size"))))
            .unwrap();
        let error = registry
            .register_pack(&TestPack("test", |r| r.declare_facet(size("test.size"))))
            .unwrap_err();
        assert!(error.to_string().contains("test.size"), "{error}");
    }

    #[test]
    fn facet_declaration_refuses_a_name_declared_twice_by_one_pack() {
        let error = Registry::new()
            .register_pack(&TestPack("test", |r| {
                r.declare_facet(size("test.size"));
                r.declare_facet(size("test.size"));
            }))
            .unwrap_err();
        assert!(error.to_string().contains("test.size"), "{error}");
    }

    #[test]
    fn facet_declaration_refuses_a_name_outside_the_pack_namespace() {
        let mut registry = Registry::new();
        let error = registry
            .register_pack(&TestPack("test", |r| r.declare_facet(size("other.size"))))
            .unwrap_err();
        assert!(error.to_string().contains("other.size"), "{error}");
        // A longer pack id that starts with the same letters is another pack.
        let error = registry
            .register_pack(&TestPack("test", |r| r.declare_facet(size("testing.size"))))
            .unwrap_err();
        assert!(error.to_string().contains("testing.size"), "{error}");
        assert!(registry.facet_schema("other.size").is_none());
        assert!(registry.facet_schema("testing.size").is_none());
    }

    /// A pack whose id is empty until it registers, then `test`.
    struct Renamed(std::sync::atomic::AtomicBool);
    impl Pack for Renamed {
        fn id(&self) -> &str {
            if self.0.load(std::sync::atomic::Ordering::SeqCst) {
                "test"
            } else {
                ""
            }
        }
        fn register(&self, registry: &mut Registry) {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
            registry.declare_facet(size("test.size"));
        }
    }

    /// A pack that swaps its staging Registry for a new one, then declares.
    struct Swapping;
    impl Pack for Swapping {
        fn id(&self) -> &str {
            "test"
        }
        fn register(&self, registry: &mut Registry) {
            *registry = Registry::new();
            registry.declare_facet(size("test.size"));
        }
    }

    /// A pack `test` that registers the pack `test.child` while it registers.
    struct Parent;
    impl Pack for Parent {
        fn id(&self) -> &str {
            "test"
        }
        fn register(&self, registry: &mut Registry) {
            registry
                .register_pack(&TestPack("test.child", |r| {
                    r.declare_facet(size("test.child.size"))
                }))
                .unwrap();
        }
    }

    #[test]
    fn facet_declaration_keeps_a_nested_packs_owner() {
        let mut registry = Registry::new();
        registry.register_pack(&Parent).unwrap();
        assert_eq!(
            registry.facet_schema("test.child.size").unwrap().owner,
            "test.child"
        );
    }

    #[test]
    fn facet_declaration_records_the_registering_pack_as_owner() {
        let mut registry = Registry::new();
        registry.register_pack(&Swapping).unwrap();
        assert_eq!(registry.facet_schema("test.size").unwrap().owner, "test");
    }

    #[test]
    fn facet_declaration_checks_the_owner_it_records() {
        let mut registry = Registry::new();
        let error = registry
            .register_pack(&Renamed(Default::default()))
            .unwrap_err();
        assert!(error.to_string().contains("test.size"), "{error}");
        assert!(registry.facet_schema("test.size").is_none());
    }
}

#[cfg(test)]
mod operator_registration {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct TestPack(&'static str, Box<dyn Fn(&mut Registry)>);
    impl Pack for TestPack {
        fn id(&self) -> &str {
            self.0
        }
        fn register(&self, registry: &mut Registry) {
            (self.1)(registry)
        }
    }

    /// An Operator whose `writes()` answer widens once `widened` is set.
    struct Shifty {
        id: &'static str,
        widened: Arc<AtomicBool>,
    }
    impl Operator for Shifty {
        fn id(&self) -> &str {
            self.id
        }
        fn version(&self) -> &str {
            if self.widened.load(Ordering::SeqCst) {
                "9.9.9"
            } else {
                "1.2.3"
            }
        }
        fn writes(&self) -> &[&str] {
            if self.widened.load(Ordering::SeqCst) {
                &["test.size", "test.width"]
            } else {
                &["test.size"]
            }
        }
        fn accepts(&self) -> &[&str] {
            &["image/x-test"]
        }
        fn run(&self, _: &mut AssetContext<'_>) -> Result<(), OperatorError> {
            Ok(())
        }
    }

    fn shifty(id: &'static str, widened: &Arc<AtomicBool>) -> Box<dyn Operator> {
        Box::new(Shifty {
            id,
            widened: widened.clone(),
        })
    }

    #[test]
    fn operator_registration_records_id_version_and_grants() {
        let flag = Arc::new(AtomicBool::new(false));
        let mut registry = Registry::new();
        registry
            .register_pack(&TestPack(
                "test",
                Box::new(move |r| r.register_operator(shifty("test.op", &flag))),
            ))
            .unwrap();
        let entry = registry.operator("test.op").unwrap();
        assert_eq!(entry.id(), "test.op");
        assert_eq!(entry.version(), "1.2.3");
        assert_eq!(entry.writes(), ["test.size"]);
        assert_eq!(registry.operator_ids(), ["test.op"]);
        assert!(registry.operator("test.missing").is_none());
    }

    #[test]
    fn operator_registration_keeps_the_grants_it_registered_with() {
        let widened = Arc::new(AtomicBool::new(false));
        let flag = widened.clone();
        let mut registry = Registry::new();
        registry
            .register_pack(&TestPack(
                "test",
                Box::new(move |r| r.register_operator(shifty("test.op", &flag))),
            ))
            .unwrap();
        widened.store(true, Ordering::SeqCst);
        let entry = registry.operator("test.op").unwrap();
        assert_eq!(entry.writes(), ["test.size"]);
        assert_eq!(entry.version(), "1.2.3");
        let run = registry
            .execute_operator(
                "test.op",
                &mut NoBytes,
                "image/x-test",
                &mut Discard,
                &mut |_| {},
                &|| false,
            )
            .unwrap();
        assert_eq!(run.grants, ["test.size"]);
        assert_eq!(run.producer_version, "1.2.3");
    }

    struct NoBytes;
    impl Candidate for NoBytes {
        fn size(&self) -> u64 {
            0
        }
        fn read_at(&mut self, _: u64, _: &mut [u8]) -> std::io::Result<usize> {
            Ok(0)
        }
    }

    #[test]
    fn operator_registration_refuses_an_id_another_operator_has() {
        let flag = Arc::new(AtomicBool::new(false));
        let mut registry = Registry::new();
        let first = flag.clone();
        registry
            .register_pack(&TestPack(
                "test",
                Box::new(move |r| r.register_operator(shifty("test.op", &first))),
            ))
            .unwrap();
        let second = flag.clone();
        let error = registry
            .register_pack(&TestPack(
                "other",
                Box::new(move |r| r.register_operator(shifty("test.op", &second))),
            ))
            .unwrap_err();
        assert_eq!(
            error,
            RegistryError::OperatorAlreadyRegistered("test.op".into())
        );
        assert!(error.to_string().contains("test.op"), "{error}");
        assert_eq!(registry.operator_ids(), ["test.op"]);
    }

    #[test]
    fn operator_registration_refuses_one_id_twice_in_one_pack() {
        let flag = Arc::new(AtomicBool::new(false));
        let error = Registry::new()
            .register_pack(&TestPack(
                "test",
                Box::new(move |r| {
                    r.register_operator(shifty("test.op", &flag));
                    r.register_operator(shifty("test.op", &flag));
                }),
            ))
            .unwrap_err();
        assert!(error.to_string().contains("test.op"), "{error}");
    }

    #[test]
    fn operator_registration_is_abi_version_3() {
        assert_eq!(ABI_VERSION, 3);
    }
}

#[cfg(test)]
mod operator_execution {
    use super::*;

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

    /// An Operator whose run is a closure, for one-off behaviors.
    struct Scripted(fn(&mut AssetContext<'_>) -> Result<(), OperatorError>);
    impl Operator for Scripted {
        fn id(&self) -> &str {
            "test.op"
        }
        fn version(&self) -> &str {
            "1.2.3"
        }
        fn writes(&self) -> &[&str] {
            &["test.first-byte"]
        }
        fn accepts(&self) -> &[&str] {
            &["image/x-test"]
        }
        fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
            (self.0)(context)
        }
    }

    struct TestPack(fn(&mut AssetContext<'_>) -> Result<(), OperatorError>);
    impl Pack for TestPack {
        fn id(&self) -> &str {
            "test"
        }
        fn register(&self, registry: &mut Registry) {
            registry.register_operator(Box::new(Scripted(self.0)));
        }
    }

    fn run(
        script: fn(&mut AssetContext<'_>) -> Result<(), OperatorError>,
        bytes: &[u8],
    ) -> Result<OperatorProposals, ExecutionError> {
        let mut registry = Registry::new();
        registry.register_pack(&TestPack(script)).unwrap();
        registry.execute_operator(
            "test.op",
            &mut Bytes(bytes.to_vec()),
            "image/x-test",
            &mut Discard,
            &mut |_| {},
            &|| false,
        )
    }

    fn first_byte(context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        let mut first = [0; 1];
        context
            .read_candidate(0, &mut first)
            .map_err(|_| OperatorError::Failed("read".into()))?;
        context
            .propose_facet("test.first-byte", FacetValue::Integer(first[0].into()))
            .map_err(|_| OperatorError::Failed("proposal".into()))
    }

    #[test]
    fn operator_execution_returns_proposals_with_host_provenance_and_grants() {
        let run = run(first_byte, b"*rest").unwrap();
        assert_eq!(
            run.proposals,
            [FacetProposal {
                name: "test.first-byte".into(),
                value: FacetValue::Integer(42)
            }]
        );
        assert_eq!(run.producer_id, "test.op");
        assert_eq!(run.producer_version, "1.2.3");
        assert_eq!(run.grants, ["test.first-byte"]);
    }

    #[test]
    fn operator_execution_returns_every_proposal_in_order() {
        let run = run(
            |context| {
                context
                    .propose_facet("test.a", FacetValue::Integer(-1))
                    .unwrap();
                context
                    .propose_facet("test.b", FacetValue::Integer(i64::MAX))
                    .unwrap();
                Ok(())
            },
            b"",
        )
        .unwrap();
        let names: Vec<_> = run.proposals.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["test.a", "test.b"]);
    }

    #[test]
    fn operator_execution_writes_log_lines_through_the_host() {
        let mut registry = Registry::new();
        registry
            .register_pack(&TestPack(|context| {
                context.log("looked at the bytes");
                Ok(())
            }))
            .unwrap();
        let mut lines = Vec::new();
        registry
            .execute_operator(
                "test.op",
                &mut Bytes(b"I".to_vec()),
                "image/x-test",
                &mut Discard,
                &mut |line| lines.push(line.to_owned()),
                &|| false,
            )
            .unwrap();
        assert_eq!(lines, ["looked at the bytes"]);
    }

    #[test]
    fn operator_execution_refuses_an_asset_proposal() {
        let outcome = run(
            |context| {
                assert_eq!(
                    context.propose_asset(AssetProposal::new("image", "image/png")),
                    Err(ContextError::NotAllowed)
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
    fn operator_execution_is_refused_to_a_source_that_proposes_a_facet() {
        struct Proposer;
        impl Source for Proposer {
            fn id(&self) -> &str {
                "test.source"
            }
            fn version(&self) -> &str {
                "1"
            }
            fn run(&self, context: &mut AssetContext<'_>) -> Result<(), SourceError> {
                assert_eq!(
                    context.propose_facet("test.size", FacetValue::Integer(1)),
                    Err(ContextError::NotAllowed)
                );
                context
                    .propose_asset(AssetProposal::new("image", "image/png"))
                    .map_err(|_| SourceError::Failed("p".into()))
            }
        }
        let mut registry = Registry::new();
        registry.register_source(Box::new(Proposer));
        let outcome = registry.execute_source(
            "test.source",
            &mut Bytes(b"I".to_vec()),
            &mut |_| {},
            &|| false,
        );
        assert!(
            matches!(outcome, Err(ExecutionError::Failed(_))),
            "{outcome:?}"
        );
    }

    #[test]
    fn operator_execution_reports_failure_unsupported_and_cancel() {
        assert_eq!(
            run(
                |_| Err(OperatorError::Failed("could not read the header".into())),
                b""
            ),
            Err(ExecutionError::Failed("could not read the header".into()))
        );
        assert_eq!(
            run(|_| Err(OperatorError::Unsupported), b""),
            Err(ExecutionError::Unsupported)
        );
        let mut registry = Registry::new();
        registry.register_pack(&TestPack(first_byte)).unwrap();
        assert_eq!(
            registry.execute_operator(
                "test.op",
                &mut Bytes(b"I".to_vec()),
                "image/x-test",
                &mut Discard,
                &mut |_| {},
                &|| true
            ),
            Err(ExecutionError::Cancelled)
        );
    }

    #[test]
    fn operator_execution_refuses_an_unknown_operator() {
        let registry = Registry::new();
        assert_eq!(
            registry.execute_operator(
                "test.op",
                &mut Bytes(vec![]),
                "image/x-test",
                &mut Discard,
                &mut |_| {},
                &|| false
            ),
            Err(ExecutionError::UnknownOperator)
        );
    }

    #[test]
    fn operator_execution_max_count_is_the_largest_safe_javascript_integer() {
        assert_eq!(MAX_COUNT, (1i64 << 53) - 1);
    }
}

#[cfg(test)]
mod pack_registration {
    use super::*;

    struct TestPack(&'static str, Box<dyn Fn(&mut Registry)>);
    impl Pack for TestPack {
        fn id(&self) -> &str {
            self.0
        }
        fn register(&self, registry: &mut Registry) {
            (self.1)(registry)
        }
    }

    fn pack(register: impl Fn(&mut Registry) + 'static) -> TestPack {
        TestPack("test", Box::new(register))
    }

    struct Op {
        id: &'static str,
        version: &'static str,
        writes: &'static [&'static str],
    }
    impl Operator for Op {
        fn id(&self) -> &str {
            self.id
        }
        fn version(&self) -> &str {
            self.version
        }
        fn writes(&self) -> &[&str] {
            self.writes
        }
        fn accepts(&self) -> &[&str] {
            &["image/x-test"]
        }
        fn run(&self, _: &mut AssetContext<'_>) -> Result<(), OperatorError> {
            Ok(())
        }
    }

    fn op(id: &'static str) -> Box<dyn Operator> {
        Box::new(Op {
            id,
            version: "1",
            writes: &[],
        })
    }

    struct Named(&'static str);
    impl Source for Named {
        fn id(&self) -> &str {
            self.0
        }
        fn version(&self) -> &str {
            "1"
        }
        fn run(&self, _: &mut AssetContext<'_>) -> Result<(), SourceError> {
            Err(SourceError::Unsupported)
        }
    }
    impl Sink for Named {
        fn id(&self) -> &str {
            self.0
        }
    }

    fn schema(name: &str) -> FacetSchema {
        FacetSchema {
            name: name.into(),
            label: "Size".into(),
            value_type: FacetType::Count,
            unit: "bytes".into(),
            version: 1,
            scope: FacetScope::AssetVersion,
        }
    }

    struct NoBytes;
    impl Candidate for NoBytes {
        fn size(&self) -> u64 {
            0
        }
        fn read_at(&mut self, _: u64, _: &mut [u8]) -> std::io::Result<usize> {
            Ok(0)
        }
    }

    fn runs(registry: &Registry, id: &str) -> Result<OperatorProposals, ExecutionError> {
        registry.execute_operator(
            id,
            &mut NoBytes,
            "image/x-test",
            &mut Discard,
            &mut |_| {},
            &|| false,
        )
    }

    fn with_earlier_pack() -> Registry {
        let mut registry = Registry::new();
        registry
            .register_pack(&TestPack(
                "earlier",
                Box::new(|r| r.register_operator(op("test.op"))),
            ))
            .unwrap();
        registry
    }

    /// The live Registry holds only what the earlier pack registered.
    fn assert_unchanged(registry: &Registry) {
        assert_eq!(registry.operator_ids(), ["test.op"]);
        assert!(registry.source_ids().is_empty());
        assert!(registry.sink_ids().is_empty());
        assert!(registry.facets.is_empty());
        assert!(runs(registry, "test.op").is_ok());
    }

    #[test]
    fn pack_registration_refused_for_one_operator_adds_nothing() {
        let mut registry = with_earlier_pack();
        let error = registry
            .register_pack(&pack(|r| {
                r.declare_facet(schema("test.good-size"));
                r.register_source(Box::new(Named("test.source")));
                r.register_sink(Box::new(Named("test.sink")));
                r.register_operator(Box::new(Op {
                    id: "test.good",
                    version: "1",
                    writes: &["test.good-size"],
                }));
                r.register_operator(op("test.op"));
            }))
            .unwrap_err();
        assert!(error.to_string().contains("test.op"), "{error}");
        assert!(registry.operator("test.good").is_none());
        assert!(registry.facet_schema("test.good-size").is_none());
        assert_eq!(
            runs(&registry, "test.good"),
            Err(ExecutionError::UnknownOperator)
        );
        assert_unchanged(&registry);
    }

    /// Each pack makes one declaration core could not store; the error
    /// names `named`, and the live Registry is unchanged.
    fn assert_refused(named: &str, register: impl Fn(&mut Registry) + 'static) {
        let mut registry = with_earlier_pack();
        let error = registry.register_pack(&pack(register)).unwrap_err();
        assert!(error.to_string().contains(named), "{named}: {error}");
        assert_unchanged(&registry);
    }

    #[test]
    fn pack_registration_refuses_a_facet_name_the_store_does_not_accept() {
        for name in [
            "test.Size",
            "test.",
            "test..size",
            "test.si ze",
            "test.size_x",
        ] {
            assert_refused(name, move |r| r.declare_facet(schema(name)));
        }
    }

    #[test]
    fn pack_registration_refuses_a_schema_version_the_store_cannot_hold() {
        for version in [0, 2_147_483_648, u32::MAX] {
            assert_refused("test.size", move |r| {
                r.declare_facet(FacetSchema {
                    version,
                    ..schema("test.size")
                })
            });
        }
    }

    #[test]
    fn pack_registration_keeps_the_largest_schema_version() {
        let mut registry = Registry::new();
        registry
            .register_pack(&pack(|r| {
                r.declare_facet(FacetSchema {
                    version: 2_147_483_647,
                    unit: String::new(),
                    ..schema("test.size")
                })
            }))
            .unwrap();
        assert!(registry.facet_schema("test.size").is_some());
    }

    #[test]
    fn pack_registration_refuses_an_empty_label_or_a_zero_character() {
        assert_refused("test.size", |r| {
            r.declare_facet(FacetSchema {
                label: String::new(),
                ..schema("test.size")
            })
        });
        assert_refused("test.size", |r| {
            r.declare_facet(FacetSchema {
                label: "Si\0ze".into(),
                ..schema("test.size")
            })
        });
        assert_refused("test.size", |r| {
            r.declare_facet(FacetSchema {
                unit: "by\0tes".into(),
                ..schema("test.size")
            })
        });
    }

    #[test]
    fn pack_registration_refuses_an_operator_core_cannot_record() {
        assert_refused("test.bad", |r| {
            r.register_operator(Box::new(Op {
                id: "test.bad",
                version: "",
                writes: &[],
            }))
        });
        assert_refused("test.bad", |r| {
            r.register_operator(Box::new(Op {
                id: "test.bad",
                version: "1\0",
                writes: &[],
            }))
        });
        assert_refused("test.b", |r| r.register_operator(op("test.b\0ad")));
        assert_refused("empty id", |r| r.register_operator(op("")));
    }

    #[test]
    fn pack_registration_names_an_operator_version_with_a_zero_character() {
        let mut registry = Registry::new();
        let error = registry
            .register_pack(&pack(|r| {
                r.register_operator(Box::new(Op {
                    id: "test.op",
                    version: "1\0",
                    writes: &[],
                }))
            }))
            .unwrap_err();
        assert!(error.to_string().contains("test.op"), "{error}");
        assert!(registry.operator("test.op").is_none());
    }

    #[test]
    fn pack_registration_leaves_the_registry_unchanged_when_register_panics() {
        let mut registry = with_earlier_pack();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            registry.register_pack(&pack(|r| {
                r.register_operator(op("test.other"));
                panic!("the pack broke");
            }))
        }));
        assert!(outcome.is_err());
        assert_unchanged(&registry);
    }
}

#[cfg(test)]
mod accepted_media_types {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct TestPack(Box<dyn Fn(&mut Registry)>);
    impl Pack for TestPack {
        fn id(&self) -> &str {
            "test"
        }
        fn register(&self, registry: &mut Registry) {
            (self.0)(registry)
        }
    }

    /// An Operator that accepts `accepts`, widened to `image/gif` once
    /// `widened` is set, and records whether it ran.
    struct Accepting {
        accepts: &'static [&'static str],
        widened: Arc<AtomicBool>,
        ran: Arc<AtomicBool>,
    }
    impl Operator for Accepting {
        fn id(&self) -> &str {
            "test.op"
        }
        fn version(&self) -> &str {
            "1"
        }
        fn writes(&self) -> &[&str] {
            &[]
        }
        fn accepts(&self) -> &[&str] {
            if self.widened.load(Ordering::SeqCst) {
                &["image/png", "image/gif"]
            } else {
                self.accepts
            }
        }
        fn run(&self, _: &mut AssetContext<'_>) -> Result<(), OperatorError> {
            self.ran.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    struct Flags {
        widened: Arc<AtomicBool>,
        ran: Arc<AtomicBool>,
    }

    fn register(accepts: &'static [&'static str]) -> (Registry, Result<(), RegistryError>, Flags) {
        let flags = Flags {
            widened: Arc::default(),
            ran: Arc::default(),
        };
        let (widened, ran) = (flags.widened.clone(), flags.ran.clone());
        let mut registry = Registry::new();
        let outcome = registry.register_pack(&TestPack(Box::new(move |r| {
            r.declare_facet(FacetSchema {
                name: "test.size".into(),
                label: "Size".into(),
                value_type: FacetType::Count,
                unit: String::new(),
                version: 1,
                scope: FacetScope::AssetVersion,
            });
            r.register_operator(Box::new(Accepting {
                accepts,
                widened: widened.clone(),
                ran: ran.clone(),
            }))
        })));
        (registry, outcome, flags)
    }

    struct NoBytes;
    impl Candidate for NoBytes {
        fn size(&self) -> u64 {
            0
        }
        fn read_at(&mut self, _: u64, _: &mut [u8]) -> std::io::Result<usize> {
            Ok(0)
        }
    }

    fn execute(registry: &Registry, media_type: &str) -> Result<OperatorProposals, ExecutionError> {
        registry.execute_operator(
            "test.op",
            &mut NoBytes,
            media_type,
            &mut Discard,
            &mut |_| {},
            &|| false,
        )
    }

    #[test]
    fn accepts_records_the_media_types_an_operator_registers_with() {
        let (registry, outcome, _) = register(&["image/png", "image/jpeg"]);
        outcome.unwrap();
        let entry = registry.operator("test.op").unwrap();
        assert_eq!(entry.accepts(), ["image/png", "image/jpeg"]);
        assert!(entry.accepts_media_type("image/png"));
        assert!(entry.accepts_media_type("image/jpeg"));
        assert!(!entry.accepts_media_type("image/gif"));
        assert!(!entry.accepts_media_type("image/PNG"));
    }

    #[test]
    fn accepts_nothing_is_refused_and_adds_nothing() {
        let (registry, outcome, _) = register(&[]);
        let error = outcome.unwrap_err();
        assert_eq!(
            error,
            RegistryError::InvalidOperator {
                operator: "test.op".into(),
                reason: "it accepts no media type",
            }
        );
        assert!(error.to_string().contains("test.op"), "{error}");
        assert!(registry.operator("test.op").is_none());
        assert!(registry.facet_schema("test.size").is_none());
    }

    #[test]
    fn accepts_a_malformed_media_type_is_refused_and_adds_nothing() {
        for accepts in [
            &["Image/PNG; q=1"][..],
            &["image/png", "image/png; q=1"],
            &["image"],
            &["image/"],
            &["/png"],
            &["image/png/x"],
        ] {
            let accepts: &'static [&'static str] = Box::leak(accepts.to_vec().into_boxed_slice());
            let (registry, outcome, _) = register(accepts);
            let error = outcome.unwrap_err();
            assert_eq!(
                error,
                RegistryError::InvalidOperator {
                    operator: "test.op".into(),
                    reason: "it accepts a media type that is not well formed",
                },
                "{accepts:?}"
            );
            assert!(registry.operator("test.op").is_none());
            assert!(registry.facet_schema("test.size").is_none());
        }
    }

    #[test]
    fn accepts_is_checked_before_the_operator_runs() {
        let (registry, outcome, flags) = register(&["image/png"]);
        outcome.unwrap();
        assert_eq!(
            execute(&registry, "image/gif").unwrap_err(),
            ExecutionError::NotAccepted
        );
        assert!(!flags.ran.load(Ordering::SeqCst));
        execute(&registry, "image/png").unwrap();
        assert!(flags.ran.load(Ordering::SeqCst));
    }

    #[test]
    fn accepts_keeps_the_media_types_it_registered_with() {
        let (registry, outcome, flags) = register(&["image/png"]);
        outcome.unwrap();
        flags.widened.store(true, Ordering::SeqCst);
        assert_eq!(
            registry.operator("test.op").unwrap().accepts(),
            ["image/png"]
        );
        assert_eq!(
            execute(&registry, "image/gif").unwrap_err(),
            ExecutionError::NotAccepted
        );
        assert!(!flags.ran.load(Ordering::SeqCst));
    }

    /// An Operator that proposes the length of the media type it reads.
    struct ReadsMediaType;
    impl Operator for ReadsMediaType {
        fn id(&self) -> &str {
            "test.reads"
        }
        fn version(&self) -> &str {
            "1"
        }
        fn writes(&self) -> &[&str] {
            &["test.size"]
        }
        fn accepts(&self) -> &[&str] {
            &["image/png"]
        }
        fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
            let len = context.media_type().map_or(-1, |m| m.len() as i64);
            context
                .propose_facet("test.size", FacetValue::Integer(len))
                .map_err(|_| OperatorError::Failed("proposal".into()))
        }
    }

    #[test]
    fn media_type_is_the_versions_in_an_operator_run() {
        let mut registry = Registry::new();
        registry.register_operator(Box::new(ReadsMediaType));
        let run = registry
            .execute_operator(
                "test.reads",
                &mut NoBytes,
                "image/png",
                &mut Discard,
                &mut |_| {},
                &|| false,
            )
            .unwrap();
        assert_eq!(run.proposals[0].value, FacetValue::Integer(9));
    }

    #[test]
    fn media_type_is_none_in_a_source_run() {
        struct Looks(Arc<AtomicBool>);
        impl Source for Looks {
            fn id(&self) -> &str {
                "test.source"
            }
            fn version(&self) -> &str {
                "1"
            }
            fn run(&self, context: &mut AssetContext<'_>) -> Result<(), SourceError> {
                self.0
                    .store(context.media_type().is_none(), Ordering::SeqCst);
                context
                    .propose_asset(AssetProposal::new("image", "image/png"))
                    .map_err(|_| SourceError::Failed("p".into()))
            }
        }
        let saw_none = Arc::new(AtomicBool::new(false));
        let mut registry = Registry::new();
        registry.register_source(Box::new(Looks(saw_none.clone())));
        registry
            .execute_source("test.source", &mut NoBytes, &mut |_| {}, &|| false)
            .unwrap();
        assert!(saw_none.load(Ordering::SeqCst));
    }
}

#[cfg(test)]
mod preview_output {
    use super::*;

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

    /// The host's destination: it keeps every write, or refuses them all.
    #[derive(Default)]
    struct Kept {
        bytes: Vec<u8>,
        writes: usize,
        refuse: bool,
    }
    impl Output for Kept {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
            self.writes += 1;
            if self.refuse {
                return Err(std::io::Error::other("the disk is full"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(())
        }
    }

    type Script = fn(&mut AssetContext<'_>) -> Result<(), OperatorError>;

    struct Previewing {
        makes_previews: bool,
        script: Script,
    }
    impl Operator for Previewing {
        fn id(&self) -> &str {
            "test.preview"
        }
        fn version(&self) -> &str {
            "1.2.3"
        }
        fn writes(&self) -> &[&str] {
            &[]
        }
        fn accepts(&self) -> &[&str] {
            &["image/png"]
        }
        fn makes_previews(&self) -> bool {
            self.makes_previews
        }
        fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
            (self.script)(context)
        }
    }

    /// An Operator that declares nothing about previews.
    struct Plain;
    impl Operator for Plain {
        fn id(&self) -> &str {
            "test.plain"
        }
        fn version(&self) -> &str {
            "1"
        }
        fn writes(&self) -> &[&str] {
            &[]
        }
        fn accepts(&self) -> &[&str] {
            &["image/png"]
        }
        fn run(&self, _: &mut AssetContext<'_>) -> Result<(), OperatorError> {
            Ok(())
        }
    }

    fn run(script: Script, output: &mut Kept) -> Result<OperatorProposals, ExecutionError> {
        let mut registry = Registry::new();
        registry.register_operator(Box::new(Previewing {
            makes_previews: true,
            script,
        }));
        registry.execute_operator(
            "test.preview",
            &mut Bytes(b"png".to_vec()),
            "image/png",
            output,
            &mut |_| {},
            &|| false,
        )
    }

    fn jpeg() -> AssetProposal {
        AssetProposal::new("image", "image/jpeg")
    }

    #[test]
    fn preview_registration_records_whether_an_operator_makes_previews() {
        let mut registry = Registry::new();
        registry.register_operator(Box::new(Previewing {
            makes_previews: true,
            script: |_| Ok(()),
        }));
        registry.register_operator(Box::new(Plain));
        assert!(registry.operator("test.preview").unwrap().makes_previews());
        assert!(!registry.operator("test.plain").unwrap().makes_previews());
        let run = registry
            .execute_operator(
                "test.plain",
                &mut Bytes(vec![]),
                "image/png",
                &mut Kept::default(),
                &mut |_| {},
                &|| false,
            )
            .unwrap();
        assert!(!run.makes_previews);
        assert_eq!(run.preview, None);
        assert_eq!(run.preview_bytes, 0);
    }

    #[test]
    fn preview_proposed_and_written_reaches_the_output_and_the_result() {
        let mut output = Kept::default();
        let run = run(
            |context| {
                context.propose_preview(jpeg())?;
                context.write_preview(b"01234")?;
                context.write_preview(b"56789")?;
                Ok(())
            },
            &mut output,
        )
        .unwrap();
        assert_eq!(output.bytes, b"0123456789");
        assert!(run.makes_previews);
        assert_eq!(run.preview, Some(jpeg()));
        assert_eq!(run.preview_bytes, 10);
        assert_eq!(run.producer_id, "test.preview");
        assert_eq!(run.producer_version, "1.2.3");
    }

    #[test]
    fn preview_proposed_by_a_source_is_refused_and_fails_its_run() {
        struct Proposer;
        impl Source for Proposer {
            fn id(&self) -> &str {
                "test.source"
            }
            fn version(&self) -> &str {
                "1"
            }
            fn run(&self, context: &mut AssetContext<'_>) -> Result<(), SourceError> {
                assert_eq!(
                    context.propose_preview(AssetProposal::new("image", "image/jpeg")),
                    Err(ContextError::NotAllowed)
                );
                assert_eq!(
                    context.write_preview(b"bytes"),
                    Err(ContextError::NotAllowed)
                );
                context
                    .propose_asset(AssetProposal::new("image", "image/png"))
                    .map_err(|_| SourceError::Failed("p".into()))
            }
        }
        let mut registry = Registry::new();
        registry.register_source(Box::new(Proposer));
        let outcome =
            registry.execute_source("test.source", &mut Bytes(vec![]), &mut |_| {}, &|| false);
        assert!(
            matches!(outcome, Err(ExecutionError::Failed(_))),
            "{outcome:?}"
        );
    }

    #[test]
    fn preview_proposed_twice_fails_the_run() {
        let outcome = run(
            |context| {
                context.propose_preview(jpeg())?;
                assert_eq!(
                    context.propose_preview(AssetProposal::new("image", "image/png")),
                    Err(ContextError::AlreadyProposed)
                );
                context.write_preview(b"bytes")?;
                Ok(())
            },
            &mut Kept::default(),
        );
        assert_eq!(
            outcome,
            Err(ExecutionError::Failed(
                "proposed more than one preview".into()
            ))
        );
    }

    #[test]
    fn preview_written_before_any_proposal_fails_the_run() {
        let mut output = Kept::default();
        let outcome = run(
            |context| {
                assert_eq!(
                    context.write_preview(b"bytes"),
                    Err(ContextError::NotAllowed)
                );
                context.propose_preview(jpeg())?;
                context.write_preview(b"bytes")?;
                Ok(())
            },
            &mut output,
        );
        assert_eq!(
            outcome,
            Err(ExecutionError::Failed(
                "wrote preview bytes before proposing a preview".into()
            ))
        );
        assert_eq!(
            output.bytes, b"bytes",
            "the first write never reached the host"
        );
    }

    #[test]
    fn preview_malformed_proposal_is_refused() {
        let outcome = run(
            |context| {
                assert_eq!(
                    context.propose_preview(AssetProposal::new("image", "JPEG")),
                    Err(ContextError::InvalidProposal)
                );
                Ok(())
            },
            &mut Kept::default(),
        )
        .unwrap();
        assert_eq!(outcome.preview, None);
    }

    #[test]
    fn preview_write_the_output_refuses_is_a_write_error() {
        let mut output = Kept {
            refuse: true,
            ..Kept::default()
        };
        let run = run(
            |context| {
                context.propose_preview(jpeg())?;
                assert_eq!(context.write_preview(b"bytes"), Err(ContextError::Write));
                Ok(())
            },
            &mut output,
        )
        .unwrap();
        assert_eq!(output.writes, 1);
        assert_eq!(run.preview_bytes, 0);
    }

    #[test]
    fn preview_write_error_converts_to_an_output_error() {
        assert_eq!(
            OperatorError::from(ContextError::Write),
            OperatorError::Output
        );
        assert!(matches!(
            OperatorError::from(ContextError::Read),
            OperatorError::Failed(_)
        ));
    }

    #[test]
    fn preview_output_error_stops_the_run_as_an_output_error() {
        let mut output = Kept {
            refuse: true,
            ..Kept::default()
        };
        let outcome = run(
            |context| {
                context.propose_preview(jpeg())?;
                context.write_preview(b"bytes")?;
                Ok(())
            },
            &mut output,
        );
        assert_eq!(outcome, Err(ExecutionError::Output));
    }
}
