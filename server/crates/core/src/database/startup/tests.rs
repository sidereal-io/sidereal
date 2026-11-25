use super::*;
use crate::test_support::TestDatabase;
use sqlx::{
    migrate::{Migration, MigrationType},
    AssertSqlSafe, SqlSafeStr,
};

async fn connection(db: &TestDatabase) -> PgConnection {
    PgConnection::connect_with(&db.options).await.unwrap()
}
async fn init(db: &TestDatabase) -> Result<(), DatabaseError> {
    initialize_with(
        &db.options,
        Duration::from_secs(3),
        Duration::from_secs(3),
        &MIGRATOR,
        &baselines(),
        None,
    )
    .await
}
async fn records(conn: &mut PgConnection) -> String {
    sqlx::query_scalar("SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY version),'[]')::text FROM public._sqlx_migrations m").fetch_one(conn).await.unwrap()
}
/// A migrator holding only the first `count` bundled migrations.
fn prefix(count: usize) -> Migrator {
    let mut migrator = Migrator::with_migrations(MIGRATOR.iter().take(count).cloned().collect());
    migrator.dangerous_set_table_name("public._sqlx_migrations");
    migrator
}
fn injected(sql: &'static str, no_tx: bool, later: bool) -> Migrator {
    let mut migrations = Vec::new();
    if later {
        migrations.extend(MIGRATOR.iter().cloned());
    }
    migrations.push(Migration::new(
        migrations.len() as i64 + 1,
        "injected".into(),
        MigrationType::Simple,
        AssertSqlSafe(sql).into_sql_str(),
        no_tx,
    ));
    let mut migrator = Migrator::with_migrations(migrations);
    migrator.dangerous_set_table_name("public._sqlx_migrations");
    migrator
}

#[tokio::test]
async fn schema_baselines_are_generated_and_stable() {
    let db = TestDatabase::new();
    let mut conn = connection(&db).await;
    conn.execute("SET search_path = public, pg_catalog")
        .await
        .unwrap();
    let mut actual = vec![schema(&mut conn).await.unwrap()];
    bootstrap(&mut conn, &MIGRATOR).await.unwrap();
    actual.push(schema(&mut conn).await.unwrap());
    for migration in MIGRATOR.iter().skip(1) {
        MIGRATOR
            .run_direct(Some(migration.version), &mut conn, false)
            .await
            .unwrap();
        actual.push(schema(&mut conn).await.unwrap());
    }
    if std::env::var_os("SIDEREAL_GENERATE_SCHEMA").is_some() {
        for (prefix, value) in actual.iter().enumerate() {
            std::fs::write(
                format!("{}/schema/{prefix}.json", env!("CARGO_MANIFEST_DIR")),
                format!("{}\n", serde_json::to_string_pretty(value).unwrap()),
            )
            .unwrap();
        }
    } else {
        assert_eq!(actual, baselines());
    }
}

#[tokio::test]
async fn upgrade_preserves_the_original_migration_and_metadata() {
    let db = TestDatabase::new();
    initialize_with(
        &db.options,
        Duration::from_secs(3),
        Duration::from_secs(3),
        &prefix(1),
        &baselines(),
        None,
    )
    .await
    .unwrap();
    let mut conn = connection(&db).await;
    let metadata = "SELECT product || ':' || format_version FROM public.sidereal_metadata";
    let before: String = sqlx::query_scalar(metadata)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    let original = records(&mut conn).await;
    init(&db).await.unwrap();
    assert_eq!(
        before,
        sqlx::query_scalar::<_, String>(metadata)
            .fetch_one(&mut conn)
            .await
            .unwrap()
    );
    let first: String = sqlx::query_scalar("SELECT (jsonb_agg(to_jsonb(m) ORDER BY version) -> 0)::text FROM public._sqlx_migrations m").fetch_one(&mut conn).await.unwrap();
    let original: Value = serde_json::from_str(&original).unwrap();
    assert_eq!(original.as_array().unwrap().len(), 1);
    assert_eq!(serde_json::from_str::<Value>(&first).unwrap(), original[0]);
    let unpaired: bool =
        sqlx::query_scalar("SELECT storage_id IS NULL FROM public.sidereal_metadata")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert!(unpaired);
    let migrations: i64 = sqlx::query_scalar("SELECT count(*) FROM public._sqlx_migrations")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(migrations, 6);
}

async fn successful_records(conn: &mut PgConnection) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM public._sqlx_migrations WHERE success")
        .fetch_one(conn)
        .await
        .unwrap()
}

