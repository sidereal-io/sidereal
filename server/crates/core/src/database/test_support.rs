//! Database lifetime is supervised outside the test runtime, including unwinds.
use sqlx::{postgres::PgConnectOptions, AssertSqlSafe, Connection, PgConnection};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
static CLEANUP: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(super) struct TestDatabase {
    pub options: PgConnectOptions,
    cleanup: Option<mpsc::Sender<()>>,
    supervisor: Option<thread::JoinHandle<()>>,
}

impl TestDatabase {
    pub fn new() -> Self {
        let url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL is required; run just db-up, then export TEST_DATABASE_URL=$(just db-test-url)");
        let options: PgConnectOptions = url
            .parse()
            .unwrap_or_else(|_| panic!("TEST_DATABASE_URL must be a PostgreSQL test-role URL"));
        let name = format!(
            "sidereal_test_{}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let database_options = options.clone().database(&name);
        let (ready_tx, ready_rx) = mpsc::channel();
        let (cleanup, cleanup_rx) = mpsc::channel();
        let supervisor = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let creation = CLEANUP.lock().unwrap_or_else(|error| error.into_inner());
            let connection = runtime.block_on(async {
                let mut conn = tokio::time::timeout(
                    Duration::from_secs(5),
                    PgConnection::connect_with(&options),
                )
                .await
                .map_err(|_| ())?
                .map_err(|_| ())?;
                sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
                    .execute(&mut conn)
                    .await
                    .map_err(|_| ())?;
                Ok::<_, ()>(conn)
            });
            drop(creation);
            let Ok(mut conn) = connection else {
                let _ = ready_tx.send(false);
                return;
            };
            if ready_tx.send(true).is_ok() {
                let _ = cleanup_rx.recv();
            }
            let _cleanup = CLEANUP.lock().unwrap_or_else(|error| error.into_inner());
            runtime.block_on(async {
                sqlx::query(AssertSqlSafe(format!("DROP DATABASE {name} WITH (FORCE)")))
                    .execute(&mut conn)
                    .await
                    .expect("test database cleanup failed; use fixture-only orphan cleanup");
                conn.close().await.unwrap();
            });
        });
        assert!(ready_rx.recv().unwrap_or(false), "test database unavailable; run just db-up and export TEST_DATABASE_URL=$(just db-test-url); the test role needs CREATEDB");
        Self {
            options: database_options,
            cleanup: Some(cleanup),
            supervisor: Some(supervisor),
        }
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        self.cleanup.take();
        if let Some(supervisor) = self.supervisor.take() {
            let result = supervisor.join();
            if !thread::panicking() {
                result.expect("database supervisor failed");
            }
        }
    }
}

#[test]
fn supervisor_cleans_up_during_unwind() {
    let name = std::sync::Mutex::new(String::new());
    let result = std::panic::catch_unwind(|| {
        let db = TestDatabase::new();
        *name.lock().unwrap() = db.options.get_database().unwrap().to_owned();
        panic!("deliberate assertion failure");
    });
    assert!(result.is_err());
    let url = std::env::var("TEST_DATABASE_URL").unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let database_name = name.lock().unwrap().clone();
    runtime.block_on(async {
        let mut conn = PgConnection::connect(&url).await.unwrap();
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1)")
                .bind(database_name)
                .fetch_one(&mut conn)
                .await
                .unwrap();
        assert!(!exists);
    });
}

#[test]
fn concurrent_databases_have_distinct_names() {
    let first = TestDatabase::new();
    let second = TestDatabase::new();
    assert_ne!(first.options.get_database(), second.options.get_database());
}
