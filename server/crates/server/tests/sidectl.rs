//! `sidectl` against a fresh database per test. Its effects are checked
//! through the in-process router, which reads the database on every request.
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    response::Response,
    Router,
};
use sidereal_core::database::{initialize, Database};
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
    use sidereal_server::{config::DatabaseConfig, sidectl::Terminal};
    use std::{collections::VecDeque, io};

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

    struct Outcome {
        status: u8,
        out: String,
        err: String,
    }

    async fn sidectl(db: &TestDatabase, args: &[&str], terminal: &mut Scripted) -> Outcome {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let options = db.options.clone();
        let status = sidereal_server::sidectl::run(
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

    async fn reset(db: &TestDatabase, answers: &[&'static str]) -> Outcome {
        sidectl(db, &["reset-admin-password"], &mut Scripted::new(answers)).await
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
        let outcome = sidectl(
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
                        database.claim("alex", PASSWORD).await.unwrap();
                        database.close().await;
                    });
                })
                .join()
                .unwrap();
            }),
        ));
        let outcome = sidectl(&db, &["reset-admin-password"], &mut terminal).await;
        assert_eq!(outcome.status, 1);
        assert_eq!(outcome.out, "No administrator exists. Creating one.\n");
        assert_eq!(
            outcome.err,
            "an administrator now exists; run the command again to set its password\n"
        );
    }
}
