//! Ingest one staged candidate: run the selected Source over it, publish the
//! object, then find the Asset that already holds those bytes or commit a
//! new Asset and its first version (ADR-011 order).
//!
//! Nothing is committed unless the Source accepted the candidate and the
//! object and its folder chain are durable. A failure after publication
//! leaves the object in place: another version may already use it.
use crate::{
    assets::{AssetRecord, NewVersion},
    database::{Database, DatabaseError},
    storage::{
        objects::{ObjectStore, Staged},
        StorageError,
    },
};
use sidereal_plugin_abi::{Candidate, ExecutionError, Registry};
use std::{
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
#[cfg(test)]
use uuid::Uuid;

/// What an ingest that succeeded did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ingested {
    /// No version had the bytes' hash, so ingest committed a new Asset.
    Created(AssetRecord),
    /// A version of this Asset already had the bytes' hash, so ingest
    /// committed nothing and left the Asset as it was.
    Found(AssetRecord),
}

/// Why an ingest committed nothing.
#[derive(Debug)]
pub enum IngestError {
    /// The display name is empty.
    EmptyName,
    /// The Source refused the candidate, with its reason.
    Refused(String),
    /// The Source could not run or failed.
    Source(ExecutionError),
    /// The caller cancelled before publication.
    Cancelled,
    Storage(StorageError),
    Database(DatabaseError),
}

impl fmt::Display for IngestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => f.write_str("the upload has no file name"),
            Self::Refused(reason) => write!(f, "the source refused the upload: {reason}"),
            Self::Source(error) => error.fmt(f),
            Self::Cancelled => f.write_str("the upload was cancelled"),
            Self::Storage(error) => error.fmt(f),
            Self::Database(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for IngestError {}

/// The staged file, as the Source's context reads it.
struct StagedBytes<'a>(&'a mut Staged);

impl Candidate for StagedBytes<'_> {
    fn size(&self) -> u64 {
        self.0.size()
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read_at(offset, buf)
    }
}

/// Ingest `staged` through the Source registered as `source`. When a version
/// of an existing Asset has the same bytes, the result is that Asset, found
/// and unchanged; otherwise it is a new Asset named `name`. `cancelled` is checked during the Source run and once
/// more before publication; after commit, the Asset stays.
pub async fn ingest(
    database: &Database,
    objects: &ObjectStore,
    registry: Arc<Registry>,
    source: &str,
    name: &str,
    staged: Staged,
    cancelled: Arc<AtomicBool>,
) -> Result<Ingested, IngestError> {
    if name.is_empty() {
        return Err(IngestError::EmptyName);
    }
    let objects = objects.clone();
    let source = source.to_owned();
    let (accepted, published) = tokio::task::spawn_blocking(move || {
        let mut staged = staged;
        let is_cancelled = || cancelled.load(Ordering::SeqCst);
        let accepted = registry
            .execute_source(
                &source,
                &mut StagedBytes(&mut staged),
                &mut |line| eprintln!("source {source}: {line:?}"),
                &is_cancelled,
            )
            .map_err(|error| match error {
                ExecutionError::Refused(reason) => IngestError::Refused(reason),
                ExecutionError::Cancelled => IngestError::Cancelled,
                other => IngestError::Source(other),
            })?;
        if is_cancelled() {
            return Err(IngestError::Cancelled);
        }
        let published = objects.publish(staged).map_err(IngestError::Storage)?;
        Ok((accepted, published))
    })
    .await
    .map_err(|_| IngestError::Source(ExecutionError::Failed("the ingest task stopped".into())))??;
    database
        .find_or_create_asset(NewVersion {
            name,
            accepted: &accepted,
            content_hash: &published.hash,
            byte_size: published.size,
        })
        .await
        .map_err(IngestError::Database)
}

#[cfg(test)]
mod tests;
