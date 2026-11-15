//! `siderealctl` against a fresh database per test. Its effects are checked
//! through the in-process router, which reads the database on every request.
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    response::Response,
    Router,
};
use sidereal_core::{
    access::Source,
    database::{initialize, Database},
};
use sidereal_server::{app_with_database, config::AllowedOrigins};
use std::time::Duration;
use tower::ServiceExt;

#[path = "../../core/src/database/test_support.rs"]
mod test_support;
use test_support::TestDatabase;

const PASSWORD: &str = "correct-horse-battery";
const NEW_PASSWORD: &str = "new-horse-battery";

async fn database(db: &TestDatabase) -> Database {
    initialize(
        db.options.clone(),
        Duration::from_secs(10),
        Duration::from_secs(10),
    )
    .await
    .unwrap()
}

/// The server's routes on a migrated database, as the server would start.
async fn router(db: &TestDatabase) -> Router {
    app_with_database(database(db).await, AllowedOrigins::default())
}

async fn send(app: &Router, method: &str, uri: &str, body: Option<String>) -> Response {
    send_with(app, method, uri, body, None).await
}

async fn send_with(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<String>,
    cookie: Option<&str>,
) -> Response {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    if body.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/json");
    }
    let body = body.map_or_else(Body::empty, Body::from);
    app.clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}

fn credentials(username: &str, password: &str) -> String {
    serde_json::json!({ "username": username, "password": password }).to_string()
}

