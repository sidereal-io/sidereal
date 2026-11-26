//! Run one registered Operator over one Asset version, record the attempt as
//! an operation run, and store the facet values and the preview it proposed
//! (ADR-008, ADR-018).
//!
//! Core is the only writer of facet values and derived Assets. It checks
//! every proposal against the Registry's schemas, the Operator's grants and
//! its registration before it stores any, and takes the producer from the
//! Operator's registration, never from the run.
use crate::{
    database::{Database, DatabaseError},
    storage::objects::{ObjectStore, Published, StageError, Staging},
};
use sidereal_plugin_abi::{
    AssetProposal, Candidate, ExecutionError, FacetScope, FacetType, FacetValue, OperatorProposals,
    Output, Registry, MAX_COUNT,
};
use std::{
    fs::File,
    os::unix::fs::FileExt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use uuid::Uuid;

/// Which Operator to run, over which version, for whom.
#[derive(Clone, Debug)]
pub struct RunRequest {
    pub operator_id: String,
    pub asset_version_id: Uuid,
    /// The `users.id` of the user whose action started the run.
    pub actor: i64,
}

/// How a recorded run ended, as its row says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunOutcome {
    Succeeded,
    /// The run is `failed`, with this reason, and stored no value.
    Failed(String),
    /// The Operator does not accept the version, because the version
    /// belongs to a derived Asset or the Operator did not register its media
    /// type. Core recorded no run and the Operator did not run.
    NotAccepted,
}

/// Why core could not report a recorded outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationError {
    /// Core could not record the run's start, or could not record or read
    /// back how it ended. The outcome is unknown.
    Unrecorded(DatabaseError),
    /// No Operator with that id is registered; nothing was recorded.
    UnknownOperator,
    /// No Asset version has that id; nothing was recorded.
    UnknownVersion,
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unrecorded(error) => error.fmt(f),
            Self::UnknownOperator => f.write_str("no such operator is registered"),
            Self::UnknownVersion => f.write_str("no such asset version exists"),
        }
    }
}

impl std::error::Error for OperationError {}

impl From<sqlx::Error> for OperationError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unrecorded(error.into())
    }
}

/// The stored object of one version, read through positioned reads and no
/// longer than the version recorded.
struct ObjectBytes {
    file: File,
    size: u64,
}

impl Candidate for ObjectBytes {
    fn size(&self) -> u64 {
        self.size
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        let remaining = self.size.saturating_sub(offset);
        let len = buf.len().min(remaining.try_into().unwrap_or(usize::MAX));
        self.file.read_at(&mut buf[..len], offset)
    }
}

/// Where an Operator's preview bytes go: a staging file, begun on the first
/// write. The first failure is kept, and every later write is refused, so a
/// failed preview is never published, even when the Operator ignores the
/// error and reports success.
struct StagingOutput {
    objects: ObjectStore,
    staging: Option<Staging>,
    failure: Option<&'static str>,
}

impl StagingOutput {
    fn new(objects: ObjectStore) -> Self {
        Self {
            objects,
            staging: None,
            failure: None,
        }
    }

    fn stage(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        let staging = match &mut self.staging {
            Some(staging) => staging,
            None => self
                .staging
                .insert(self.objects.begin().map_err(|_| PREVIEW_NOT_STORED)?),
        };
        staging.write(bytes).map_err(|error| match error {
            StageError::TooLarge => PREVIEW_TOO_LARGE,
            StageError::Storage(_) => PREVIEW_NOT_STORED,
        })
    }
}

impl Output for StagingOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(failure) = self.failure {
            return Err(std::io::Error::other(failure));
        }
        self.stage(bytes).map_err(|failure| {
            self.failure = Some(failure);
            self.staging = None;
            std::io::Error::other(failure)
        })
    }
}

/// A run's checked preview, staged and not yet published.
struct StagedPreview {
    proposal: AssetProposal,
    staging: Staging,
}

/// The version row a run reads.
struct Version {
    asset_id: Uuid,
    content_hash: String,
    byte_size: u64,
    media_type: String,
}

fn scope_text(scope: FacetScope) -> &'static str {
    match scope {
        FacetScope::Asset => "asset",
        FacetScope::AssetVersion => "asset_version",
    }
}

fn type_text(value_type: FacetType) -> &'static str {
    match value_type {
        FacetType::Count => "count",
    }
}