async fn application_tables(conn: &mut PgConnection) -> Vec<String> {
    sqlx::query_scalar("SELECT tablename::text FROM pg_tables WHERE schemaname NOT IN ('pg_catalog', 'information_schema') ORDER BY tablename")
        .fetch_all(conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn first_startup_holds_six_migrations_and_the_facet_tables() {
    let db = TestDatabase::new();
    init(&db).await.unwrap();
    let mut conn = connection(&db).await;
    assert_eq!(successful_records(&mut conn).await, 6);
    assert_eq!(
        application_tables(&mut conn).await,
        [
            "_sqlx_migrations",
            "asset_versions",
            "assets",
            "facet_schemas",
            "facet_values",
            "operation_runs",
            "sessions",
            "sidereal_metadata",
            "users"
        ]
    );
}

#[tokio::test]
async fn concurrent_startup_applies_six_records_once() {
    let db = TestDatabase::new();
    let (first, second) = tokio::join!(init(&db), init(&db));
    first.unwrap();
    second.unwrap();
    let mut conn = connection(&db).await;
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM public._sqlx_migrations")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!((total, successful_records(&mut conn).await), (6, 6));
}

#[tokio::test]
async fn upgrade_adds_empty_access_tables() {
    let db = TestDatabase::new();
    initialize_with(
        &db.options,
        Duration::from_secs(3),
        Duration::from_secs(3),
        &prefix(2),
        &baselines(),
        None,
    )
    .await
    .unwrap();
    let mut conn = connection(&db).await;
    let original: Value = serde_json::from_str(&records(&mut conn).await).unwrap();
    init(&db).await.unwrap();
    let upgraded: Value = serde_json::from_str(&records(&mut conn).await).unwrap();
    assert_eq!(
        upgraded.as_array().unwrap()[..2],
        original.as_array().unwrap()[..]
    );
    assert_eq!(successful_records(&mut conn).await, 6);
    let rows: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM public.users) + (SELECT count(*) FROM public.sessions)",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(rows, 0);
}

const SAM: &str = "INSERT INTO public.users (username, password_hash, role, created_at) VALUES ('sam', 'hash', 'admin', '2026-01-01T00:00:00Z')";
const USERS: &str =
    "SELECT coalesce(jsonb_agg(to_jsonb(u) ORDER BY id), '[]')::text FROM public.users u";
const ASSET_ROWS: &str =
    "SELECT (SELECT count(*) FROM public.assets) + (SELECT count(*) FROM public.asset_versions)";

#[tokio::test]
async fn asset_schema_upgrade_from_three_keeps_a_claimed_user() {
    let db = TestDatabase::new();
    initialize_with(
        &db.options,
        Duration::from_secs(3),
        Duration::from_secs(3),
        &prefix(3),
        &baselines(),
        None,
    )
    .await
    .unwrap();
    let mut conn = connection(&db).await;
    conn.execute(SAM).await.unwrap();
    let users: String = sqlx::query_scalar(USERS)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    let original: Value = serde_json::from_str(&records(&mut conn).await).unwrap();
    init(&db).await.unwrap();
    let upgraded: Value = serde_json::from_str(&records(&mut conn).await).unwrap();
    assert_eq!(
        upgraded.as_array().unwrap()[..3],
        original.as_array().unwrap()[..]
    );
    assert_eq!(successful_records(&mut conn).await, 6);
    let rows: i64 = sqlx::query_scalar(ASSET_ROWS)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows, 0);
    let after: String = sqlx::query_scalar(USERS)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(users, after);
}

const FACET_ROWS: &str = "SELECT (SELECT count(*) FROM public.facet_schemas) + (SELECT count(*) FROM public.facet_values) + (SELECT count(*) FROM public.operation_runs)";
const ASSET_AND_VERSIONS: &str = "SELECT (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]') FROM public.assets a) || (SELECT coalesce(jsonb_agg(to_jsonb(v) ORDER BY id), '[]') FROM public.asset_versions v)";

#[tokio::test]
async fn facet_schema_upgrade_from_four_keeps_a_user_and_an_asset() {
    let db = TestDatabase::new();
    initialize_with(
        &db.options,
        Duration::from_secs(3),
        Duration::from_secs(3),
        &prefix(4),
        &baselines(),
        None,
    )
    .await
    .unwrap();
    let mut conn = connection(&db).await;
    conn.execute(SAM).await.unwrap();
    insert_pair(&mut conn, ASSET_A, VERSION_A, VERSION_A, ASSET_A)
        .await
        .unwrap();
    let users: String = sqlx::query_scalar(USERS)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    let assets: Value = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT ({ASSET_AND_VERSIONS})::text"
    )))
    .fetch_one(&mut conn)
    .await
    .map(|text: String| serde_json::from_str(&text).unwrap())
    .unwrap();
    let original: Value = serde_json::from_str(&records(&mut conn).await).unwrap();
    init(&db).await.unwrap();
    let upgraded: Value = serde_json::from_str(&records(&mut conn).await).unwrap();
    assert_eq!(
        upgraded.as_array().unwrap()[..4],
        original.as_array().unwrap()[..]
    );
    assert_eq!(successful_records(&mut conn).await, 6);
    let rows: i64 = sqlx::query_scalar(FACET_ROWS)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows, 0);
    let after: String = sqlx::query_scalar(USERS)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(users, after);
    let assets_after: Value = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT ({ASSET_AND_VERSIONS})::text"
    )))
    .fetch_one(&mut conn)
    .await
    .map(|text: String| serde_json::from_str(&text).unwrap())
    .unwrap();
    assert_eq!(assets.as_array().unwrap().len(), 2);
    assert_eq!(assets, assets_after);
}

