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
