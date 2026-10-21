use super::*;
use sqlx::ConnectOptions;
use std::{
    io::{BufRead, BufReader},
    process::{Child, Stdio},
    sync::{Arc, Mutex},
    time::Instant,
};
use test_support::{Relay, RelayMode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Server {
    child: Child,
    port: u16,
    lines: Arc<Mutex<Vec<String>>>,
    reader: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    async fn start(options: sqlx::postgres::PgConnectOptions) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sidereal-server"))
            .env("DATABASE_URL", options.to_url_lossy().as_str())
            .env("PORT", "0")
            .env_remove("WEB_ROOT")
            .env_remove("STORAGE_ROOT")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (address_tx, address_rx) = std::sync::mpsc::channel();
        let address_reader = std::thread::spawn(move || {
            let mut line = String::new();
            let port = BufReader::new(stdout)
                .read_line(&mut line)
                .ok()
                .and_then(|_| line.split_once("listening on "))
                .and_then(|(_, value)| value.split_whitespace().next())
                .and_then(|value| value.parse::<std::net::SocketAddr>().ok())
                .map(|addr| addr.port());
            let _ = address_tx.send(port);
        });
        let stderr = child.stderr.take().unwrap();
        let lines = Arc::new(Mutex::new(Vec::new()));
        let captured = lines.clone();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                captured.lock().unwrap().push(line.unwrap());
            }
        });
        let mut server = Self {
            child,
            port: 0,
            lines,
            reader: Some(reader),
        };
        let start = Instant::now();
        loop {
            if let Ok(port) = address_rx.try_recv() {
                server.port = port.expect("server must report its bound address");
                assert_ne!(server.port, 0);
                break;
            }
            assert!(
                server.child.try_wait().unwrap().is_none(),
                "server exited: {:?}",
                server.logs()
            );
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "server failed to listen"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        address_reader.join().unwrap();
        server
    }

    fn logs(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }

    async fn expect_logs(&self, expected: &[&str]) {
        let start = Instant::now();
        loop {
            let actual = self.logs();
            if actual.len() >= expected.len() {
                assert_eq!(actual, expected);
                return;
            }
            assert!(
                start.elapsed() < Duration::from_secs(1),
                "missing lines: {actual:?}"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.reader.take().unwrap().join().unwrap();
    }
}

async fn request(port: u16, path: &str, ready: bool) {
    let exchange = async {
        let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        socket
            .write_all(
                format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await
            .unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).await.unwrap();
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        let (status, body_expected) = match (path, ready) {
            ("/healthz", _) => ("200", "{\"status\":\"ok\"}"),
            (_, true) => ("200", "{\"status\":\"ready\"}"),
            (_, false) => ("503", "{\"status\":\"not_ready\"}"),
        };
        assert!(
            headers.starts_with(&format!("HTTP/1.1 {status}")),
            "{response}"
        );
        assert_eq!(body, body_expected);
    };
    tokio::time::timeout(Duration::from_millis(2250), exchange)
        .await
        .unwrap();
}

async fn blocked_query(conn: &mut PgConnection, history: bool) -> i32 {
    let start = Instant::now();
    loop {
        conn.execute("SELECT pg_stat_clear_snapshot()")
            .await
            .unwrap();
        let pid: Option<i32> = sqlx::query_scalar("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND ($1 = false OR query LIKE 'SELECT version, description, success, checksum FROM public._sqlx_migrations%') AND wait_event_type='Lock' AND pid <> pg_backend_pid()")
            .bind(history).fetch_optional(&mut *conn).await.unwrap();
        if let Some(pid) = pid {
            return pid;
        }
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "probe did not reach migration history"
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[tokio::test]
async fn ready_probes_write_no_lines() {
    let db = TestDatabase::new();
    let server = Server::start(db.options.clone()).await;
    for _ in 0..20 {
        request(server.port, "/readyz", true).await;
    }
    server.expect_logs(&[]).await;
}

#[tokio::test]
async fn outage_recovery_and_overlapping_probes_are_redacted() {
    let db = TestDatabase::new();
    let target = format!("127.0.0.1:{}", db.options.get_port())
        .parse()
        .unwrap();
    let relay = Relay::new(target).await;
    let options = db.options.clone().host("127.0.0.1").port(relay.port);
    let url = options.to_url_lossy().to_string();
    let server = Server::start(options).await;
    request(server.port, "/readyz", true).await;
    relay.set_mode(RelayMode::Silent);
    let port = server.port;
    let first = tokio::spawn(request(port, "/readyz", false));
    tokio::time::sleep(Duration::from_millis(100)).await;
    let start = Instant::now();
    request(port, "/readyz", true).await;
    assert!(start.elapsed() < Duration::from_millis(250));
    first.await.unwrap();
    server
        .expect_logs(&["readiness: not ready (database unreachable)"])
        .await;
    let next = tokio::spawn(request(port, "/readyz", false));
    tokio::time::sleep(Duration::from_millis(100)).await;
    let start = Instant::now();
    request(port, "/readyz", false).await;
    assert!(start.elapsed() < Duration::from_millis(250));
    next.await.unwrap();
    relay.set_mode(RelayMode::Refuse);
    let outage = Instant::now();
    while outage.elapsed() < Duration::from_secs(10) {
        request(port, "/readyz", false).await;
        request(port, "/healthz", true).await;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    server
        .expect_logs(&["readiness: not ready (database unreachable)"])
        .await;
    relay.set_mode(RelayMode::Forward);
    let start = Instant::now();
    request(port, "/readyz", true).await;
    assert!(start.elapsed() < Duration::from_secs(5));
    server
        .expect_logs(&[
            "readiness: not ready (database unreachable)",
            "readiness: ready",
        ])
        .await;
    let logs = server.logs().join("\n");
    assert!(!logs.contains("fixture_test"));
    assert!(!logs.contains(&url));
    assert!(!logs.contains("postgres"));
}

#[tokio::test]
async fn locked_history_times_out_and_overlapping_probe_keeps_last_answer() {
    let db = TestDatabase::new();
    let server = Server::start(db.options.clone().options([("lock_timeout", "25ms")])).await;
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    conn.execute("BEGIN; LOCK TABLE public._sqlx_migrations IN ACCESS EXCLUSIVE MODE")
        .await
        .unwrap();
    let port = server.port;
    let pending = tokio::spawn(request(port, "/readyz", false));
    blocked_query(&mut conn, false).await;
    let start = Instant::now();
    request(port, "/readyz", true).await;
    assert!(start.elapsed() < Duration::from_millis(250));
    pending.await.unwrap();
    server
        .expect_logs(&["readiness: not ready (timed out)"])
        .await;
    // A different failing cause writes a second line; repeats do not.
    conn.execute("ROLLBACK; UPDATE public._sqlx_migrations SET version=99 WHERE version=2")
        .await
        .unwrap();
    request(port, "/readyz", false).await;
    request(port, "/readyz", false).await;
    server
        .expect_logs(&[
            "readiness: not ready (timed out)",
            "readiness: not ready (incompatible migration history)",
        ])
        .await;
    conn.execute("UPDATE public._sqlx_migrations SET version=2 WHERE version=99")
        .await
        .unwrap();
    request(port, "/readyz", true).await;
    server
        .expect_logs(&[
            "readiness: not ready (timed out)",
            "readiness: not ready (incompatible migration history)",
            "readiness: ready",
        ])
        .await;
}

#[tokio::test]
async fn history_connection_loss_and_backend_shutdown_are_unreachable() {
    for terminate in [false, true] {
        let db = TestDatabase::new();
        let relay = Relay::new(
            format!("127.0.0.1:{}", db.options.get_port())
                .parse()
                .unwrap(),
        )
        .await;
        let server = Server::start(db.options.clone().host("127.0.0.1").port(relay.port)).await;
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        relay.hold_history();
        let port = server.port;
        let pending = tokio::spawn(request(port, "/readyz", false));
        relay.history_reached().await;
        conn.execute("BEGIN; LOCK TABLE public._sqlx_migrations IN ACCESS EXCLUSIVE MODE")
            .await
            .unwrap();
        relay.release_history();
        let pid = blocked_query(&mut conn, true).await;
        if terminate {
            sqlx::query("SELECT pg_terminate_backend($1)")
                .bind(pid)
                .execute(&mut conn)
                .await
                .unwrap();
        } else {
            relay.close_connections();
        }
        pending.await.unwrap();
        server
            .expect_logs(&["readiness: not ready (database unreachable)"])
            .await;
        conn.execute("ROLLBACK").await.unwrap();
        request(port, "/readyz", true).await;
        server
            .expect_logs(&[
                "readiness: not ready (database unreachable)",
                "readiness: ready",
            ])
            .await;
    }
}