/// A succeeded run over `VERSION_A` by `sam`, the `image.file-size` schema
/// row it needed, and its value, as the file-size Operator stores them.
const FILE_SIZE_FACT: &str = "\
    INSERT INTO public.operation_runs (id, operator_id, operator_version, asset_version_id, actor_user_id, status, started_at, ended_at) \
        SELECT '00000000-0000-4000-8000-0000000000c1', 'image.measure-file-size', '0.1.0', '00000000-0000-4000-8000-0000000000a1', id, 'succeeded', now(), now() FROM public.users WHERE username = 'sam'; \
    INSERT INTO public.facet_schemas (name, version, owner_pack, label, value_type, unit, scope) \
        VALUES ('image.file-size', 1, 'image', 'File size', 'count', 'bytes', 'asset_version'); \
    INSERT INTO public.facet_values (id, facet, schema_version, asset_version_id, value, producer_id, producer_version, run_id, written_at) \
        VALUES ('00000000-0000-4000-8000-0000000000d1', 'image.file-size', 1, '00000000-0000-4000-8000-0000000000a1', '4', 'image.measure-file-size', '0.1.0', '00000000-0000-4000-8000-0000000000c1', now())";
const EVERY_ROW: &str = "SELECT jsonb_build_array(\
    (SELECT coalesce(jsonb_agg(to_jsonb(u) ORDER BY id), '[]') FROM public.users u), \
    (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY id), '[]') FROM public.assets a), \
    (SELECT coalesce(jsonb_agg(to_jsonb(v) ORDER BY id), '[]') FROM public.asset_versions v), \
    (SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY id), '[]') FROM public.operation_runs r), \
    (SELECT coalesce(jsonb_agg(to_jsonb(s) - 'allowed_values' ORDER BY name, version), '[]') FROM public.facet_schemas s), \
    (SELECT coalesce(jsonb_agg(to_jsonb(f) ORDER BY id), '[]') FROM public.facet_values f))::text";

#[tokio::test]
async fn allowed_values_upgrade_from_five_keeps_every_row() {
    let db = TestDatabase::new();
    initialize_with(
        &db.options,
        Duration::from_secs(3),
        Duration::from_secs(3),
        &prefix(5),
        &baselines(),
        None,
    )
    .await
    .unwrap();
    let mut conn = connection(&db).await;
    conn.execute(SAM).await.unwrap();
    insert_pair(&mut conn, ASSET_A, VERSION_A, VERSION_A, ASSET_A)
        .await
        .unwrap();
    conn.execute(FILE_SIZE_FACT).await.unwrap();
    let rows: String = sqlx::query_scalar(EVERY_ROW)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    let original: Value = serde_json::from_str(&records(&mut conn).await).unwrap();
    init(&db).await.unwrap();
    let upgraded: Value = serde_json::from_str(&records(&mut conn).await).unwrap();
    assert_eq!(
        upgraded.as_array().unwrap()[..5],
        original.as_array().unwrap()[..]
    );
    assert_eq!(successful_records(&mut conn).await, 6);
    let after: String = sqlx::query_scalar(EVERY_ROW)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows, after);
    let allowed: Option<Vec<String>> =
        sqlx::query_scalar("SELECT allowed_values FROM public.facet_schemas")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!(allowed, None);
}

/// Insert one `facet_schemas` row of `value_type` with `allowed`, a SQL
/// expression, into a fully migrated database.
async fn insert_schema(value_type: &str, allowed: &str) -> Result<(), sqlx::Error> {
    let db = TestDatabase::new();
    init(&db).await.unwrap();
    let mut conn = connection(&db).await;
    conn.execute(AssertSqlSafe(format!(
        "INSERT INTO public.facet_schemas \
         (name, version, owner_pack, label, value_type, unit, scope, allowed_values) \
         VALUES ('test.kind', 1, 'test', 'Kind', '{value_type}', '', 'asset_version', {allowed})"
    )))
    .await
    .map(|_| ())
}

/// PostgreSQL refused the row with a check violation.
fn assert_check_violation(outcome: Result<(), sqlx::Error>) {
    let error = outcome.unwrap_err();
    let code = error.as_database_error().and_then(|e| e.code());
    assert_eq!(code.as_deref(), Some("23514"), "{error}");
}

#[tokio::test]
async fn allowed_values_schema_refuses_a_choice_with_no_allowed_values() {
    assert_check_violation(insert_schema("choice", "NULL").await);
}

#[tokio::test]
async fn allowed_values_schema_refuses_a_choice_with_an_empty_list() {
    assert_check_violation(insert_schema("choice", "ARRAY[]::text[]").await);
}

#[tokio::test]
async fn allowed_values_schema_refuses_a_count_with_allowed_values() {
    assert_check_violation(insert_schema("count", "ARRAY['a']").await);
}

#[tokio::test]
async fn allowed_values_schema_keeps_a_choice_with_allowed_values() {
    insert_schema("choice", "ARRAY['PNG', 'JPEG']")
        .await
        .unwrap();
}