fn json(value: FacetValue) -> serde_json::Value {
    match value {
        FacetValue::Integer(n) => n.into(),
    }
}

/// One stored facet value as a reader sees it: no provenance, and the label
/// and unit of the schema version it was written under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FacetEntry {
    pub name: String,
    pub label: String,
    pub value: i64,
    pub unit: String,
}

impl Database {
    /// The facet values of the Asset `asset_id` and of its version
    /// `version_id`, ordered by facet name, then write time, then id.
    pub async fn facets(
        &self,
        asset_id: Uuid,
        version_id: Uuid,
    ) -> Result<Vec<FacetEntry>, DatabaseError> {
        let rows: Vec<(String, String, String, serde_json::Value, String)> = sqlx::query_as(
            "SELECT v.facet, s.label, s.value_type, v.value, s.unit \
             FROM public.facet_values v \
             JOIN public.facet_schemas s ON s.name = v.facet AND s.version = v.schema_version \
             WHERE v.asset_id = $1 OR v.asset_version_id = $2 \
             ORDER BY v.facet, v.written_at, v.id",
        )
        .bind(asset_id)
        .bind(version_id)
        .fetch_all(self.pool())
        .await?;
        rows.into_iter()
            .map(|(name, label, value_type, value, unit)| {
                // Only core writes these rows, and only counts exist: any
                // other stored value is a damaged database, not a fact.
                let value = match value_type.as_str() {
                    "count" => value.as_i64().filter(|n| (0..=MAX_COUNT).contains(n)),
                    _ => None,
                }
                .ok_or(DatabaseError::Schema)?;
                Ok(FacetEntry {
                    name,
                    label,
                    value,
                    unit,
                })
            })
            .collect()
    }
}

/// Run the Operator that `run` names over its Asset version, record the run,
/// and store its values when every proposal passes.
pub async fn run_operator(
    database: &Database,
    objects: &ObjectStore,
    registry: Arc<Registry>,
    run: RunRequest,
    cancelled: Arc<AtomicBool>,
) -> Result<RunOutcome, OperationError> {
    let entry = registry
        .operator(&run.operator_id)
        .ok_or(OperationError::UnknownOperator)?;
    let row: (Uuid, String, i64, String, String) = sqlx::query_as(
        "SELECT v.asset_id, v.content_hash, v.byte_size, v.media_type, a.origin \
         FROM public.asset_versions v JOIN public.assets a ON a.id = v.asset_id \
         WHERE v.id = $1",
    )
    .bind(run.asset_version_id)
    .fetch_optional(database.pool())
    .await?
    .ok_or(OperationError::UnknownVersion)?;
    // An Operator lists originals' media types only, so no Operator accepts
    // a derived Asset's version, such as a preview (ADR-019).
    if row.4 != "source" || !entry.accepts_media_type(&row.3) {
        return Ok(RunOutcome::NotAccepted);
    }
    let version = Version {
        asset_id: row.0,
        content_hash: row.1,
        byte_size: row.2 as u64,
        media_type: row.3,
    };
    let run_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO public.operation_runs \
         (id, operator_id, operator_version, asset_version_id, actor_user_id, status, started_at) \
         VALUES ($1, $2, $3, $4, $5, 'running', now())",
    )
    .bind(run_id)
    .bind(entry.id())
    .bind(entry.version())
    .bind(run.asset_version_id)
    .bind(run.actor)
    .execute(database.pool())
    .await?;

    let (proposals, staging) =
        match execute(objects, &registry, &run.operator_id, &version, &cancelled).await {
            Ok(ran) => ran,
            Err(reason) => return Ok(record_failure(database, run_id, &reason).await?),
        };
    let preview = match check(&registry, &proposals, staging) {
        Ok(preview) => preview,
        Err(reason) => return Ok(record_failure(database, run_id, &reason).await?),
    };
    if cancelled.load(Ordering::SeqCst) {
        return Ok(record_failure(database, run_id, CANCELLED).await?);
    }
    let published = match preview {
        None => None,
        Some(preview) => match publish(objects, preview.staging).await {
            Ok(published) => Some((preview.proposal, published)),
            Err(reason) => return Ok(record_failure(database, run_id, reason).await?),
        },
    };
    // Publication can take a while; a cancellation during it stops the run
    // before core records anything. An object it published stays, unused.
    if cancelled.load(Ordering::SeqCst) {
        return Ok(record_failure(database, run_id, CANCELLED).await?);
    }
    let stored = store(
        database,
        &registry,
        run_id,
        run.asset_version_id,
        &version,
        &proposals,
        published.as_ref(),
    )
    .await;
    match stored {
        Ok(Ok(())) => Ok(RunOutcome::Succeeded),
        Ok(Err(reason)) => Ok(record_failure(database, run_id, &reason).await?),
        // The transaction failed, or its commit's answer was lost, so core
        // cannot tell whether the values landed. The conditional update
        // changes the row only when they did not; otherwise core reads back
        // what the row says.
        Err(_) => Ok(record_failure(database, run_id, VALUES_NOT_STORED).await?),
    }
}

