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

#[derive(Clone, Copy, Debug)]
pub(super) enum RelayMode {
    Forward,
    Refuse,
    Silent,
}

/// A loopback fault injector. Changing mode also closes existing connections.
pub(super) struct Relay {
    pub port: u16,
    mode: tokio::sync::watch::Sender<RelayMode>,
    task: tokio::task::JoinHandle<()>,
    history: std::sync::Arc<HistoryGate>,
}

#[derive(Default)]
struct HistoryGate {
    paused: std::sync::atomic::AtomicBool,
    reached: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

async fn forward(
    client: &mut tokio::net::TcpStream,
    server: &mut tokio::net::TcpStream,
    gate: &HistoryGate,
) -> std::io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    const QUERY: &[u8] =
        b"SELECT version, description, success, checksum FROM public._sqlx_migrations";
    let (mut incoming, mut outgoing) = client.split();
    let (mut reply, mut destination) = server.split();
    let send = async {
        let mut buffer = [0; 8192];
        let mut tail = Vec::new();
        loop {
            let count = incoming.read(&mut buffer).await?;
            if count == 0 {
                return destination.shutdown().await;
            }
            tail.extend_from_slice(&buffer[..count]);
            if gate.paused.load(Ordering::SeqCst)
                && tail.windows(QUERY.len()).any(|part| part == QUERY)
            {
                gate.reached.notify_one();
                gate.release.notified().await;
            }
            destination.write_all(&buffer[..count]).await?;
            if tail.len() > QUERY.len() {
                tail.drain(..tail.len() - QUERY.len());
            }
        }
    };
    let receive = tokio::io::copy(&mut reply, &mut outgoing);
    tokio::try_join!(send, receive)?;
    Ok(())
}

impl Relay {
    pub async fn new(target: std::net::SocketAddr) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (mode, mut changes) = tokio::sync::watch::channel(RelayMode::Forward);
        let history = std::sync::Arc::new(HistoryGate::default());
        let gate = history.clone();
        let task = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let Ok((mut client, _)) = accepted else { break };
                        let mut changed = changes.clone();
                        let current = *changed.borrow_and_update();
                        let gate = gate.clone();
                        connections.spawn(async move {
                            tokio::select! {
                                _ = changed.changed() => {},
                                _ = async {
                                    match current {
                                        RelayMode::Refuse => {},
                                        RelayMode::Silent => std::future::pending::<()>().await,
                                        RelayMode::Forward => {
                                            if let Ok(mut server) = tokio::net::TcpStream::connect(target).await {
                                                let _ = forward(&mut client, &mut server, &gate).await;
                                            }
                                        }
                                    }
                                } => {},
                            }
                        });
                    },
                    _ = changes.changed() => {},
                    Some(_) = connections.join_next() => {},
                }
            }
        });
        Self {
            port,
            mode,
            task,
            history,
        }
    }

    pub fn hold_history(&self) {
        self.history.paused.store(true, Ordering::SeqCst);
    }

    pub async fn history_reached(&self) {
        tokio::time::timeout(Duration::from_secs(1), self.history.reached.notified())
            .await
            .unwrap();
    }

    pub fn release_history(&self) {
        self.history.paused.store(false, Ordering::SeqCst);
        self.history.release.notify_one();
    }

    pub fn set_mode(&self, mode: RelayMode) {
        self.mode.send_replace(mode);
    }

    pub fn close_connections(&self) {
        self.mode.send_modify(|_| {});
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[tokio::test]
async fn relay_forwards_refuses_stalls_and_closes_connections() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relay = Relay::new(listener.local_addr().unwrap()).await;
    let echo = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let (mut read, mut write) = socket.split();
                let _ = tokio::io::copy(&mut read, &mut write).await;
            });
        }
    });
    let mut client = tokio::net::TcpStream::connect(("127.0.0.1", relay.port))
        .await
        .unwrap();
    client.write_all(b"hello").await.unwrap();
    let mut bytes = [0; 5];
    client.read_exact(&mut bytes).await.unwrap();
    assert_eq!(&bytes, b"hello");
    relay.close_connections();
    assert_eq!(client.read(&mut bytes).await.unwrap(), 0);
    relay.set_mode(RelayMode::Refuse);
    let mut client = tokio::net::TcpStream::connect(("127.0.0.1", relay.port))
        .await
        .unwrap();
    assert_eq!(client.read(&mut bytes).await.unwrap(), 0);
    relay.set_mode(RelayMode::Silent);
    let mut client = tokio::net::TcpStream::connect(("127.0.0.1", relay.port))
        .await
        .unwrap();
    client.write_all(b"hello").await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), client.read(&mut bytes))
            .await
            .is_err()
    );
    relay.set_mode(RelayMode::Forward);
    assert!(client.read(&mut bytes).await.unwrap_or(0) == 0);
    let mut client = tokio::net::TcpStream::connect(("127.0.0.1", relay.port))
        .await
        .unwrap();
    client.write_all(b"again").await.unwrap();
    client.read_exact(&mut bytes).await.unwrap();
    assert_eq!(&bytes, b"again");
    relay.hold_history();
    let query = b"SELECT version, description, success, checksum FROM public._sqlx_migrations";
    client.write_all(query).await.unwrap();
    relay.history_reached().await;
    relay.release_history();
    let mut echoed = vec![0; query.len()];
    client.read_exact(&mut echoed).await.unwrap();
    assert_eq!(echoed, query);
    echo.abort();
}