const ASSET_A: &str = "00000000-0000-4000-8000-00000000000a";
const ASSET_B: &str = "00000000-0000-4000-8000-00000000000b";
const VERSION_A: &str = "00000000-0000-4000-8000-0000000000a1";
const VERSION_B: &str = "00000000-0000-4000-8000-0000000000b1";
const HASH: &str = "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262";

/// Insert one Asset and one version in a transaction, as ingest does.
async fn insert_pair(
    conn: &mut PgConnection,
    asset: &str,
    current: &str,
    version: &str,
    owner: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = conn.begin().await?;
    sqlx::query("INSERT INTO public.assets (id, kind, name, current_version_id) VALUES ($1::uuid, 'image', 'nebula.png', $2::uuid)")
        .bind(asset)
        .bind(current)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO public.asset_versions (id, asset_id, content_hash, byte_size, media_type, source_id, source_version, ingested_at) VALUES ($1::uuid, $2::uuid, $3, 4, 'image/png', 'image.manual-upload', '0.1.0', now())")
        .bind(version)
        .bind(owner)
        .bind(HASH)
        .execute(&mut *tx)
        .await?;
    tx.commit().await
}

#[tokio::test]
async fn asset_schema_accepts_an_asset_whose_current_version_is_its_own() {
    let db = TestDatabase::new();
    init(&db).await.unwrap();
    let mut conn = connection(&db).await;
    insert_pair(&mut conn, ASSET_A, VERSION_A, VERSION_A, ASSET_A)
        .await
        .unwrap();
    let revision: i64 = sqlx::query_scalar("SELECT revision FROM public.assets")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(revision, 0);
}

#[tokio::test]
async fn asset_schema_refuses_a_current_version_that_does_not_exist() {
    let db = TestDatabase::new();
    init(&db).await.unwrap();
    let mut conn = connection(&db).await;
    assert!(
        insert_pair(&mut conn, ASSET_A, VERSION_B, VERSION_A, ASSET_A)
            .await
            .is_err()
    );
    let rows: i64 = sqlx::query_scalar(ASSET_ROWS)
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(rows, 0);
}

#[tokio::test]
async fn asset_schema_refuses_a_current_version_of_another_asset() {
    let db = TestDatabase::new();
    init(&db).await.unwrap();
    let mut conn = connection(&db).await;
    insert_pair(&mut conn, ASSET_A, VERSION_A, VERSION_A, ASSET_A)
        .await
        .unwrap();
    let mut tx = conn.begin().await.unwrap();
    sqlx::query("INSERT INTO public.assets (id, kind, name, current_version_id) VALUES ($1::uuid, 'image', 'other.png', $2::uuid)")
        .bind(ASSET_B)
        .bind(VERSION_A)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(tx.commit().await.is_err());
    let assets: i64 = sqlx::query_scalar("SELECT count(*) FROM public.assets")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(assets, 1);
}

#[tokio::test]
async fn asset_schema_lets_two_versions_share_one_hash() {
    let db = TestDatabase::new();
    init(&db).await.unwrap();
    let mut conn = connection(&db).await;
    insert_pair(&mut conn, ASSET_A, VERSION_A, VERSION_A, ASSET_A)
        .await
        .unwrap();
    insert_pair(&mut conn, ASSET_B, VERSION_B, VERSION_B, ASSET_B)
        .await
        .unwrap();
}

#[tokio::test]
async fn repeated_and_concurrent_startup_preserve_every_record_field() {
    let db = TestDatabase::new();
    let (first, second) = tokio::join!(init(&db), init(&db));
    first.unwrap();
    second.unwrap();
    let mut conn = connection(&db).await;
    let before = records(&mut conn).await;
    init(&db).await.unwrap();
    assert_eq!(before, records(&mut conn).await);
    // sqlx's timing sentinel is explicitly accepted and never repaired.
    conn.execute("UPDATE public._sqlx_migrations SET execution_time = -1")
        .await
        .unwrap();
    let sentinel = records(&mut conn).await;
    init(&db).await.unwrap();
    assert_eq!(sentinel, records(&mut conn).await);
}

#[tokio::test]
async fn unrelated_objects_and_versions_are_rejected_without_writes() {
    for object in ["CREATE TABLE public.foreign_data (value text); INSERT INTO public.foreign_data VALUES ('keep')", "CREATE VIEW public.foreign_view AS SELECT 1", "CREATE FUNCTION public.foreign_function() RETURNS int LANGUAGE SQL AS 'SELECT 1'", "CREATE EXTENSION hstore", "CREATE SCHEMA foreign_schema", "CREATE TYPE public.foreign_enum AS ENUM ('keep')", "CREATE SEQUENCE public.foreign_sequence"] {
        let db = TestDatabase::new(); let mut conn = connection(&db).await;
        conn.execute(AssertSqlSafe(object)).await.unwrap();
        let before = schema(&mut conn).await.unwrap();
        assert!(init(&db).await.is_err());
        assert_eq!(before, schema(&mut conn).await.unwrap());
        if object.starts_with("CREATE TABLE") {
            assert_eq!(sqlx::query_scalar::<_,String>("SELECT value FROM public.foreign_data").fetch_one(&mut conn).await.unwrap(), "keep");
        }
    }
    for version in [170000, 190000] {
        assert_eq!(version_policy(version), Err(DatabaseError::Version));
        let db = TestDatabase::new();
        let mut conn = connection(&db).await;
        let before = schema(&mut conn).await.unwrap();
        assert_eq!(
            initialize_with(
                &db.options,
                Duration::from_secs(2),
                Duration::from_secs(2),
                &MIGRATOR,
                &baselines(),
                Some(version)
            )
            .await,
            Err(DatabaseError::Version)
        );
        assert_eq!(before, schema(&mut conn).await.unwrap());
    }
    assert!(version_policy(180006).is_ok());
}