const CANCELLED: &str = "the run was cancelled";
const VALUES_NOT_STORED: &str = "the values could not be stored";
const PREVIEW_NOT_STORED: &str = "the preview could not be stored";
const PREVIEW_TOO_LARGE: &str = "the preview is larger than 100 MiB";

/// The media types a browser shows, which a preview may have.
const PREVIEW_MEDIA_TYPES: &[&str] = &["image/jpeg", "image/png"];

/// On a blocking worker: open the version's object, check that its length
/// is the version's recorded size, and run the Operator over it, staging
/// any preview bytes it writes. The error is the reason the run failed; the
/// staging is dropped with it.
async fn execute(
    objects: &ObjectStore,
    registry: &Arc<Registry>,
    operator_id: &str,
    version: &Version,
    cancelled: &Arc<AtomicBool>,
) -> Result<(OperatorProposals, Option<Staging>), String> {
    let objects = objects.clone();
    let registry = registry.clone();
    let operator_id = operator_id.to_owned();
    let hash = version.content_hash.clone();
    let size = version.byte_size;
    let media_type = version.media_type.clone();
    let cancelled = cancelled.clone();
    let outcome = tokio::task::spawn_blocking(move || {
        let (file, length) = objects
            .open_object(&hash)
            .map_err(|error| format!("the stored object could not be opened: {error}"))?;
        if length != size {
            return Err("the stored object does not match its version".to_owned());
        }
        let mut output = StagingOutput::new(objects);
        let outcome = registry.execute_operator(
            &operator_id,
            &mut ObjectBytes { file, size },
            &media_type,
            &mut output,
            &mut |line| eprintln!("operator {operator_id}: {line:?}"),
            &|| cancelled.load(Ordering::SeqCst),
        );
        // A staging failure is what stopped the run when the Operator
        // reports success, or says that a preview write stopped it. Any
        // other failure keeps the Operator's own reason.
        match (outcome, output.failure) {
            (Ok(_) | Err(ExecutionError::Output), Some(failure)) => Err(failure.to_owned()),
            (Err(ExecutionError::Output), None) => Err(PREVIEW_NOT_STORED.to_owned()),
            (Ok(proposals), None) => Ok((proposals, output.staging)),
            (Err(error), _) => Err(match error {
                ExecutionError::Failed(reason) => reason,
                ExecutionError::Unsupported => "the operator cannot run yet".to_owned(),
                ExecutionError::Cancelled => CANCELLED.to_owned(),
                other => other.to_string(),
            }),
        }
    })
    .await;
    outcome.unwrap_or_else(|_| Err("the operator stopped".to_owned()))
}

/// The reason core records: every zero character removed, because
/// PostgreSQL text cannot hold one, and never empty. Every update that
/// records `failed` calls this.
fn cleaned(reason: &str) -> String {
    let reason: String = reason.chars().filter(|&c| c != '\0').collect();
    if reason.trim().is_empty() {
        "the operator gave no reason".to_owned()
    } else {
        reason
    }
}

