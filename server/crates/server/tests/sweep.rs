//! The session sweep task against a fresh database per test, on a schedule
//! of milliseconds rather than the server's minute and hour.
use sidereal_core::{
    access::Source,
    database::{initialize, Database},
};
use sidereal_server::{
    access::sweep::{sweep, Schedule},
    Log,
};
use sqlx::{Connection, PgConnection};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[path = "../../core/src/database/test_support.rs"]
mod test_support;
use test_support::TestDatabase;

const PASSWORD: &str = "correct-horse-battery";
/// How long a test waits for the sweep before it fails.
const WAIT: Duration = Duration::from_secs(10);
/// A gap no test outlasts, so a schedule's next sweep never comes.
const NEVER: Duration = Duration::from_secs(60 * 60);

/// A migrated database with an administrator, and the startup timeout its
/// pools wait for a connection.
async fn claimed(db: &TestDatabase, startup: Duration) -> Database {
    let database = initialize(db.options.clone(), startup, Duration::from_secs(10))
        .await
        .unwrap();
    database
        .claim("sam", PASSWORD, Source::Siderealctl)
        .await
        .unwrap();
    database
}

async fn connection(db: &TestDatabase) -> PgConnection {
    PgConnection::connect_with(&db.options).await.unwrap()
}

/// Add `count` session rows whose expiry has passed.
async fn expired(db: &TestDatabase, database: &Database, count: usize) {
    for _ in 0..count {
        database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
    }
    sqlx::query("UPDATE public.sessions SET expires_at = now() - interval '1 minute'")
        .execute(&mut connection(db).await)
        .await
        .unwrap();
}

/// Add one unexpired session row.
async fn unexpired(database: &Database) {
    database.sign_in("sam", PASSWORD).await.unwrap().unwrap();
}

/// How many session rows there are, and how many of them have expired.
async fn sessions(db: &TestDatabase) -> (i64, i64) {
    sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE expires_at <= now()) FROM public.sessions",
    )
    .fetch_one(&mut connection(db).await)
    .await
    .unwrap()
}

/// Poll `done` until it answers true, failing after [`WAIT`].
async fn wait_until(mut done: impl AsyncFnMut() -> bool) {
    tokio::time::timeout(WAIT, async {
        while !done().await {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("waited too long");
}

/// The lines the sweep wrote to its log.
#[derive(Clone, Default)]
struct Lines(Arc<Mutex<Vec<String>>>);

impl Lines {
    fn log(&self) -> Log {
        let lines = self.0.clone();
        Log::new(move |line| lines.lock().unwrap().push(line.to_owned()))
    }

    fn written(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }

    /// The lines written, once there is at least one.
    async fn first(&self) -> Vec<String> {
        wait_until(async || !self.written().is_empty()).await;
        self.written()
    }
}

fn first_after(first: Duration) -> Schedule {
    Schedule {
        first,
        every: NEVER,
    }
}

#[tokio::test]
async fn nothing_is_deleted_before_the_first_sweep() {
    let db = TestDatabase::new();
    let database = claimed(&db, Duration::from_secs(10)).await;
    expired(&db, &database, 1).await;
    sweep(
        database,
        first_after(Duration::from_millis(500)),
        Lines::default().log(),
    );
    assert_eq!(sessions(&db).await, (1, 1));
}

#[tokio::test]
async fn the_first_sweep_deletes_only_expired_sessions() {
    let db = TestDatabase::new();
    let database = claimed(&db, Duration::from_secs(10)).await;
    expired(&db, &database, 1).await;
    unexpired(&database).await;
    sweep(
        database,
        first_after(Duration::from_millis(300)),
        Lines::default().log(),
    );
    wait_until(async || sessions(&db).await.1 == 0).await;
    assert_eq!(sessions(&db).await, (1, 0));
}

#[tokio::test]
async fn a_session_that_expires_later_goes_when_the_sweep_runs_again() {
    let db = TestDatabase::new();
    let database = claimed(&db, Duration::from_secs(10)).await;
    expired(&db, &database, 1).await;
    unexpired(&database).await;
    let schedule = Schedule {
        first: Duration::from_millis(200),
        every: Duration::from_secs(1),
    };
    sweep(database, schedule, Lines::default().log());
    wait_until(async || sessions(&db).await.1 == 0).await;
    sqlx::query("UPDATE public.sessions SET expires_at = now() - interval '1 minute'")
        .execute(&mut connection(&db).await)
        .await
        .unwrap();
    wait_until(async || sessions(&db).await.0 == 0).await;
}

#[tokio::test]
async fn three_deleted_sessions_write_one_line() {
    let db = TestDatabase::new();
    let database = claimed(&db, Duration::from_secs(10)).await;
    expired(&db, &database, 3).await;
    let lines = Lines::default();
    sweep(
        database,
        first_after(Duration::from_millis(200)),
        lines.log(),
    );
    assert_eq!(
        lines.first().await,
        ["session sweep: deleted 3 expired sessions"]
    );
}

#[tokio::test]
async fn one_deleted_session_writes_a_singular_line() {
    let db = TestDatabase::new();
    let database = claimed(&db, Duration::from_secs(10)).await;
    expired(&db, &database, 1).await;
    let lines = Lines::default();
    sweep(
        database,
        first_after(Duration::from_millis(200)),
        lines.log(),
    );
    assert_eq!(
        lines.first().await,
        ["session sweep: deleted 1 expired session"]
    );
}

#[tokio::test]
async fn a_sweep_that_deletes_nothing_writes_no_line() {
    let db = TestDatabase::new();
    let database = claimed(&db, Duration::from_secs(10)).await;
    unexpired(&database).await;
    let lines = Lines::default();
    sweep(
        database,
        first_after(Duration::from_millis(200)),
        lines.log(),
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(lines.written().is_empty());
}