#[tokio::test]
async fn damaged_history_identity_and_schema_do_not_change() {
    for damage in ["UPDATE public._sqlx_migrations SET version=99 WHERE version=2", "UPDATE public._sqlx_migrations SET checksum='bad'::bytea", "DELETE FROM public._sqlx_migrations", "UPDATE public._sqlx_migrations SET success=false", "DELETE FROM public.sidereal_metadata", "ALTER TABLE public.sidereal_metadata ADD COLUMN unexpected text", "ALTER TABLE public.sidereal_metadata DROP CONSTRAINT sidereal_metadata_format_version_check; UPDATE public.sidereal_metadata SET format_version=2"] {
        let db=TestDatabase::new(); init(&db).await.unwrap(); let mut conn=connection(&db).await;
        conn.execute(AssertSqlSafe(damage)).await.unwrap();
        let before=schema(&mut conn).await.unwrap(); let history=records(&mut conn).await;
        let identity: String=sqlx::query_scalar("SELECT coalesce(jsonb_agg(to_jsonb(m)),'[]')::text FROM public.sidereal_metadata m").fetch_one(&mut conn).await.unwrap();
        assert!(init(&db).await.is_err());
        assert_eq!(before,schema(&mut conn).await.unwrap()); assert_eq!(history,records(&mut conn).await);
        let after: String=sqlx::query_scalar("SELECT coalesce(jsonb_agg(to_jsonb(m)),'[]')::text FROM public.sidereal_metadata m").fetch_one(&mut conn).await.unwrap();
        assert_eq!(identity,after);
    }
    let db = TestDatabase::new();
    let mut conn = connection(&db).await;
    conn.execute(INITIAL_TABLE).await.unwrap();
    let before = schema(&mut conn).await.unwrap();
    assert_eq!(init(&db).await, Err(DatabaseError::Ownership));
    assert_eq!(before, schema(&mut conn).await.unwrap());
}

#[tokio::test]
async fn failed_bootstrap_and_later_migrations_roll_back() {
    for later in [false, true] {
        let db = TestDatabase::new();
        if later {
            init(&db).await.unwrap();
        }
        let mut conn = connection(&db).await;
        let before = schema(&mut conn).await.unwrap();
        let history = if later {
            Some(records(&mut conn).await)
        } else {
            None
        };
        let migrator = injected(
            "CREATE TABLE public.rollback_probe(value int); SELECT 1/0",
            false,
            later,
        );
        assert!(initialize_with(
            &db.options,
            Duration::from_secs(2),
            Duration::from_secs(2),
            &migrator,
            &baselines(),
            None
        )
        .await
        .is_err());
        assert_eq!(before, schema(&mut conn).await.unwrap());
        if let Some(history) = history {
            assert_eq!(history, records(&mut conn).await);
        }
        // Failure closed the dedicated connection and released both locks.
        init(&db).await.unwrap();
    }
    let db = TestDatabase::new();
    let migrator = injected(
        "CREATE TABLE public.should_not_exist(value int)",
        true,
        false,
    );
    assert_eq!(
        initialize_with(
            &db.options,
            Duration::from_secs(2),
            Duration::from_secs(2),
            &migrator,
            &baselines(),
            None
        )
        .await,
        Err(DatabaseError::Migration)
    );
    let mut conn = connection(&db).await;
    assert_eq!(schema(&mut conn).await.unwrap(), baselines()[0]);
}

#[tokio::test]
async fn lock_and_migration_deadlines_release_connections() {
    let db = TestDatabase::new();
    let mut blocker = connection(&db).await;
    blocker.execute(LOCK).await.unwrap();
    let start = Instant::now();
    assert!(initialize_with(
        &db.options,
        Duration::from_millis(100),
        Duration::from_secs(1),
        &MIGRATOR,
        &baselines(),
        None
    )
    .await
    .is_err());
    assert!(start.elapsed() < Duration::from_millis(350));
    blocker.execute(UNLOCK).await.unwrap();
    let migrator = injected("SELECT pg_sleep(2)", false, false);
    let start = Instant::now();
    assert!(initialize_with(
        &db.options,
        Duration::from_secs(1),
        Duration::from_millis(100),
        &migrator,
        &baselines(),
        None
    )
    .await
    .is_err());
    assert!(start.elapsed() < Duration::from_millis(350));
    init(&db).await.unwrap();
}

