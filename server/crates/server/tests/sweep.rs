//! The session sweep task against a fresh database per test, on a schedule
//! of milliseconds rather than the server's minute and hour.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sidereal_core::{
    access::Source,
    database::{initialize, Database},
};
use sidereal_server::{
    access::sweep::{sweep, Schedule},
    app_with_database,
    config::AllowedOrigins,
    Log,
};
use sqlx::{AssertSqlSafe, Connection, Executor, PgConnection};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::task::JoinHandle;
use tower::ServiceExt;

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

/// What a sweep writes when it cannot reach PostgreSQL.
const UNAVAILABLE: &str =
    "database unavailable: cannot sweep expired sessions; trying again in an hour";
/// The sweep's pools give up waiting for a connection after this long.
const ACQUIRE: Duration = Duration::from_secs(1);

/// Refuse every new connection to the test database, and end every client
/// connection but the returned one, which the test keeps. An autovacuum
/// worker is left alone: the test role may not end it.
async fn refuse_connections(db: &TestDatabase) -> PgConnection {
    let name = db.options.get_database().unwrap();
    let mut conn = connection(db).await;
    conn.execute(AssertSqlSafe(format!(
        "ALTER DATABASE {name} CONNECTION LIMIT 0"
    )))
    .await
    .unwrap();
    sqlx::query(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
         WHERE datname = current_database() AND backend_type = 'client backend' \
         AND pid <> pg_backend_pid()",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    conn
}

/// A sweep task whose first sweep, a second after it starts, finds the
/// database refusing connections. Before it starts, the database holds
/// `expired_before` expired sessions.
async fn sweeping_while_unavailable(
    db: &TestDatabase,
    lines: &Lines,
    expired_before: usize,
    every: Duration,
) -> Outage {
    let database = claimed(db, ACQUIRE).await;
    expired(db, &database, expired_before).await;
    let schedule = Schedule {
        first: Duration::from_secs(1),
        every,
    };
    let task = sweep(database.clone(), schedule, lines.log());
    let conn = refuse_connections(db).await;
    Outage {
        database,
        task,
        conn,
    }
}

/// A sweep task running while the database refuses connections.
struct Outage {
    database: Database,
    task: JoinHandle<()>,
    /// The test's own connection, which outlives the outage.
    conn: PgConnection,
}

impl Outage {
    async fn end(mut self, db: &TestDatabase) {
        let name = db.options.get_database().unwrap();
        self.conn
            .execute(AssertSqlSafe(format!(
                "ALTER DATABASE {name} CONNECTION LIMIT -1"
            )))
            .await
            .unwrap();
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

#[tokio::test]
async fn an_unavailable_database_gets_one_warning() {
    let db = TestDatabase::new();
    let lines = Lines::default();
    let _outage = sweeping_while_unavailable(&db, &lines, 0, NEVER).await;
    assert_eq!(lines.first().await, [UNAVAILABLE]);
}

#[tokio::test]
async fn the_sweep_task_outlives_an_unavailable_database() {
    let db = TestDatabase::new();
    let lines = Lines::default();
    let outage = sweeping_while_unavailable(&db, &lines, 0, NEVER).await;
    lines.first().await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(!outage.task.is_finished());
}

#[tokio::test]
async fn healthz_answers_200_while_the_sweep_finds_the_database_unavailable() {
    let db = TestDatabase::new();
    let lines = Lines::default();
    let outage = sweeping_while_unavailable(&db, &lines, 0, NEVER).await;
    lines.first().await;
    let response = app_with_database(outage.database.clone(), AllowedOrigins::default())
        .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

/// A failed sweep waits for the pool's acquire timeout, so the outage ends
/// only once its warning is written, and the next sweep is due after that.
#[tokio::test]
async fn the_sweep_recovers_once_the_database_is_back() {
    let db = TestDatabase::new();
    let lines = Lines::default();
    let outage = sweeping_while_unavailable(&db, &lines, 1, Duration::from_secs(3)).await;
    assert_eq!(lines.first().await, [UNAVAILABLE]);
    outage.end(&db).await;
    wait_until(async || sessions(&db).await == (0, 0)).await;
}