/// Check every proposal against the Registry before any is stored: a
/// preview comes from an Operator that registered as making previews, has a
/// media type a browser shows and holds bytes; each facet has a schema, is
/// in the Operator's registered grants, and its value fits the schema's
/// type. The reason names the first proposal refused. A refusal drops the
/// staged preview.
fn check(
    registry: &Registry,
    proposals: &OperatorProposals,
    staging: Option<Staging>,
) -> Result<Option<StagedPreview>, String> {
    let preview = match (&proposals.preview, staging) {
        (None, _) => None,
        (Some(_), _) if !proposals.makes_previews => {
            return Err(format!(
                "operator {} does not make previews",
                proposals.producer_id
            ))
        }
        (Some(proposal), _) if !PREVIEW_MEDIA_TYPES.contains(&proposal.media_type.as_str()) => {
            return Err(format!("a preview cannot be {}", proposal.media_type))
        }
        (Some(proposal), Some(staging)) if proposals.preview_bytes > 0 => Some(StagedPreview {
            proposal: proposal.clone(),
            staging,
        }),
        (Some(_), _) => return Err("the preview is empty".to_owned()),
    };
    check_facets(registry, proposals)?;
    Ok(preview)
}

/// On a blocking worker: hash the staged preview and publish it before any
/// record names it (ADR-011).
async fn publish(objects: &ObjectStore, staging: Staging) -> Result<Published, &'static str> {
    let objects = objects.clone();
    tokio::task::spawn_blocking(move || {
        let staged = staging.finish().map_err(|_| PREVIEW_NOT_STORED)?;
        objects.publish(staged).map_err(|_| PREVIEW_NOT_STORED)
    })
    .await
    .unwrap_or(Err(PREVIEW_NOT_STORED))
}

fn check_facets(registry: &Registry, proposals: &OperatorProposals) -> Result<(), String> {
    for proposal in &proposals.proposals {
        let name = &proposal.name;
        let Some(declared) = registry.facet_schema(name) else {
            return Err(format!("facet {name} has no registered schema"));
        };
        if !proposals.grants.iter().any(|grant| grant == name) {
            return Err(format!(
                "facet {name} is not granted to {}",
                proposals.producer_id
            ));
        }
        match (declared.schema.value_type, proposal.value) {
            (FacetType::Count, FacetValue::Integer(n)) if (0..=MAX_COUNT).contains(&n) => {}
            (FacetType::Count, FacetValue::Integer(n)) => {
                return Err(format!(
                    "facet {name} is a count from 0 to {MAX_COUNT}, not {n}"
                ))
            }
        }
    }
    Ok(())
}

/// Record the run `failed` with `reason`, cleaned, and answer what its row
/// says. A run that already ended keeps its row: this then reads it back.
async fn record_failure(
    database: &Database,
    run_id: Uuid,
    reason: &str,
) -> Result<RunOutcome, sqlx::Error> {
    let reason = cleaned(reason);
    let updated = sqlx::query(
        "UPDATE public.operation_runs \
         SET status = 'failed', reason = $2, ended_at = greatest(now(), started_at) \
         WHERE id = $1 AND status = 'running'",
    )
    .bind(run_id)
    .bind(&reason)
    .execute(database.pool())
    .await?;
    if updated.rows_affected() == 1 {
        return Ok(RunOutcome::Failed(reason));
    }
    let (status, kept): (String, Option<String>) =
        sqlx::query_as("SELECT status, reason FROM public.operation_runs WHERE id = $1")
            .bind(run_id)
            .fetch_one(database.pool())
            .await?;
    match (status.as_str(), kept) {
        ("succeeded", _) => Ok(RunOutcome::Succeeded),
        ("failed", Some(kept)) => Ok(RunOutcome::Failed(kept)),
        _ => Err(sqlx::Error::RowNotFound),
    }
}

/// The kept row of a schema version, in the order core compares it.
type KeptSchema = (String, String, String, String, String);

