use super::DatabaseError;
use serde_json::Value;
use sqlx::{
    migrate::{Migrate, Migrator},
    postgres::{PgConnectOptions, PgPoolOptions},
    Connection, Executor, PgConnection, PgPool, Row,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::Semaphore, time::timeout};

static MIGRATOR: std::sync::LazyLock<Migrator> = std::sync::LazyLock::new(|| {
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.table_name = std::borrow::Cow::Borrowed("public._sqlx_migrations");
    migrator
});
const INITIAL_TABLE: &str = include_str!("../../schema/migration-table.sql");
const CATALOG: &str = include_str!("../../schema/catalog.sql");
const LOCK: &str = "SELECT pg_advisory_lock(1397310533, 1)";
const UNLOCK: &str = "SELECT pg_advisory_unlock(1397310533, 1)";
const READY_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct Database {
    operational: PgPool,
    readiness: PgPool,
    permit: Arc<Semaphore>,
}

impl Database {
    pub async fn ready(&self) -> bool {
        let Ok(_permit) = self.permit.try_acquire() else {
            return false;
        };
        let result = timeout(READY_TIMEOUT, async {
            let conn = self
                .readiness
                .acquire()
                .await
                .map_err(DatabaseError::from)?;
            // Detach immediately: cancellation always closes the socket instead of
            // returning a connection with outstanding work to the readiness pool.
            let mut conn = conn.detach();
            let result = async {
                conn.execute("BEGIN TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
                    .await?;
                conn.execute("SET LOCAL statement_timeout = '2000ms'")
                    .await?;
                let prefix = preflight(&mut conn, &MIGRATOR, &baselines(), None).await?;
                if prefix != Some(MIGRATOR.iter().count()) {
                    return Err(DatabaseError::History);
                }
                conn.execute("COMMIT").await?;
                Ok::<_, DatabaseError>(())
            }
            .await;
            drop(conn);
            result
        })
        .await;
        matches!(result, Ok(Ok(())))
    }

    pub async fn close(&self) {
        self.readiness.close().await;
        self.operational.close().await;
    }
}

pub async fn initialize(
    options: PgConnectOptions,
    startup: Duration,
    migration: Duration,
) -> Result<Database, DatabaseError> {
    initialize_with(&options, startup, migration, &MIGRATOR, &baselines(), None).await?;
    Ok(Database {
        operational: PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(startup)
            .connect_lazy_with(options.clone()),
        readiness: PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(READY_TIMEOUT)
            .connect_lazy_with(options),
        permit: Arc::new(Semaphore::new(1)),
    })
}

fn baselines() -> Vec<Value> {
    [
        include_str!("../../schema/0.json"),
        include_str!("../../schema/1.json"),
    ]
    .into_iter()
    .map(|value| serde_json::from_str(value).expect("generated schema JSON must be valid"))
    .collect()
}

fn version_policy(version: i32) -> Result<(), DatabaseError> {
    if version / 10000 == 18 {
        Ok(())
    } else {
        Err(DatabaseError::Version)
    }
}

async fn initialize_with(
    options: &PgConnectOptions,
    startup: Duration,
    migration: Duration,
    migrator: &Migrator,
    expected: &[Value],
    version_override: Option<i32>,
) -> Result<(), DatabaseError> {
    if migrator.no_tx
        || migrator
            .iter()
            .any(|m| m.no_tx || m.migration_type.is_down_migration())
    {
        return Err(DatabaseError::Migration);
    }
    let mut conn = timeout(startup, PgConnection::connect_with(options))
        .await
        .map_err(|_| DatabaseError::StartupDeadline)??;
    let mut cancellation = timeout(startup, Cancellation::new(&mut conn, options.clone()))
        .await
        .map_err(|_| DatabaseError::StartupDeadline)??;
    let result = async {
        conn.execute("SET search_path = public, pg_catalog").await?;
        set_deadline(&mut conn, startup).await?;
        timeout(startup, async {
            conn.execute(LOCK).await?;
            conn.lock()
                .await
                .map_err(|_| DatabaseError::StartupDeadline)?;
            Ok::<_, DatabaseError>(())
        })
        .await
        .map_err(|_| DatabaseError::StartupDeadline)??;
        set_deadline(&mut conn, migration).await?;
        let prefix = preflight(&mut conn, migrator, expected, version_override).await?;
        let pending = prefix.unwrap_or(0) < migrator.iter().count();
        privileges(&mut conn, prefix.is_some(), pending).await?;
        if prefix.is_none() {
            timeout(migration, bootstrap(&mut conn, migrator))
                .await
                .map_err(|_| DatabaseError::Migration)??;
        }
        timeout(migration, migrator.run_direct(None, &mut conn, false))
            .await
            .map_err(|_| DatabaseError::Migration)?
            .map_err(migration_error)?;
        if preflight(&mut conn, migrator, expected, version_override).await?
            != Some(migrator.iter().count())
        {
            return Err(DatabaseError::History);
        }
        // run_direct released its own reentrant acquisition; release ours too.
        conn.unlock().await.map_err(migration_error)?;
        conn.execute(UNLOCK).await?;
        Ok::<_, DatabaseError>(())
    }
    .await;
    // Dropping an unpooled connection closes its socket on every failure,
    // including cancellation, releasing session locks without pool reuse.
    if result.is_ok() {
        cancellation.armed = false;
        timeout(startup, conn.close())
            .await
            .map_err(|_| DatabaseError::StartupDeadline)??;
    }
    result
}

async fn set_deadline(conn: &mut PgConnection, duration: Duration) -> Result<(), DatabaseError> {
    let ms = duration.as_millis().clamp(1, i32::MAX as u128).to_string();
    sqlx::query(
        "SELECT set_config('statement_timeout',$1,false), set_config('lock_timeout',$1,false)",
    )
    .bind(ms)
    .execute(conn)
    .await?;
    Ok(())
}

fn migration_error(error: sqlx::migrate::MigrateError) -> DatabaseError {
    match error {
        sqlx::migrate::MigrateError::Execute(error)
        | sqlx::migrate::MigrateError::ExecuteMigration(error, _) => {
            if DatabaseError::from(error) == DatabaseError::Permission {
                DatabaseError::Permission
            } else {
                DatabaseError::Migration
            }
        }
        _ => DatabaseError::Migration,
    }
}

async fn schema(conn: &mut PgConnection) -> Result<Value, DatabaseError> {
    let text: String = sqlx::query_scalar(CATALOG).fetch_one(conn).await?;
    serde_json::from_str(&text).map_err(|_| DatabaseError::Schema)
}

async fn preflight(
    conn: &mut PgConnection,
    migrator: &Migrator,
    expected: &[Value],
    version_override: Option<i32>,
) -> Result<Option<usize>, DatabaseError> {
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
        .fetch_one(&mut *conn)
        .await?;
    version_policy(version_override.unwrap_or(version))?;
    let actual = schema(conn).await?;
    if actual == expected[0] {
        return Ok(None);
    }
    let owned: bool = sqlx::query_scalar("SELECT to_regclass('public.sidereal_metadata') IS NOT NULL AND to_regclass('public._sqlx_migrations') IS NOT NULL").fetch_one(&mut *conn).await?;
    if !owned {
        return Err(DatabaseError::Ownership);
    }
    // Check SELECT privileges before issuing queries that could expose raw errors.
    privileges(conn, true, false).await?;
    let identity: Vec<(String, i32)> =
        sqlx::query_as("SELECT product, format_version FROM public.sidereal_metadata")
            .fetch_all(&mut *conn)
            .await
            .map_err(|_| DatabaseError::Ownership)?;
    if identity != [("sidereal".to_owned(), 1)] {
        return Err(DatabaseError::Ownership);
    }
    let records = sqlx::query("SELECT version, description, success, checksum FROM public._sqlx_migrations ORDER BY version").fetch_all(&mut *conn).await.map_err(|_| DatabaseError::History)?;
    if records.is_empty() || records.len() > migrator.iter().count() {
        return Err(DatabaseError::History);
    }
    for (record, migration) in records.iter().zip(migrator.iter()) {
        if record.try_get::<i64, _>("version")? != migration.version
            || !record.try_get::<bool, _>("success")?
            || record.try_get::<Vec<u8>, _>("checksum")? != migration.checksum.as_ref()
            || record.try_get::<String, _>("description")? != migration.description.as_ref()
        {
            return Err(DatabaseError::History);
        }
    }
    if expected.get(records.len()) != Some(&actual) {
        return Err(DatabaseError::Schema);
    }
    Ok(Some(records.len()))
}

async fn privileges(
    conn: &mut PgConnection,
    owned: bool,
    pending: bool,
) -> Result<(), DatabaseError> {
    let can_use: bool = sqlx::query_scalar("SELECT has_database_privilege(current_database(), 'CONNECT') AND has_schema_privilege('public','USAGE') AND (NOT $1 OR has_schema_privilege('public','CREATE'))").bind(pending).fetch_one(&mut *conn).await?;
    if !can_use {
        return Err(DatabaseError::Permission);
    }
    if owned {
        let can_read: bool = sqlx::query_scalar("SELECT has_table_privilege('public.sidereal_metadata','SELECT') AND has_table_privilege('public._sqlx_migrations','SELECT') AND (NOT $1 OR (has_table_privilege('public._sqlx_migrations','INSERT') AND has_table_privilege('public._sqlx_migrations','UPDATE')))").bind(pending).fetch_one(conn).await?;
        if !can_read {
            return Err(DatabaseError::Permission);
        }
    }
    Ok(())
}

async fn bootstrap(conn: &mut PgConnection, migrator: &Migrator) -> Result<(), DatabaseError> {
    let initial = migrator.iter().next().ok_or(DatabaseError::Migration)?;
    let start = Instant::now();
    let mut tx = conn.begin().await?;
    tx.execute(INITIAL_TABLE)
        .await
        .map_err(|_| DatabaseError::Migration)?;
    tx.execute(initial.sql.clone())
        .await
        .map_err(|_| DatabaseError::Migration)?;
    sqlx::query("INSERT INTO public._sqlx_migrations (version, description, success, checksum, execution_time) VALUES ($1,$2,true,$3,$4)")
        .bind(initial.version).bind(initial.description.as_ref()).bind(initial.checksum.as_ref()).bind(start.elapsed().as_nanos().min(i64::MAX as u128) as i64).execute(&mut *tx).await.map_err(|_| DatabaseError::Migration)?;
    tx.commit().await.map_err(|_| DatabaseError::Migration)?;
    Ok(())
}

#[cfg(test)]
mod tests;

// sqlx has no public PostgreSQL cancellation token. A separate connection may
// terminate only our exact backend (PID plus start time, database, and role).
// The guard also runs when the startup future itself is dropped.
struct Cancellation {
    options: PgConnectOptions,
    pid: i32,
    started: String,
    armed: bool,
}
impl Cancellation {
    async fn new(
        conn: &mut PgConnection,
        options: PgConnectOptions,
    ) -> Result<Self, DatabaseError> {
        let (pid, started): (i32, String) = sqlx::query_as(
            "SELECT pid, backend_start::text FROM pg_stat_activity WHERE pid=pg_backend_pid()",
        )
        .fetch_one(conn)
        .await?;
        Ok(Self {
            options,
            pid,
            started,
            armed: true,
        })
    }
}
impl Drop for Cancellation {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let options = self.options.clone();
        let pid = self.pid;
        let started = self.started.clone();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build cancellation runtime");
            runtime.block_on(async move {
                let _=timeout(Duration::from_secs(2),async {
                    if let Ok(mut conn)=PgConnection::connect_with(&options).await {
                        let _=sqlx::query("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid=$1 AND backend_start::text=$2 AND datname=current_database() AND usename=current_user")
                            .bind(pid).bind(started).execute(&mut conn).await;
                        let _=conn.close().await;
                    }
                }).await;
            });
        });
    }
}