#[tokio::test]
async fn readiness_is_bounded_consistent_and_read_only() {
    let db = TestDatabase::new();
    let database = initialize(
        db.options.clone(),
        Duration::from_secs(2),
        Duration::from_secs(2),
    )
    .await
    .unwrap();
    assert!(database.ready().await);
    let permit = database.permit.acquire().await.unwrap();
    let start = Instant::now();
    assert!(database.ready().await);
    assert!(start.elapsed() < Duration::from_millis(100));
    drop(permit);
    let mut conn = connection(&db).await;
    let history = records(&mut conn).await;
    conn.execute("UPDATE public._sqlx_migrations SET version=99 WHERE version=2")
        .await
        .unwrap();
    assert!(!database.ready().await);
    conn.execute("UPDATE public._sqlx_migrations SET version=2 WHERE version=99")
        .await
        .unwrap();
    assert!(database.ready().await);
    assert_eq!(history, records(&mut conn).await);
    let mut tx = conn.begin().await.unwrap();
    tx.execute("LOCK TABLE public._sqlx_migrations IN ACCESS EXCLUSIVE MODE")
        .await
        .unwrap();
    let start = Instant::now();
    assert!(!database.ready().await);
    assert!(start.elapsed() < Duration::from_millis(2250));
    tx.rollback().await.unwrap();
    let start = Instant::now();
    assert!(database.ready().await);
    assert!(start.elapsed() < Duration::from_secs(5));
    database.close().await;
}

#[tokio::test]
async fn different_owners_and_permissions_are_checked_separately() {
    let db = TestDatabase::new();
    init(&db).await.unwrap();
    let mut conn = connection(&db).await;
    let baseline = schema(&mut conn).await.unwrap();
    let history = records(&mut conn).await;
    conn.execute("ALTER TABLE public.sidereal_metadata OWNER TO pg_database_owner; ALTER TABLE public._sqlx_migrations OWNER TO pg_database_owner").await.unwrap();
    assert_eq!(baseline, schema(&mut conn).await.unwrap());
    init(&db).await.unwrap();
    assert_eq!(history, records(&mut conn).await);
    conn.execute("REVOKE SELECT ON public._sqlx_migrations FROM pg_database_owner")
        .await
        .unwrap();
    assert_eq!(init(&db).await, Err(DatabaseError::Permission));
    assert_eq!(baseline, schema(&mut conn).await.unwrap());
    conn.execute("GRANT SELECT ON public._sqlx_migrations TO pg_database_owner")
        .await
        .unwrap();
    assert_eq!(history, records(&mut conn).await);
}

#[tokio::test]
async fn pending_migrations_require_write_privileges_before_writes() {
    for privilege in ["INSERT", "UPDATE"] {
        let db = TestDatabase::new();
        init(&db).await.unwrap();
        let mut conn = connection(&db).await;
        let migrator = injected(
            "CREATE TABLE public.permission_probe(value int)",
            false,
            true,
        );
        let mut tx = conn.begin().await.unwrap();
        tx.execute("CREATE TABLE public.permission_probe(value int)")
            .await
            .unwrap();
        let next_schema = schema(&mut tx).await.unwrap();
        tx.rollback().await.unwrap();
        let mut expected = baselines();
        expected.push(next_schema);

        // Transfer ownership so the test role's original owner grants can no
        // longer mask a missing grant on the database-owner role.
        conn.execute("ALTER TABLE public._sqlx_migrations OWNER TO pg_database_owner")
            .await
            .unwrap();
        conn.execute(AssertSqlSafe(format!(
            "REVOKE {privilege} ON public._sqlx_migrations FROM pg_database_owner"
        )))
        .await
        .unwrap();
        let before = schema(&mut conn).await.unwrap();
        let history = records(&mut conn).await;
        // The current migration prefix is readable and needs no write grants.
        init(&db).await.unwrap();
        assert_eq!(
            initialize_with(
                &db.options,
                Duration::from_secs(2),
                Duration::from_secs(2),
                &migrator,
                &expected,
                None
            )
            .await,
            Err(DatabaseError::Permission),
            "pending migration must require {privilege}"
        );
        assert_eq!(before, schema(&mut conn).await.unwrap());
        assert_eq!(history, records(&mut conn).await);

        // The same pending migration succeeds once its required grant returns.
        conn.execute(AssertSqlSafe(format!(
            "GRANT {privilege} ON public._sqlx_migrations TO pg_database_owner"
        )))
        .await
        .unwrap();
        initialize_with(
            &db.options,
            Duration::from_secs(2),
            Duration::from_secs(2),
            &migrator,
            &expected,
            None,
        )
        .await
        .unwrap();
        assert_eq!(&schema(&mut conn).await.unwrap(), expected.last().unwrap());
    }
}

