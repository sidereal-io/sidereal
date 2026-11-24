//! Run one registered Operator over one Asset version, record the attempt as
//! an operation run, and store the facet values it proposed (ADR-008).
//!
//! Core is the only writer of facet values. It checks every proposal against
//! the Registry's schemas and the Operator's grants before it stores any, and
//! takes the producer from the Operator's registration, never from the run.
use crate::{
    database::{Database, DatabaseError},
    storage::objects::ObjectStore,
};
use sidereal_plugin_abi::{
    Candidate, FacetProposals, FacetScope, FacetType, FacetValue, Registry, MAX_COUNT,
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
}

/// Why core could not report a recorded outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationError {
    /// Core could not record the run's start, or could not record or read
    /// back how it ended. The outcome is unknown.
    Unrecorded(DatabaseError),
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unrecorded(error) => error.fmt(f),
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

/// The version row a run reads.
struct Version {
    asset_id: Uuid,
    content_hash: String,
    byte_size: u64,
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
        .expect("upload names a registered operator");
    let row: (Uuid, String, i64) = sqlx::query_as(
        "SELECT asset_id, content_hash, byte_size FROM public.asset_versions WHERE id = $1",
    )
    .bind(run.asset_version_id)
    .fetch_one(database.pool())
    .await?;
    let version = Version {
        asset_id: row.0,
        content_hash: row.1,
        byte_size: row.2 as u64,
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

    let objects = objects.clone();
    let operator_id = run.operator_id.clone();
    let hash = version.content_hash.clone();
    let size = version.byte_size;
    let flag = cancelled.clone();
    let blocking = registry.clone();
    let proposals = tokio::task::spawn_blocking(move || {
        let (file, _) = objects.open_object(&hash).expect("the object opens");
        blocking.execute_operator(
            &operator_id,
            &mut ObjectBytes { file, size },
            &mut |line| eprintln!("operator {operator_id}: {line:?}"),
            &|| flag.load(Ordering::SeqCst),
        )
    })
    .await
    .expect("the operator finishes")
    .expect("the operator succeeds");

    if let Err(reason) = check(&registry, &proposals) {
        return Ok(record_failure(database, run_id, &reason).await?);
    }
    let stored = store(
        database,
        &registry,
        run_id,
        run.asset_version_id,
        &version,
        &proposals,
    )
    .await?;
    match stored {
        Ok(()) => Ok(RunOutcome::Succeeded),
        Err(reason) => Ok(record_failure(database, run_id, &reason).await?),
    }
}

/// Check every proposal against the Registry before any is stored: its
/// facet has a schema, it is in the Operator's registered grants, and its
/// value fits the schema's type. The reason names the first facet refused.
fn check(registry: &Registry, proposals: &FacetProposals) -> Result<(), String> {
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

/// Record the run `failed` with `reason`, and answer what its row says.
async fn record_failure(
    database: &Database,
    run_id: Uuid,
    reason: &str,
) -> Result<RunOutcome, sqlx::Error> {
    sqlx::query(
        "UPDATE public.operation_runs \
         SET status = 'failed', reason = $2, ended_at = greatest(now(), started_at) \
         WHERE id = $1",
    )
    .bind(run_id)
    .bind(reason)
    .execute(database.pool())
    .await?;
    Ok(RunOutcome::Failed(reason.to_owned()))
}

/// The kept row of a schema version, in the order core compares it.
type KeptSchema = (String, String, String, String, String);

/// In one transaction: keep each used schema version, insert every value
/// against the subject its scope names, and mark the run `succeeded`. A kept
/// schema version that differs from the running declaration refuses the run
/// (the inner error) and stores nothing.
async fn store(
    database: &Database,
    registry: &Registry,
    run_id: Uuid,
    version_id: Uuid,
    version: &Version,
    proposals: &FacetProposals,
) -> Result<Result<(), String>, sqlx::Error> {
    let mut tx = database.pool().begin().await?;
    for proposal in &proposals.proposals {
        let declared = registry
            .facet_schema(&proposal.name)
            .expect("check found the schema");
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
    sqlx::query(
        "UPDATE public.operation_runs \
         SET status = 'succeeded', ended_at = greatest(now(), started_at) WHERE id = $1",
    )
    .bind(run_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Ok(()))
}

#[cfg(test)]
mod fixture;
#[cfg(test)]
mod refuses;
#[cfg(test)]
mod stores;
