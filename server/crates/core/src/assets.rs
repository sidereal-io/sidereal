//! Assets and their immutable versions in PostgreSQL (ADR-003).
//!
//! Core mints every id and records the hash it computed. This module finds
//! the original that already holds a set of bytes, or creates an original
//! with its first version, and reads originals back with their previews; it
//! offers no way to change or delete a version. A derived Asset, such as a
//! preview, is read only as the preview of its original (ADR-018).
use crate::{
    database::{Database, DatabaseError},
    ingest::Ingested,
};
use sidereal_plugin_abi::Accepted;
use sqlx::{postgres::PgRow, Row};
use uuid::Uuid;

/// One original Asset with its current version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetRecord {
    pub id: Uuid,
    pub kind: String,
    /// Display text only; never a path.
    pub name: String,
    pub current_version_id: Uuid,
    pub version: VersionRecord,
    /// Whether the current version has a preview.
    pub has_preview: bool,
}

/// One immutable Asset version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionRecord {
    pub id: Uuid,
    /// Lowercase hexadecimal BLAKE3 hash of the stored bytes.
    pub content_hash: String,
    pub byte_size: u64,
    pub media_type: String,
    pub source_id: String,
    pub source_version: String,
    /// When core committed the version, in UTC, as RFC 3339 text.
    pub ingested_at: String,
}

/// The new Asset's first version, as ingest hands it to the database.
pub(crate) struct NewVersion<'a> {
    pub name: &'a str,
    pub accepted: &'a Accepted,
    pub content_hash: &'a str,
    pub byte_size: u64,
}

/// The first key of the transaction lock that serializes uploads of the
/// same bytes: "Asset content". It differs from the startup lock's key.
const ASSET_CONTENT: i32 = 1_095_979_860;

/// The second key of that lock: the bits of the hash's first 8 hex digits,
/// read as a `u32` and kept as an `i32`. Hashes that share these digits only
/// wait for each other; the lookup still matches the full hash. A hash core
/// did not compute, which the table's check refuses anyway, gets key 0.
pub(crate) fn content_lock_key(hash: &str) -> i32 {
    hash.get(..8)
        .and_then(|digits| u32::from_str_radix(digits, 16).ok())
        .unwrap_or(0) as i32
}

const INGESTED_AT: &str =
    r#"to_char(v.ingested_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"')"#;

/// The preview of the version `v`: the derived version linked to it as its
/// preview by the run with the latest recorded completion time, then the
/// greatest run id.
const PREVIEW: &str = "LEFT JOIN LATERAL ( \
     SELECT l.output_version_id FROM public.asset_lineage l \
     JOIN public.operation_runs r ON r.id = l.run_id \
     WHERE l.input_version_id = v.id AND l.role = 'preview' \
     ORDER BY r.ended_at DESC, r.id DESC LIMIT 1) p ON true";

/// Every original with its current version and whether that version has a
/// preview. Callers add conditions with `AND`.
fn select() -> String {
    format!(
        "SELECT a.id, a.kind, a.name, a.current_version_id, v.id, v.content_hash, v.byte_size, \
         v.media_type, v.source_id, v.source_version, {INGESTED_AT}, \
         p.output_version_id IS NOT NULL \
         FROM public.assets a \
         JOIN public.asset_versions v ON v.asset_id = a.id AND v.id = a.current_version_id \
         {PREVIEW} WHERE a.origin = 'source'"
    )
}

fn record(row: &PgRow) -> Result<AssetRecord, sqlx::Error> {
    Ok(AssetRecord {
        id: row.try_get(0)?,
        kind: row.try_get(1)?,
        name: row.try_get(2)?,
        current_version_id: row.try_get(3)?,
        version: version_at(row, 4)?,
        has_preview: row.try_get(11)?,
    })
}

/// The version whose columns start at `first`, in `select`'s order.
fn version_at(row: &PgRow, first: usize) -> Result<VersionRecord, sqlx::Error> {
    Ok(VersionRecord {
        id: row.try_get(first)?,
        content_hash: row.try_get(first + 1)?,
        byte_size: row.try_get::<i64, _>(first + 2)? as u64,
        media_type: row.try_get(first + 3)?,
        source_id: row.try_get(first + 4)?,
        source_version: row.try_get(first + 5)?,
        ingested_at: row.try_get(first + 6)?,
    })
}