#[tokio::test]
async fn later_migrations_and_exact_prefix_validation() {
    let db = TestDatabase::new();
    init(&db).await.unwrap();
    let mut conn = connection(&db).await;
    let migrator = injected("CREATE TABLE public.later_probe(value int)", false, true);
    // Generate the injected prefix's expected logical schema using PostgreSQL,
    // then roll it back so startup must apply the pending migration itself.
    let mut tx = conn.begin().await.unwrap();
    tx.execute("CREATE TABLE public.later_probe(value int)")
        .await
        .unwrap();
    let next_schema = schema(&mut tx).await.unwrap();
    tx.rollback().await.unwrap();
    let mut expected = baselines();
    expected.push(next_schema);
    initialize_with(
        &db.options,
        Duration::from_secs(2),
        Duration::from_secs(2),
        &migrator,
        &expected,
        None,
    )
    .await
    .unwrap();
    let history = records(&mut conn).await;
    initialize_with(
        &db.options,
        Duration::from_secs(2),
        Duration::from_secs(2),
        &migrator,
        &expected,
        None,
    )
    .await
    .unwrap();
    assert_eq!(history, records(&mut conn).await);
    conn.execute("DELETE FROM public._sqlx_migrations WHERE version=1")
        .await
        .unwrap();
    let history = records(&mut conn).await;
    let before = schema(&mut conn).await.unwrap();
    assert_eq!(
        initialize_with(
            &db.options,
            Duration::from_secs(2),
            Duration::from_secs(2),
            &migrator,
            &expected,
            None
        )
        .await,
        Err(DatabaseError::History)
    );
    assert_eq!(history, records(&mut conn).await);
    assert_eq!(before, schema(&mut conn).await.unwrap());
}

#[tokio::test]
async fn cancelling_startup_releases_session_locks() {
    let db = TestDatabase::new();
    let migrator = injected("SELECT pg_sleep(10)", false, false);
    assert!(tokio::time::timeout(
        Duration::from_millis(150),
        initialize_with(
            &db.options,
            Duration::from_secs(2),
            Duration::from_secs(20),
            &migrator,
            &baselines(),
            None
        )
    )
    .await
    .is_err());
    init(&db).await.unwrap();
}

