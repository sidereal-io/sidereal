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
async fn schema_baselines_are_generated_and_stable() {
    let db = TestDatabase::new();
    let mut conn = connection(&db).await;
    conn.execute("SET search_path = public, pg_catalog")
        .await
        .unwrap();
    let mut actual = vec![schema(&mut conn).await.unwrap()];
    bootstrap(&mut conn, &MIGRATOR).await.unwrap();
    actual.push(schema(&mut conn).await.unwrap());
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
    assert!(!database.ready().await);
    assert!(start.elapsed() < Duration::from_millis(100));
    drop(permit);
    let mut conn = connection(&db).await;
    let history = records(&mut conn).await;
    conn.execute("UPDATE public._sqlx_migrations SET version=99")
        .await
        .unwrap();
    assert!(!database.ready().await);
    conn.execute("UPDATE public._sqlx_migrations SET version=1")
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
        Err(DatabaseError::StartupDeadline)
    );
    assert!(start.elapsed() < Duration::from_millis(350));
    timeout(Duration::from_millis(250), peer)
        .await
        .unwrap()
        .unwrap();
}