/// In one transaction: keep each used schema version, insert every value
/// against the subject its scope names, record the published preview as a
/// derived Asset with one version and its lineage row, and mark the run
/// `succeeded`. A kept schema version that differs from the running
/// declaration refuses the run (the inner error) and stores nothing. Schema
/// versions are kept in name order, so two runs that first write the same
/// schemas cannot deadlock.
async fn store(
    database: &Database,
    registry: &Registry,
    run_id: Uuid,
    version_id: Uuid,
    version: &Version,
    proposals: &OperatorProposals,
    preview: Option<&(AssetProposal, Published)>,
) -> Result<Result<(), String>, sqlx::Error> {
    let mut tx = database.pool().begin().await?;
    let mut used: Vec<_> = proposals
        .proposals
        .iter()
        .map(|proposal| {
            registry
                .facet_schema(&proposal.name)
                .expect("check found the schema")
        })
        .collect();
    used.sort_by(|a, b| a.schema.name.cmp(&b.schema.name));
    used.dedup_by(|a, b| a.schema.name == b.schema.name);
    for declared in used {
        let schema = &declared.schema;
        sqlx::query(
            "INSERT INTO public.facet_schemas \
             (name, version, owner_pack, label, value_type, unit, scope) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (name, version) DO NOTHING",
        )
        .bind(&schema.name)
        .bind(schema.version as i32)
        .bind(&declared.owner)
        .bind(&schema.label)
        .bind(type_text(schema.value_type))
        .bind(&schema.unit)
        .bind(scope_text(schema.scope))
        .execute(&mut *tx)
        .await?;
        let kept: KeptSchema = sqlx::query_as(
            "SELECT owner_pack, label, value_type, unit, scope FROM public.facet_schemas \
             WHERE name = $1 AND version = $2",
        )
        .bind(&schema.name)
        .bind(schema.version as i32)
        .fetch_one(&mut *tx)
        .await?;
        let running: KeptSchema = (
            declared.owner.clone(),
            schema.label.clone(),
            type_text(schema.value_type).into(),
            schema.unit.clone(),
            scope_text(schema.scope).into(),
        );
        if kept != running {
            return Ok(Err(format!(
                "facet {} version {} changed without a new version",
                schema.name, schema.version
            )));
        }
    }
    for proposal in &proposals.proposals {
        let schema = &registry
            .facet_schema(&proposal.name)
            .expect("check found the schema")
            .schema;
        let (asset_id, asset_version_id) = match schema.scope {
            FacetScope::Asset => (Some(version.asset_id), None),
            FacetScope::AssetVersion => (None, Some(version_id)),
        };
        sqlx::query(
            "INSERT INTO public.facet_values \
             (id, facet, schema_version, asset_id, asset_version_id, value, producer_id, \
              producer_version, run_id, written_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now())",
        )
        .bind(Uuid::new_v4())
        .bind(&schema.name)
        .bind(schema.version as i32)
        .bind(asset_id)
        .bind(asset_version_id)
        .bind(json(proposal.value))
        .bind(&proposals.producer_id)
        .bind(&proposals.producer_version)
        .bind(run_id)
        .execute(&mut *tx)
        .await?;
    }
    if let Some((proposal, published)) = preview {
        let name: String = sqlx::query_scalar("SELECT name FROM public.assets WHERE id = $1")
            .bind(version.asset_id)
            .fetch_one(&mut *tx)
            .await?;
        let (asset, derived) = (Uuid::new_v4(), Uuid::new_v4());
        sqlx::query(
            "INSERT INTO public.assets (id, kind, name, current_version_id, origin) \
             VALUES ($1, $2, $3, $4, 'derived')",
        )
        .bind(asset)
        .bind(&proposal.kind)
        .bind(&name)
        .bind(derived)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO public.asset_versions \
             (id, asset_id, content_hash, byte_size, media_type, source_id, source_version, ingested_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, now())",
        )
        .bind(derived)
        .bind(asset)
        .bind(&published.hash)
        .bind(published.size as i64)
        .bind(&proposal.media_type)
        .bind(&proposals.producer_id)
        .bind(&proposals.producer_version)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO public.asset_lineage (output_version_id, input_version_id, role, run_id) \
             VALUES ($1, $2, 'preview', $3)",
        )
        .bind(derived)
        .bind(version_id)
        .bind(run_id)
        .execute(&mut *tx)
        .await?;
    }
    // The clock time, not the transaction's start, records when the run was
    // marked succeeded: a version's newest preview is the one whose run ended
    // last.
    let ended = sqlx::query(
        "UPDATE public.operation_runs \
         SET status = 'succeeded', ended_at = greatest(clock_timestamp(), started_at) \
         WHERE id = $1 AND status = 'running'",
    )
    .bind(run_id)
    .execute(&mut *tx)
    .await?;
    if ended.rows_affected() != 1 {
        return Ok(Err("the run had already ended".to_owned()));
    }
    tx.commit().await?;
    Ok(Ok(()))
}

#[cfg(test)]
mod accepts;
#[cfg(test)]
mod derives;
#[cfg(test)]
mod fails;
#[cfg(test)]
mod finished;
#[cfg(test)]
mod fixture;
#[cfg(test)]
mod refuses;
#[cfg(test)]
mod stores;