async fn claim(app: &Router, username: &str, password: &str) {
    let response = send(
        app,
        "POST",
        "/api/claim",
        Some(credentials(username, password)),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

async fn sign_in(app: &Router, username: &str, password: &str) -> StatusCode {
    send(
        app,
        "POST",
        "/api/session",
        Some(credentials(username, password)),
    )
    .await
    .status()
}

/// Sign in as `sam` and answer the `name=value` pair the browser sends back.
async fn signed_in(app: &Router) -> String {
    let response = send(
        app,
        "POST",
        "/api/session",
        Some(credentials("sam", PASSWORD)),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers().get(header::SET_COOKIE).unwrap();
    cookie
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

async fn claim_state(app: &Router) -> String {
    let response = send(app, "GET", "/api/claim", None).await;
    let bytes = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

mod in_process {
    use super::*;
    use sidereal_server::{config::DatabaseConfig, siderealctl::Terminal};
    use sqlx::{Connection, PgConnection};
    use std::{collections::VecDeque, io};

    type Row = (String, String, Option<String>, Option<i64>, bool);

    /// Each `security_events` row's source, kind, username and sessions
    /// ended, and whether it names no client.
    async fn rows(db: &TestDatabase) -> Vec<Row> {
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        sqlx::query_as(
            "SELECT source, kind, username, sessions_ended, client_address IS NULL AND user_agent IS NULL \
             FROM public.security_events ORDER BY id",
        )
        .fetch_all(&mut conn)
        .await
        .unwrap()
    }

    fn from_the_box(kind: &str, sessions_ended: Option<i64>) -> Row {
        (
            "siderealctl".into(),
            kind.into(),
            Some("sam".into()),
            sessions_ended,
            true,
        )
    }

    /// The `security event:` lines of a command's stderr.
    fn events(err: &str) -> Vec<&str> {
        err.lines()
            .filter(|line| line.starts_with("security event:"))
            .collect()
    }

    /// A terminal a person types into, answering from a script. An exhausted
    /// script is a closed stdin. `before_read` runs once, before the read it
    /// names, so a test can change the database while the command waits.
    struct Scripted {
        answers: VecDeque<&'static str>,
        reads: usize,
        before_read: Option<(usize, Box<dyn FnOnce()>)>,
    }

    impl Scripted {
        fn new(answers: &[&'static str]) -> Self {
            Self {
                answers: answers.iter().copied().collect(),
                reads: 0,
                before_read: None,
            }
        }

        fn read(&mut self) -> io::Result<Option<String>> {
            if self
                .before_read
                .as_ref()
                .is_some_and(|(n, _)| *n == self.reads)
            {
                (self.before_read.take().unwrap().1)();
            }
            self.reads += 1;
            Ok(self.answers.pop_front().map(str::to_owned))
        }
    }

    impl Terminal for Scripted {
        fn is_interactive(&self) -> bool {
            true
        }
        fn prompt(&mut self, _text: &str) -> io::Result<()> {
            Ok(())
        }
        fn read_line(&mut self) -> io::Result<Option<String>> {
            self.read()
        }
        fn read_hidden(&mut self) -> io::Result<Option<String>> {
            self.read()
        }
    }

    pub(super) struct Outcome {
        pub status: u8,
        out: String,
        pub err: String,
    }

    async fn siderealctl(db: &TestDatabase, args: &[&str], terminal: &mut Scripted) -> Outcome {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let options = db.options.clone();
        let status = sidereal_server::siderealctl::run(
            args.iter().map(|arg| arg.to_string()),
            move || {
                Ok(DatabaseConfig {
                    options,
                    startup_timeout: Duration::from_secs(10),
                    migration_timeout: Duration::from_secs(10),
                })
            },
            terminal,
            &mut out,
            &mut err,
        )
        .await;
        Outcome {
            status,
            out: String::from_utf8(out).unwrap(),
            err: String::from_utf8(err).unwrap(),
        }
    }

    pub(super) async fn reset(db: &TestDatabase, answers: &[&'static str]) -> Outcome {
        siderealctl(db, &["reset-admin-password"], &mut Scripted::new(answers)).await
    }

    /// A stdout whose reader went away, as under `| head -1` or on a full
    /// disk: every write after the first line fails.
    struct ClosedAfterFirstLine(Vec<u8>);

    impl io::Write for ClosedAfterFirstLine {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.0.contains(&b'\n') {
                return Err(io::Error::from(io::ErrorKind::BrokenPipe));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn in_process_a_generated_password_that_cannot_be_shown_fails_the_command() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        claim(&app, "sam", PASSWORD).await;
        let options = db.options.clone();
        let mut out = ClosedAfterFirstLine(Vec::new());
        let mut err = Vec::new();
        let status = sidereal_server::siderealctl::run(
            ["reset-admin-password".to_string()].into_iter(),
            move || {
                Ok(DatabaseConfig {
                    options,
                    startup_timeout: Duration::from_secs(10),
                    migration_timeout: Duration::from_secs(10),
                })
            },
            &mut Scripted::new(&[""]),
            &mut out,
            &mut err,
        )
        .await;
        let err = String::from_utf8(err).unwrap();
        assert_eq!(status, 1, "{err}");
        // The reset took effect, so its event line stands before the refusal.
        assert_eq!(
            err,
            "security event: kind=access_recovery outcome=success source=siderealctl username=\"sam\" sessions_ended=0\n\
             the generated password could not be shown; run the command again to set a new one\n"
        );
        // The reset itself stood: the old password no longer signs in.
        assert_eq!(
            sign_in(&app, "sam", PASSWORD).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn in_process_the_state_line_comes_first() {
        let db = TestDatabase::new();
        database(&db).await;
        let outcome = reset(&db, &["sam", PASSWORD, PASSWORD]).await;
        assert_eq!(
            outcome.out.lines().next(),
            Some("No administrator exists. Creating one.")
        );
    }

    #[tokio::test]
    async fn in_process_the_create_state_creates_an_administrator_who_can_sign_in() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        let outcome = reset(&db, &["sam", PASSWORD, PASSWORD]).await;
        assert_eq!(outcome.status, 0, "{}", outcome.err);
        assert_eq!(
            outcome.out,
            "No administrator exists. Creating one.\nCreated administrator sam.\n"
        );
        assert_eq!(claim_state(&app).await, r#"{"open":false}"#);
        assert_eq!(sign_in(&app, "sam", PASSWORD).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn in_process_the_reset_state_reports_the_sessions_it_ended() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        claim(&app, "sam", PASSWORD).await;
        signed_in(&app).await;
        signed_in(&app).await;
        let outcome = reset(&db, &[NEW_PASSWORD, NEW_PASSWORD]).await;
        assert_eq!(outcome.status, 0, "{}", outcome.err);
        assert_eq!(
            outcome.out,
            "Administrator sam exists. Setting a new password and ending every session.\n\
             Reset the password of administrator sam and ended 2 sessions.\n"
        );
    }

    #[tokio::test]
    async fn in_process_a_cookie_whose_session_the_reset_ended_is_refused() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        claim(&app, "sam", PASSWORD).await;
        let cookie = signed_in(&app).await;
        reset(&db, &[NEW_PASSWORD, NEW_PASSWORD]).await;
        let response = send_with(&app, "GET", "/api/session", None, Some(&cookie)).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn in_process_a_username_that_is_not_the_administrator_is_refused() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        claim(&app, "sam", PASSWORD).await;
        let mut terminal = Scripted::new(&[NEW_PASSWORD, NEW_PASSWORD]);
        let outcome = siderealctl(
            &db,
            &["reset-admin-password", "--username", "alex"],
            &mut terminal,
        )
        .await;
        assert_eq!(outcome.status, 1);
        assert!(outcome.err.contains("sam"), "{}", outcome.err);
        assert_eq!(sign_in(&app, "sam", PASSWORD).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn in_process_a_generated_password_is_printed_once_and_signs_in() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        claim(&app, "sam", PASSWORD).await;
        let outcome = reset(&db, &[""]).await;
        assert_eq!(outcome.status, 0, "{}", outcome.err);
        let lines: Vec<&str> = outcome.out.lines().collect();
        let [_, generated, done] = lines[..] else {
            panic!("expected three lines: {lines:?}");
        };
        let password = generated.strip_prefix("Generated password: ").unwrap();
        assert_eq!(password.len(), 24);
        assert!(password.chars().all(|c| c.is_ascii_alphanumeric()));
        assert!(done.starts_with("Reset the password of administrator sam"));
        assert!(!outcome.err.contains(password));
        assert_eq!(sign_in(&app, "sam", password).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn in_process_a_generated_password_in_the_create_state_signs_in() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        let outcome = reset(&db, &["sam", ""]).await;
        assert_eq!(outcome.status, 0, "{}", outcome.err);
        let password = outcome
            .out
            .lines()
            .find_map(|line| line.strip_prefix("Generated password: "))
            .unwrap();
        assert!(outcome.out.ends_with("Created administrator sam.\n"));
        assert_eq!(sign_in(&app, "sam", password).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn in_process_a_generated_password_is_not_printed_when_the_write_fails() {
        let db = TestDatabase::new();
        database(&db).await;
        let options = db.options.clone();
        let mut terminal = Scripted::new(&["sam", ""]);
        // Someone claims through the web while the command waits for the password.
        terminal.before_read = Some((
            1,
            Box::new(move || {
                std::thread::spawn(move || {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    runtime.block_on(async {
                        let database = Database::open(options, Duration::from_secs(10))
                            .await
                            .unwrap();
                        let browser = Source::Http {
                            client_address: None,
                            user_agent: None,
                        };
                        database.claim("alex", PASSWORD, browser).await.unwrap();
                        database.close().await;
                    });
                })
                .join()
                .unwrap();
            }),
        ));
        let outcome = siderealctl(&db, &["reset-admin-password"], &mut terminal).await;
        assert_eq!(outcome.status, 1);
        assert_eq!(outcome.out, "No administrator exists. Creating one.\n");
        assert_eq!(
            outcome.err,
            "an administrator now exists; run the command again to set its password\n"
        );
    }

    /// Run `sql` on its own connection from inside a synchronous hook.
    fn execute_now(options: sqlx::postgres::PgConnectOptions, sql: &'static str) {
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let mut conn = PgConnection::connect_with(&options).await.unwrap();
                sqlx::Executor::execute(&mut conn, sql).await.unwrap();
            });
        })
        .join()
        .unwrap();
    }

    const REFUSE_EVENTS: &str =
        "CREATE FUNCTION public.refuse() RETURNS trigger LANGUAGE plpgsql AS 'BEGIN RAISE EXCEPTION ''refused''; END'; \
         CREATE TRIGGER refuse BEFORE INSERT ON public.security_events FOR EACH STATEMENT EXECUTE FUNCTION public.refuse()";

    /// A script whose first read waits until PostgreSQL refuses every
    /// insert into `security_events`. The command has checked the schema by then.
    fn refusing(db: &TestDatabase, answers: &[&'static str]) -> Scripted {
        let options = db.options.clone();
        let mut terminal = Scripted::new(answers);
        terminal.before_read = Some((0, Box::new(move || execute_now(options, REFUSE_EVENTS))));
        terminal
    }

    /// The administrator's hash, the session count and the user count.
    async fn access(db: &TestDatabase) -> (Option<String>, i64, i64) {
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        sqlx::query_as(
            "SELECT (SELECT password_hash FROM public.users WHERE role = 'admin'), \
             (SELECT count(*) FROM public.sessions), (SELECT count(*) FROM public.users)",
        )
        .fetch_one(&mut conn)
        .await
        .unwrap()
    }

    const UNAVAILABLE: &str =
        "database unavailable: check that PostgreSQL is running, then try again\n";

    #[tokio::test]
    async fn in_process_a_reset_whose_row_is_refused_changes_nothing() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        claim(&app, "sam", PASSWORD).await;
        signed_in(&app).await;
        let before = access(&db).await;
        let mut terminal = refusing(&db, &[NEW_PASSWORD, NEW_PASSWORD]);
        let outcome = siderealctl(&db, &["reset-admin-password"], &mut terminal).await;
        assert_eq!(outcome.status, 1);
        assert_eq!(outcome.err, UNAVAILABLE);
        assert_eq!(access(&db).await, before);
    }

    #[tokio::test]
    async fn in_process_a_create_whose_row_is_refused_creates_no_administrator() {
        let db = TestDatabase::new();
        database(&db).await;
        let mut terminal = refusing(&db, &["sam", PASSWORD, PASSWORD]);
        let outcome = siderealctl(&db, &["reset-admin-password"], &mut terminal).await;
        assert_eq!(outcome.status, 1);
        assert_eq!(outcome.err, UNAVAILABLE);
        assert_eq!(access(&db).await, (None, 0, 0));
    }

    #[tokio::test]
    async fn in_process_a_create_that_loses_the_race_keeps_only_the_browser_row() {
        let db = TestDatabase::new();
        database(&db).await;
        let options = db.options.clone();
        let mut terminal = Scripted::new(&["sam", PASSWORD, PASSWORD]);
        terminal.before_read = Some((
            1,
            Box::new(move || {
                std::thread::spawn(move || {
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    runtime.block_on(async {
                        let app = app_with_database(
                            Database::open(options, Duration::from_secs(10))
                                .await
                                .unwrap(),
                            AllowedOrigins::default(),
                        );
                        claim(&app, "alex", PASSWORD).await;
                    });
                })
                .join()
                .unwrap();
            }),
        ));
        let outcome = siderealctl(&db, &["reset-admin-password"], &mut terminal).await;
        assert_eq!(outcome.status, 1);
        assert!(events(&outcome.err).is_empty(), "{}", outcome.err);
        assert_eq!(
            rows(&db).await,
            [(
                "http".into(),
                "instance_claim".into(),
                Some("alex".into()),
                None,
                true
            )]
        );
    }

    #[tokio::test]
    async fn no_secret_reaches_an_event() {
        use sidereal_server::{app_with_tuning, Log, Tuning};
        use std::sync::{Arc, Mutex};

        const CLAIMED: &str = "claimed-distinctive-1";
        const WRONG: &str = "wrong-distinctive-2";
        const RESET: &str = "reset-distinctive-3";
        let db = TestDatabase::new();
        let written = Arc::new(Mutex::new(Vec::<String>::new()));
        let log = {
            let written = written.clone();
            Log::new(move |line| written.lock().unwrap().push(line.to_owned()))
        };
        let tuning = Tuning {
            log,
            ..Tuning::default()
        };
        let app = app_with_tuning(database(&db).await, AllowedOrigins::default(), tuning);
        claim(&app, "sam", CLAIMED).await;
        let response = send(
            &app,
            "POST",
            "/api/session",
            Some(credentials("sam", CLAIMED)),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        assert_eq!(sign_in(&app, "sam", WRONG).await, StatusCode::UNAUTHORIZED);
        let signed_out = send_with(&app, "DELETE", "/api/session", None, Some(&cookie)).await;
        assert_eq!(signed_out.status(), StatusCode::NO_CONTENT);
        let outcome = reset(&db, &[RESET, RESET]).await;
        assert_eq!(outcome.status, 0, "{}", outcome.err);

        let mut lines = written.lock().unwrap().clone();
        lines.extend(events(&outcome.err).into_iter().map(str::to_owned));
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT to_jsonb(e)::text FROM public.security_events e ORDER BY id",
        )
        .fetch_all(&mut conn)
        .await
        .unwrap();
        assert_eq!((lines.len(), rows.len()), (5, 3), "{lines:?} {rows:?}");
        let token = cookie.split_once('=').unwrap().1;
        for text in lines.iter().chain(&rows) {
            for secret in [CLAIMED, WRONG, RESET, "$argon2", token] {
                assert!(!text.contains(secret), "{secret:?} in {text}");
            }
        }
    }

    #[tokio::test]
    async fn in_process_a_create_writes_its_event_line() {
        let db = TestDatabase::new();
        database(&db).await;
        let outcome = reset(&db, &["sam", PASSWORD, PASSWORD]).await;
        assert_eq!(outcome.status, 0, "{}", outcome.err);
        assert_eq!(
            outcome.err,
            "security event: kind=instance_claim outcome=success source=siderealctl username=\"sam\"\n"
        );
    }

    #[tokio::test]
    async fn in_process_a_create_keeps_its_row() {
        let db = TestDatabase::new();
        database(&db).await;
        reset(&db, &["sam", PASSWORD, PASSWORD]).await;
        assert_eq!(rows(&db).await, [from_the_box("instance_claim", None)]);
    }

    #[tokio::test]
    async fn in_process_a_reset_writes_its_event_line() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        claim(&app, "sam", PASSWORD).await;
        signed_in(&app).await;
        signed_in(&app).await;
        let outcome = reset(&db, &[NEW_PASSWORD, NEW_PASSWORD]).await;
        assert_eq!(outcome.status, 0, "{}", outcome.err);
        assert_eq!(
            events(&outcome.err),
            [
                r#"security event: kind=access_recovery outcome=success source=siderealctl username="sam" sessions_ended=2"#
            ]
        );
    }

    #[tokio::test]
    async fn in_process_a_reset_keeps_its_row() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        claim(&app, "sam", PASSWORD).await;
        signed_in(&app).await;
        signed_in(&app).await;
        reset(&db, &[NEW_PASSWORD, NEW_PASSWORD]).await;
        assert_eq!(
            rows(&db).await.last(),
            Some(&from_the_box("access_recovery", Some(2)))
        );
    }
}

/// The server's relay of `siderealctl` lines, running against the test
/// database with a capturing log.
mod relay {
    use super::{in_process::reset, *};
    use sidereal_server::{access::relay, Log};
    use sqlx::{Connection, PgConnection};
    use std::sync::{Arc, Mutex};

    /// A line `siderealctl` could send, so the relay passes it through.
    const MARKER: &str = r#"security event: kind=access_recovery outcome=success source=siderealctl username="marker""#;

    /// The lines the relay wrote to its log.
    #[derive(Clone, Default)]
    struct Lines(Arc<Mutex<Vec<String>>>);

    impl Lines {
        fn log(&self) -> Log {
            let lines = self.0.clone();
            Log::new(move |line| lines.lock().unwrap().push(line.to_owned()))
        }

        /// Every line relayed before a marker sent now. PostgreSQL delivers
        /// notifications in commit order, so nothing sent earlier is still
        /// on its way once the marker is in.
        async fn written(&self, db: &TestDatabase) -> Vec<String> {
            notify(db, MARKER).await;
            tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    let mut lines = self.0.lock().unwrap().clone();
                    if lines.last().map(String::as_str) == Some(MARKER) {
                        lines.pop();
                        return lines;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .expect("the marker never reached the log")
        }
    }

    async fn notify(db: &TestDatabase, payload: &str) {
        let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
        sqlx::query("SELECT pg_notify('sidereal_security_event', $1)")
            .bind(payload)
            .execute(&mut conn)
            .await
            .unwrap();
    }

    /// A migrated database with the relay running, writing to the returned
    /// `Lines`.
    async fn relaying(db: &TestDatabase) -> (Database, Lines) {
        let database = database(db).await;
        let lines = Lines::default();
        relay(&database, lines.log()).await.unwrap();
        (database, lines)
    }

    #[tokio::test]
    async fn relay_writes_a_reset_from_the_box() {
        let db = TestDatabase::new();
        let (database, lines) = relaying(&db).await;
        let app = app_with_database(database, AllowedOrigins::default());
        claim(&app, "sam", PASSWORD).await;
        signed_in(&app).await;
        let outcome = reset(&db, &[NEW_PASSWORD, NEW_PASSWORD]).await;
        assert_eq!(outcome.status, 0, "{}", outcome.err);
        assert_eq!(
            lines.written(&db).await,
            [
                r#"security event: kind=access_recovery outcome=success source=siderealctl username="sam" sessions_ended=1"#
            ]
        );
    }

    #[tokio::test]
    async fn relay_writes_a_create_from_the_box() {
        let db = TestDatabase::new();
        let (_database, lines) = relaying(&db).await;
        let outcome = reset(&db, &["sam", PASSWORD, PASSWORD]).await;
        assert_eq!(outcome.status, 0, "{}", outcome.err);
        assert_eq!(
            lines.written(&db).await,
            [
                r#"security event: kind=instance_claim outcome=success source=siderealctl username="sam""#
            ]
        );
    }

    #[tokio::test]
    async fn relay_drops_a_line_siderealctl_never_sends() {
        let db = TestDatabase::new();
        let (_database, lines) = relaying(&db).await;
        notify(&db, "forged").await;
        notify(
            &db,
            r#"security event: kind=sign_in outcome=success source=http username="sam""#,
        )
        .await;
        assert!(lines.written(&db).await.is_empty());
    }
}

mod spawned {
    use super::*;
    use sqlx::{Connection, Executor, PgConnection};
    use std::{
        io::{BufRead, BufReader, Read, Write},
        process::{Child, Command, Stdio},
    };

    struct Run {
        status: i32,
        out: String,
        err: String,
    }

    /// The URL `siderealctl` reads: the test role's URL, naming this test's database.
    fn url(db: &TestDatabase) -> String {
        let mut url = url::Url::parse(&std::env::var("TEST_DATABASE_URL").unwrap()).unwrap();
        url.set_path(db.options.get_database().unwrap());
        url.to_string()
    }

    fn start(url: Option<&str>, args: &[&str], stdin: Stdio) -> Child {
        let mut command = Command::new(env!("CARGO_BIN_EXE_siderealctl"));
        command
            .env_clear()
            .args(args)
            .stdin(stdin)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(url) = url {
            command.env("DATABASE_URL", url);
        }
        command.spawn().unwrap()
    }

    fn finish(child: Child) -> Run {
        let output = child.wait_with_output().unwrap();
        Run {
            status: output.status.code().unwrap(),
            out: String::from_utf8(output.stdout).unwrap(),
            err: String::from_utf8(output.stderr).unwrap(),
        }
    }

    /// Run `siderealctl` with `input` on stdin, or with stdin closed and empty.
    fn siderealctl(url: Option<&str>, args: &[&str], input: Option<&str>) -> Run {
        let Some(input) = input else {
            return finish(start(url, args, Stdio::null()));
        };
        let mut child = start(url, args, Stdio::piped());
        // The command may refuse before it reads; a closed pipe is fine then.
        let _ = child.stdin.take().unwrap().write_all(input.as_bytes());
        finish(child)
    }

    const RESET: &[&str] = &["reset-admin-password", "--password-stdin"];

    fn reset(db: &TestDatabase, input: Option<&str>) -> Run {
        siderealctl(Some(&url(db)), RESET, input)
    }

    async fn connection(db: &TestDatabase) -> PgConnection {
        PgConnection::connect_with(&db.options).await.unwrap()
    }

    /// Every user and every session, to show a refusal changed nothing.
    async fn snapshot(db: &TestDatabase) -> String {
        sqlx::query_scalar(
            "SELECT (SELECT coalesce(jsonb_agg(to_jsonb(u) ORDER BY id), '[]') FROM public.users u)::text \
             || (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY token_hash), '[]') FROM public.sessions s)::text",
        )
        .fetch_one(&mut connection(db).await)
        .await
        .unwrap()
    }

    async fn count(db: &TestDatabase, table: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT count(*) FROM public.{table}"
        )))
        .fetch_one(&mut connection(db).await)
        .await
        .unwrap()
    }

    /// A claimed instance whose administrator `sam` is signed in once.
    async fn claimed(db: &TestDatabase) {
        let app = router(db).await;
        claim(&app, "sam", PASSWORD).await;
        signed_in(&app).await;
    }

    #[tokio::test]
    async fn spawned_create_state_creates_the_administrator() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        let run = siderealctl(
            Some(&url(&db)),
            &[
                "reset-admin-password",
                "--username",
                "sam",
                "--password-stdin",
            ],
            Some("correct-horse-battery"),
        );
        assert_eq!(run.status, 0, "{}", run.err);
        assert_eq!(
            run.out,
            "No administrator exists. Creating one.\nCreated administrator sam.\n"
        );
        assert_eq!(claim_state(&app).await, r#"{"open":false}"#);
        assert_eq!(sign_in(&app, "sam", PASSWORD).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn spawned_reset_state_sets_the_password_and_ends_every_session() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        claim(&app, "sam", PASSWORD).await;
        let cookies = [signed_in(&app).await, signed_in(&app).await];
        let run = reset(&db, Some("new-horse-battery\n"));
        assert_eq!(run.status, 0, "{}", run.err);
        assert_eq!(
            run.out,
            "Administrator sam exists. Setting a new password and ending every session.\n\
             Reset the password of administrator sam and ended 2 sessions.\n"
        );
        assert_eq!(count(&db, "sessions").await, 0);
        for cookie in &cookies {
            let response = send_with(&app, "GET", "/api/session", None, Some(cookie)).await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
        assert_eq!(
            sign_in(&app, "sam", PASSWORD).await,
            StatusCode::UNAUTHORIZED
        );
        // A new router, so the failure just made does not limit the sign-in.
        let app = router(&db).await;
        assert_eq!(sign_in(&app, "sam", NEW_PASSWORD).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn spawned_reset_with_no_session_ends_none() {
        let db = TestDatabase::new();
        claim(&router(&db).await, "sam", PASSWORD).await;
        let run = reset(&db, Some("new-horse-battery\n"));
        assert_eq!(run.status, 0, "{}", run.err);
        assert!(run.out.ends_with("ended 0 sessions.\n"), "{}", run.out);
    }

    #[tokio::test]
    async fn spawned_password_stdin_strips_one_line_ending() {
        for input in ["new-horse-battery\n", "new-horse-battery\r\n"] {
            let db = TestDatabase::new();
            let app = router(&db).await;
            claim(&app, "sam", PASSWORD).await;
            assert_eq!(reset(&db, Some(input)).status, 0);
            assert_eq!(
                sign_in(&app, "sam", NEW_PASSWORD).await,
                StatusCode::OK,
                "{input:?}"
            );
        }
    }

    #[tokio::test]
    async fn spawned_password_stdin_keeps_surrounding_spaces() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        claim(&app, "sam", PASSWORD).await;
        assert_eq!(reset(&db, Some(" new-horse-battery \n")).status, 0);
        assert_eq!(
            sign_in(&app, "sam", " new-horse-battery ").await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn spawned_a_username_that_is_not_the_administrator_changes_nothing() {
        let db = TestDatabase::new();
        claimed(&db).await;
        let before = snapshot(&db).await;
        let run = siderealctl(
            Some(&url(&db)),
            &[
                "reset-admin-password",
                "--username",
                "alex",
                "--password-stdin",
            ],
            Some("new-horse-battery\n"),
        );
        assert_eq!(run.status, 1);
        assert!(run.err.contains("sam"), "{}", run.err);
        assert_eq!(snapshot(&db).await, before);
    }

    #[tokio::test]
    async fn spawned_the_administrators_username_is_accepted() {
        let db = TestDatabase::new();
        claimed(&db).await;
        let run = siderealctl(
            Some(&url(&db)),
            &[
                "reset-admin-password",
                "--username",
                "sam",
                "--password-stdin",
            ],
            Some("new-horse-battery\n"),
        );
        assert_eq!(run.status, 0, "{}", run.err);
    }

    #[tokio::test]
    async fn spawned_a_short_password_changes_nothing() {
        let db = TestDatabase::new();
        claimed(&db).await;
        let before = snapshot(&db).await;
        let run = reset(&db, Some("short\n"));
        assert_eq!(run.status, 1);
        assert_eq!(run.err, "password must be 8 to 256 bytes\n");
        assert_eq!(snapshot(&db).await, before);
    }

    #[tokio::test]
    async fn spawned_a_padded_username_creates_nothing() {
        let db = TestDatabase::new();
        database(&db).await;
        let run = siderealctl(
            Some(&url(&db)),
            &[
                "reset-admin-password",
                "--username",
                " sam",
                "--password-stdin",
            ],
            Some("correct-horse-battery\n"),
        );
        assert_eq!(run.status, 1);
        assert!(run.err.starts_with("username "), "{}", run.err);
        assert_eq!(count(&db, "users").await, 0);
    }

    #[tokio::test]
    async fn spawned_password_stdin_refuses_an_empty_line_or_no_input() {
        for input in [Some("\n"), Some(""), None] {
            let db = TestDatabase::new();
            claimed(&db).await;
            let before = snapshot(&db).await;
            let run = reset(&db, input);
            assert_eq!(run.status, 1, "{input:?}");
            assert_eq!(run.err, "a password is required on stdin\n", "{input:?}");
            assert_eq!(snapshot(&db).await, before, "{input:?}");
        }
    }

    #[tokio::test]
    async fn spawned_a_closed_stdin_at_a_prompt_changes_nothing() {
        let db = TestDatabase::new();
        claimed(&db).await;
        let before = snapshot(&db).await;
        let run = siderealctl(Some(&url(&db)), &["reset-admin-password"], None);
        assert_eq!(run.status, 1);
        assert_eq!(
            run.out,
            "Administrator sam exists. Setting a new password and ending every session.\n"
        );
        assert!(
            run.err.ends_with("stdin closed before an answer\n"),
            "{}",
            run.err
        );
        assert_eq!(snapshot(&db).await, before);
    }

    #[tokio::test]
    async fn spawned_an_empty_answer_through_a_pipe_is_refused() {
        let db = TestDatabase::new();
        claimed(&db).await;
        let before = snapshot(&db).await;
        let run = siderealctl(Some(&url(&db)), &["reset-admin-password"], Some("\n"));
        assert_eq!(run.status, 1);
        assert!(run.err.ends_with("a password is required\n"), "{}", run.err);
        assert!(!run.out.contains("Generated password"));
        assert_eq!(snapshot(&db).await, before);
    }

    #[tokio::test]
    async fn spawned_a_repeat_that_differs_changes_nothing() {
        let db = TestDatabase::new();
        claimed(&db).await;
        let before = snapshot(&db).await;
        let run = siderealctl(
            Some(&url(&db)),
            &["reset-admin-password"],
            Some("correct-horse-battery\ncorrect-horse-batery\n"),
        );
        assert_eq!(run.status, 1);
        assert!(run.err.ends_with("passwords do not match\n"), "{}", run.err);
        assert_eq!(snapshot(&db).await, before);
    }

    #[tokio::test]
    async fn spawned_prompts_go_to_stderr() {
        let db = TestDatabase::new();
        database(&db).await;
        let run = siderealctl(
            Some(&url(&db)),
            &["reset-admin-password"],
            Some("sam\ncorrect-horse-battery\ncorrect-horse-battery\n"),
        );
        assert_eq!(run.status, 0, "{}", run.err);
        assert_eq!(
            run.out,
            "No administrator exists. Creating one.\nCreated administrator sam.\n"
        );
        // Through a pipe each prompt's line is ended, so the event line
        // starts a line of its own.
        assert_eq!(
            run.err,
            "Username: \nNew password: \nRepeat password: \n\
             security event: kind=instance_claim outcome=success source=siderealctl username=\"sam\"\n"
        );
    }

    #[tokio::test]
    async fn spawned_password_stdin_before_the_claim_needs_a_username() {
        let db = TestDatabase::new();
        database(&db).await;
        let run = reset(&db, Some("correct-horse-battery\n"));
        assert_eq!(run.status, 2);
        assert!(run.err.contains("--username"), "{}", run.err);
        assert_eq!(count(&db, "users").await, 0);
    }

    #[tokio::test]
    async fn spawned_a_claim_between_the_check_and_the_write_wins() {
        let db = TestDatabase::new();
        let app = router(&db).await;
        let mut child = start(Some(&url(&db)), &["reset-admin-password"], Stdio::piped());
        let mut out = BufReader::new(child.stdout.take().unwrap());
        let mut state = String::new();
        out.read_line(&mut state).unwrap();
        assert_eq!(state, "No administrator exists. Creating one.\n");
        // The command now waits at the username prompt.
        claim(&app, "alex", PASSWORD).await;
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(b"sam\ncorrect-horse-battery\ncorrect-horse-battery\n")
            .unwrap();
        drop(stdin);
        let mut rest = String::new();
        out.read_to_string(&mut rest).unwrap();
        let run = finish(child);
        assert_eq!(run.status, 1);
        assert_eq!(rest, "");
        assert!(
            run.err.ends_with(
                "an administrator now exists; run the command again to set its password\n"
            ),
            "{}",
            run.err
        );
        let users: Vec<String> = sqlx::query_scalar("SELECT username FROM public.users")
            .fetch_all(&mut connection(&db).await)
            .await
            .unwrap();
        assert_eq!(users, ["alex"]);
    }

    #[tokio::test]
    async fn spawned_an_empty_database_is_left_without_migrations() {
        let db = TestDatabase::new();
        let run = siderealctl(
            Some(&url(&db)),
            &[
                "reset-admin-password",
                "--username",
                "sam",
                "--password-stdin",
            ],
            Some("correct-horse-battery\n"),
        );
        assert_eq!(run.status, 1);
        assert!(run.err.contains("start the server first"), "{}", run.err);
        let migrated: bool =
            sqlx::query_scalar("SELECT to_regclass('public._sqlx_migrations') IS NOT NULL")
                .fetch_one(&mut connection(&db).await)
                .await
                .unwrap();
        assert!(!migrated);
    }

    #[tokio::test]
    async fn spawned_a_foreign_database_is_refused_unchanged() {
        let db = TestDatabase::new();
        let mut conn = connection(&db).await;
        conn.execute("CREATE TABLE public.foreign_data (value text); INSERT INTO public.foreign_data VALUES ('keep')")
            .await
            .unwrap();
        let tables = "SELECT string_agg(tablename, ',' ORDER BY tablename) FROM pg_tables WHERE schemaname = 'public'";
        let before: String = sqlx::query_scalar(tables)
            .fetch_one(&mut conn)
            .await
            .unwrap();
        let run = siderealctl(
            Some(&url(&db)),
            &[
                "reset-admin-password",
                "--username",
                "sam",
                "--password-stdin",
            ],
            Some("correct-horse-battery\n"),
        );
        assert_eq!(run.status, 1);
        assert!(
            run.err
                .starts_with("database is not an empty or owned Sidereal database"),
            "{}",
            run.err
        );
        let after: String = sqlx::query_scalar(tables)
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn spawned_a_missing_database_url_is_refused() {
        let run = siderealctl(None, RESET, Some("correct-horse-battery\n"));
        assert_eq!(run.status, 1);
        assert!(
            run.err.starts_with("missing database configuration"),
            "{}",
            run.err
        );
    }

    #[tokio::test]
    async fn spawned_output_holds_no_password_hash_or_url() {
        const DISTINCTIVE: &str = "Zebra-Quartz-9931-distinctive";
        let db = TestDatabase::new();
        claimed(&db).await;
        let url = url(&db);
        let database_password = url::Url::parse(&url)
            .unwrap()
            .password()
            .unwrap()
            .to_owned();
        let run = siderealctl(Some(&url), RESET, Some(&format!("{DISTINCTIVE}\n")));
        assert_eq!(run.status, 0, "{}", run.err);
        let output = format!("{}{}", run.out, run.err);
        for secret in [DISTINCTIVE, &database_password, &url, "$argon2", "postgres"] {
            assert!(!output.contains(secret), "{secret:?} in {output}");
        }
    }

    #[test]
    fn spawned_a_rejected_login_keeps_the_database_password_out_of_stderr() {
        const DISTINCTIVE: &str = "Distinctive-Db-Secret-4471";
        let mut url = url::Url::parse(&std::env::var("TEST_DATABASE_URL").unwrap()).unwrap();
        url.set_password(Some(DISTINCTIVE)).unwrap();
        let run = siderealctl(Some(url.as_str()), RESET, Some("correct-horse-battery\n"));
        assert_eq!(run.status, 1);
        assert!(
            run.err.starts_with("database login rejected"),
            "{}",
            run.err
        );
        assert!(!run.err.contains(DISTINCTIVE));
        assert!(!run.out.contains(DISTINCTIVE));
    }

    #[test]
    fn spawned_help_names_the_command_and_its_flags_without_a_database() {
        for args in [&["--help"][..], &["reset-admin-password", "--help"][..]] {
            let run = siderealctl(None, args, None);
            assert_eq!(run.status, 0, "{args:?}");
            for name in ["reset-admin-password", "--username", "--password-stdin"] {
                assert!(run.out.contains(name), "{args:?} {name}");
            }
        }
    }

    #[test]
    fn spawned_an_unknown_command_prints_the_usage() {
        let run = siderealctl(None, &["rotate-keys"], None);
        assert_eq!(run.status, 2);
        assert!(run.err.contains("Usage: siderealctl"), "{}", run.err);
        assert!(run.out.is_empty());
    }
}