#[tokio::test]
async fn verified_tls_rejects_untrusted_and_wrong_host_certificates_without_writes() {
    use sqlx::postgres::PgSslMode;
    let db = TestDatabase::new();
    let mut conn = connection(&db).await;
    let before = schema(&mut conn).await.unwrap();
    let fixture = format!(
        "{}/../../../.workspace/db-fixture-tls",
        env!("CARGO_MANIFEST_DIR")
    );
    for options in [
        db.options
            .clone()
            .host("localhost")
            .ssl_mode(PgSslMode::VerifyFull)
            .ssl_root_cert(format!("{fixture}/wrong-ca.crt")),
        db.options
            .clone()
            .host("127.0.0.1")
            .ssl_mode(PgSslMode::VerifyFull)
            .ssl_root_cert(format!("{fixture}/ca.crt")),
    ] {
        assert!(initialize_with(
            &options,
            Duration::from_secs(2),
            Duration::from_secs(2),
            &MIGRATOR,
            &baselines(),
            None
        )
        .await
        .is_err());
        assert_eq!(before, schema(&mut conn).await.unwrap());
    }
    let valid = db
        .options
        .clone()
        .host("localhost")
        .ssl_mode(PgSslMode::VerifyFull)
        .ssl_root_cert(format!("{fixture}/ca.crt"));
    initialize_with(
        &valid,
        Duration::from_secs(2),
        Duration::from_secs(2),
        &MIGRATOR,
        &baselines(),
        None,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn unresponsive_connection_deadline_closes_socket() {
    use tokio::io::AsyncReadExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = [0; 1024];
        loop {
            if socket.read(&mut bytes).await.unwrap() == 0 {
                break;
            }
        }
    });
    let options = PgConnectOptions::new()
        .host("127.0.0.1")
        .port(port)
        .ssl_mode(sqlx::postgres::PgSslMode::Disable);
    let start = Instant::now();
    assert_eq!(
        initialize_with(
            &options,
            Duration::from_millis(100),
            Duration::from_secs(1),
            &MIGRATOR,
            &baselines(),
            None
        )
        .await,
        Err(DatabaseError::Unreachable)
    );
    assert!(start.elapsed() < Duration::from_millis(350));
    timeout(Duration::from_millis(250), peer)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn silent_relay_reports_unreachable_within_startup_deadline() {
    use crate::test_support::{Relay, RelayMode};
    let relay = Relay::new("127.0.0.1:9".parse().unwrap()).await;
    relay.set_mode(RelayMode::Silent);
    let options = PgConnectOptions::new()
        .host("127.0.0.1")
        .port(relay.port)
        .ssl_mode(sqlx::postgres::PgSslMode::Disable);
    let start = Instant::now();
    let error = initialize(options, Duration::from_secs(1), Duration::from_secs(1))
        .await
        .err()
        .unwrap();
    assert_eq!(error, DatabaseError::Unreachable);
    assert!(error.to_string().starts_with("database unreachable"));
    assert!(start.elapsed() < Duration::from_millis(1250));
}

#[tokio::test]
async fn open_reports_a_silent_host_as_unreachable_like_the_server() {
    use crate::test_support::{Relay, RelayMode};
    let relay = Relay::new("127.0.0.1:9".parse().unwrap()).await;
    relay.set_mode(RelayMode::Silent);
    let options = PgConnectOptions::new()
        .host("127.0.0.1")
        .port(relay.port)
        .ssl_mode(sqlx::postgres::PgSslMode::Disable);
    let start = Instant::now();
    let error = Database::open(options, Duration::from_secs(1))
        .await
        .err()
        .unwrap();
    assert_eq!(error, DatabaseError::Unreachable);
    assert!(start.elapsed() < Duration::from_millis(1250));
}

async fn open(db: &TestDatabase) -> Result<Database, DatabaseError> {
    Database::open(db.options.clone(), Duration::from_secs(3)).await
}

#[tokio::test]
async fn open_returns_a_handle_for_a_current_schema() {
    let db = TestDatabase::new();
    init(&db).await.unwrap();
    let database = open(&db).await.unwrap();
    assert!(database.ready().await);
}

#[tokio::test]
async fn open_answers_pending_for_an_empty_database_without_creating_a_table() {
    let db = TestDatabase::new();
    assert_eq!(open(&db).await.err(), Some(DatabaseError::Pending));
    assert!(application_tables(&mut connection(&db).await)
        .await
        .is_empty());
}

#[tokio::test]
async fn open_answers_pending_for_a_database_behind_without_migrating() {
    let db = TestDatabase::new();
    initialize_with(
        &db.options,
        Duration::from_secs(3),
        Duration::from_secs(3),
        &prefix(2),
        &baselines(),
        None,
    )
    .await
    .unwrap();
    let mut conn = connection(&db).await;
    let before = records(&mut conn).await;
    assert_eq!(open(&db).await.err(), Some(DatabaseError::Pending));
    assert_eq!(before, records(&mut conn).await);
}

#[tokio::test]
async fn open_passes_other_preflight_errors_through() {
    let db = TestDatabase::new();
    let mut conn = connection(&db).await;
    conn.execute("CREATE TABLE public.foreign_data (value text)")
        .await
        .unwrap();
    let before = schema(&mut conn).await.unwrap();
    assert_eq!(open(&db).await.err(), Some(DatabaseError::Ownership));
    assert_eq!(before, schema(&mut conn).await.unwrap());
}

mod startup_cleanup {
    use super::*;
    use crate::storage::{tests::TempRoot, Entry, Problem};

    const SECONDS: Duration = Duration::from_secs(3);

    /// Pair `temp` with `db`, as a first start does.
    async fn paired(db: &TestDatabase, temp: &TempRoot) {
        let (database, _, _) =
            initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &temp.root())
                .await
                .unwrap();
        database.close().await;
    }

    /// A paired root whose `tmp` holds two leftover files.
    async fn leftovers(db: &TestDatabase) -> TempRoot {
        let temp = TempRoot::new();
        paired(db, &temp).await;
        std::fs::create_dir(temp.path.join("tmp")).unwrap();
        for name in ["a", "b"] {
            std::fs::write(temp.path.join("tmp").join(name), b"left").unwrap();
        }
        temp
    }

    async fn stored_id(db: &TestDatabase) -> Option<uuid::Uuid> {
        let mut conn = connection(db).await;
        sqlx::query_scalar("SELECT storage_id FROM public.sidereal_metadata")
            .fetch_one(&mut conn)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn startup_removes_leftover_staging_files() {
        let db = TestDatabase::new();
        let temp = leftovers(&db).await;
        paired(&db, &temp).await;
        assert_eq!(std::fs::read_dir(temp.path.join("tmp")).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn startup_answers_what_cleanup_removed() {
        let db = TestDatabase::new();
        let temp = leftovers(&db).await;
        let (database, _, cleanup) =
            initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &temp.root())
                .await
                .unwrap();
        database.close().await;
        assert_eq!(cleanup.removed, 2);
    }

    #[tokio::test]
    async fn an_unsafe_tmp_fails_startup_with_a_storage_error() {
        let db = TestDatabase::new();
        let temp = TempRoot::new();
        paired(&db, &temp).await;
        let outside = TempRoot::new();
        std::os::unix::fs::symlink(&outside.path, temp.path.join("tmp")).unwrap();
        let outcome =
            initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &temp.root()).await;
        match outcome {
            Err(StartupError::Storage(error)) => {
                assert_eq!(error.problem(), &Problem::Unsafe(Entry::Tmp))
            }
            Err(other) => panic!("{other}"),
            Ok(_) => panic!("startup went on with an unsafe tmp"),
        }
    }

    #[tokio::test]
    async fn an_unsafe_tmp_leaves_the_database_uuid_unchanged() {
        let db = TestDatabase::new();
        let temp = TempRoot::new();
        paired(&db, &temp).await;
        let before = stored_id(&db).await;
        std::fs::write(temp.path.join("tmp"), b"not a folder").unwrap();
        assert!(
            initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &temp.root())
                .await
                .is_err()
        );
        assert_eq!(stored_id(&db).await, before);
    }
}