impl Database {
    /// Every committed original, oldest first, then by id.
    pub async fn assets(&self) -> Result<Vec<AssetRecord>, DatabaseError> {
        let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
            "{} ORDER BY v.ingested_at, a.id",
            select()
        )))
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(record).collect::<Result<_, _>>()?)
    }

    /// One committed original, when it exists. A derived Asset's id reads as
    /// missing.
    pub async fn asset(&self, id: Uuid) -> Result<Option<AssetRecord>, DatabaseError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!("{} AND a.id = $1", select())))
            .bind(id)
            .fetch_optional(self.pool())
            .await?;
        Ok(row.as_ref().map(record).transpose()?)
    }

    /// The preview version of the original `asset_id`'s current version,
    /// when it has one.
    pub async fn preview(&self, asset_id: Uuid) -> Result<Option<VersionRecord>, DatabaseError> {
        let row = sqlx::query(sqlx::AssertSqlSafe(format!(
            "SELECT pv.id, pv.content_hash, pv.byte_size, pv.media_type, pv.source_id, \
             pv.source_version, to_char(pv.ingested_at AT TIME ZONE 'UTC', \
             'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') \
             FROM public.assets a \
             JOIN public.asset_versions v ON v.asset_id = a.id AND v.id = a.current_version_id \
             {PREVIEW} JOIN public.asset_versions pv ON pv.id = p.output_version_id \
             WHERE a.origin = 'source' AND a.id = $1"
        )))
        .bind(asset_id)
        .fetch_optional(self.pool())
        .await?;
        Ok(row.as_ref().map(|row| version_at(row, 0)).transpose()?)
    }

    /// Find the Asset with a version of `new`'s bytes, or else insert a new
    /// Asset and its first version, in one transaction. The current-version
    /// check runs at commit, so neither new row exists alone. A found Asset
    /// is left as it is; `new.name` is used only for a new one.
    pub(crate) async fn find_or_create_asset(
        &self,
        new: NewVersion<'_>,
    ) -> Result<Ingested, DatabaseError> {
        let mut tx = self.pool().begin().await?;
        // Never inherit a REPEATABLE READ default: its snapshot would be
        // taken as the lock statement starts, before the lock is granted, and
        // the lookup would miss the rows of the upload it waited for.
        sqlx::raw_sql("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .execute(&mut *tx)
            .await?;
        // Uploads of the same bytes commit one at a time. The lock has its
        // own statement, so the lookup's snapshot is taken after it is
        // granted.
        sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
            .bind(ASSET_CONTENT)
            .bind(content_lock_key(new.content_hash))
            .execute(&mut *tx)
            .await?;
        // The oldest matching version of any original, current or not. A
        // preview's bytes never match: uploading them makes a new original.
        let existing: Option<Uuid> = sqlx::query_scalar(
            "SELECT v.asset_id FROM public.asset_versions v \
             JOIN public.assets a ON a.id = v.asset_id AND a.origin = 'source' \
             WHERE v.content_hash = $1 \
             ORDER BY v.ingested_at, v.asset_id LIMIT 1",
        )
        .bind(new.content_hash)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(asset) = existing {
            let row = sqlx::query(sqlx::AssertSqlSafe(format!("{} AND a.id = $1", select())))
                .bind(asset)
                .fetch_one(&mut *tx)
                .await?;
            let found = record(&row)?;
            tx.commit().await?;
            return Ok(Ingested::Found(found));
        }
        let (asset, version) = (Uuid::new_v4(), Uuid::new_v4());
        sqlx::query(
            "INSERT INTO public.assets (id, kind, name, current_version_id, origin) \
             VALUES ($1, $2, $3, $4, 'source')",
        )
        .bind(asset)
        .bind(&new.accepted.proposal.kind)
        .bind(new.name)
        .bind(version)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO public.asset_versions \
             (id, asset_id, content_hash, byte_size, media_type, source_id, source_version, ingested_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, now())",
        )
        .bind(version)
        .bind(asset)
        .bind(new.content_hash)
        .bind(new.byte_size as i64)
        .bind(&new.accepted.proposal.media_type)
        .bind(&new.accepted.provenance.source_id)
        .bind(&new.accepted.provenance.source_version)
        .execute(&mut *tx)
        .await?;
        let row = sqlx::query(sqlx::AssertSqlSafe(format!("{} AND a.id = $1", select())))
            .bind(asset)
            .fetch_one(&mut *tx)
            .await?;
        let created = record(&row)?;
        tx.commit().await?;
        Ok(Ingested::Created(created))
    }
}
