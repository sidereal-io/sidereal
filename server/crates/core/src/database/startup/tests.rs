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
fn injected(sql: &'static str, no_tx: bool, later: bool) -> Migrator {
    let mut migrations = Vec::new();
    if later {
        migrations.push(MIGRATOR.iter().next().unwrap().clone());
    }
    migrations.push(Migration::new(
        if later { 2 } else { 1 },
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
async fn verified_tls_rejects_untrusted_and_wrong_host_certificates_without_writes() {
    use sqlx::postgres::PgSslMode;
    let db = TestDatabase::new();
    let mut conn = connection(&db).await;
    let before = schema(&mut conn).await.unwrap();
    let fixture = format!("{}/../../tests/fixtures/tls", env!("CARGO_MANIFEST_DIR"));
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
    for damage in ["UPDATE public._sqlx_migrations SET version=99", "UPDATE public._sqlx_migrations SET checksum='bad'::bytea", "DELETE FROM public._sqlx_migrations", "UPDATE public._sqlx_migrations SET success=false", "DELETE FROM public.sidereal_metadata", "ALTER TABLE public.sidereal_metadata ADD COLUMN unexpected text", "ALTER TABLE public.sidereal_metadata DROP CONSTRAINT sidereal_metadata_format_version_check; UPDATE public.sidereal_metadata SET format_version=2"] {
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
