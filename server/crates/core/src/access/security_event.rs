//! A security event: one trust-relevant act on access. One value makes its
//! `security event:` log line, so the server and `siderealctl` cannot drift
//! apart.
use super::Unavailable;
use crate::database::Database;
use sqlx::{postgres::PgListener, PgExecutor};
use std::{fmt, net::IpAddr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecurityEvent {
    pub kind: Kind,
    pub outcome: Outcome,
    pub source: Source,
    pub username: Option<String>,
    pub sessions_ended: Option<u64>,
    /// The seconds of a 429's `Retry-After`.
    pub retry_after: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    InstanceClaim,
    SignIn,
    SignOut,
    AccessRecovery,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InstanceClaim => "instance_claim",
            Self::SignIn => "sign_in",
            Self::SignOut => "sign_out",
            Self::AccessRecovery => "access_recovery",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Success,
    WrongCredentials,
    Limited,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::WrongCredentials => "wrong_credentials",
            Self::Limited => "limited",
        }
    }
}

/// Where an event came from. Only an HTTP request names a client.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Http {
        client_address: Option<IpAddr>,
        user_agent: Option<String>,
    },
    Siderealctl,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Http { .. } => "http",
            Self::Siderealctl => "siderealctl",
        }
    }

    pub fn client_address(&self) -> Option<IpAddr> {
        match self {
            Self::Http { client_address, .. } => *client_address,
            Self::Siderealctl => None,
        }
    }

    pub fn user_agent(&self) -> Option<&str> {
        match self {
            Self::Http { user_agent, .. } => user_agent.as_deref(),
            Self::Siderealctl => None,
        }
    }
}

/// The channel a `siderealctl` event's line takes to the server's log.
const CHANNEL: &str = "sidereal_security_event";

/// Send the event's line on `executor`. PostgreSQL delivers it only when the
/// act's transaction commits, so an act that did not happen sends nothing.
pub(super) async fn notify<'e>(
    executor: impl PgExecutor<'e>,
    event: &SecurityEvent,
) -> Result<(), Unavailable> {
    sqlx::query("SELECT pg_notify($1, $2)")
        .bind(CHANNEL)
        .bind(event.to_string())
        .execute(executor)
        .await?;
    Ok(())
}

impl Database {
    /// Listen for the lines `siderealctl` sends, on a pool of its own so the
    /// relay never takes an operational connection.
    pub async fn security_event_lines(&self) -> Result<SecurityEventLines, Unavailable> {
        let mut listener = PgListener::connect_with(self.listener_pool()).await?;
        listener.listen(CHANNEL).await?;
        Ok(SecurityEventLines(listener))
    }
}

/// The lines received on the channel that `siderealctl` could have sent.
pub struct SecurityEventLines(PgListener);

impl SecurityEventLines {
    /// The next relayed line. A connection lost to an I/O error is
    /// reconnected until the pool's acquire timeout, and anything sent while
    /// it was down is lost. An error means the listener is gone for good.
    pub async fn next(&mut self) -> Result<String, Unavailable> {
        loop {
            let notification = self.0.recv().await?;
            if relayed(notification.payload()) {
                return Ok(notification.payload().to_owned());
            }
        }
    }
}

/// Any session that can connect may send on the channel, so only the two
/// lines `siderealctl` sends get through, and none can break a line.
fn relayed(payload: &str) -> bool {
    const SENT: [&str; 2] = [
        "security event: kind=instance_claim outcome=success source=siderealctl",
        "security event: kind=access_recovery outcome=success source=siderealctl",
    ];
    SENT.iter().any(|prefix| payload.starts_with(prefix)) && !payload.chars().any(char::is_control)
}

/// The whole `security event:` line, with no line feed.
impl fmt::Display for SecurityEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "security event: kind={} outcome={} source={}",
            self.kind.as_str(),
            self.outcome.as_str(),
            self.source.as_str()
        )?;
        if let Some(username) = &self.username {
            write!(f, " username={}", Quoted(username))?;
        }
        if let Some(address) = self.source.client_address() {
            write!(f, " client_address={address}")?;
        }
        if let Some(user_agent) = self.source.user_agent() {
            write!(f, " user_agent={}", Quoted(user_agent))?;
        }
        if let Some(sessions) = self.sessions_ended {
            write!(f, " sessions_ended={sessions}")?;
        }
        if let Some(seconds) = self.retry_after {
            write!(f, " retry_after={seconds}")?;
        }
        Ok(())
    }
}

/// A caller-supplied value, escaped so it cannot end its field or its line.
struct Quoted<'a>(&'a str);

impl fmt::Display for Quoted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        for c in self.0.chars() {
            match c {
                '\\' => f.write_str(r"\\")?,
                '"' => f.write_str(r#"\""#)?,
                c if c.is_control() => write!(f, "{}", c.escape_unicode())?,
                c => write!(f, "{c}")?,
            }
        }
        f.write_str("\"")
    }
}

/// A listener on the channel, so a test can see what an act sent there.
#[cfg(test)]
pub(super) struct TestChannel {
    listener: sqlx::postgres::PgListener,
    options: sqlx::postgres::PgConnectOptions,
}

#[cfg(test)]
impl TestChannel {
    const END: &str = "end of the test's lines";

    pub async fn listen(db: &crate::test_support::TestDatabase) -> Self {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy_with(db.options.clone());
        let mut listener = sqlx::postgres::PgListener::connect_with(&pool)
            .await
            .unwrap();
        listener.listen(CHANNEL).await.unwrap();
        Self {
            listener,
            options: db.options.clone(),
        }
    }

    /// Send `payload` on the channel from another session.
    pub async fn send(&self, payload: &str) {
        use sqlx::Connection;
        let mut conn = sqlx::PgConnection::connect_with(&self.options)
            .await
            .unwrap();
        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(CHANNEL)
            .bind(payload)
            .execute(&mut conn)
            .await
            .unwrap();
    }

    /// Every payload sent since the last call. PostgreSQL delivers
    /// notifications in commit order, so once a marker sent now arrives,
    /// nothing committed before it is still on its way.
    pub async fn lines(&mut self) -> Vec<String> {
        self.send(Self::END).await;
        let mut lines = Vec::new();
        loop {
            let notification =
                tokio::time::timeout(std::time::Duration::from_secs(10), self.listener.recv())
                    .await
                    .expect("the marker never arrived")
                    .unwrap();
            if notification.payload() == Self::END {
                return lines;
            }
            lines.push(notification.payload().to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{database::initialize, test_support::TestDatabase};
    use std::time::Duration;

    const ADDRESS: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 20));

    fn http(client_address: Option<IpAddr>, user_agent: Option<&str>) -> Source {
        Source::Http {
            client_address,
            user_agent: user_agent.map(str::to_owned),
        }
    }

    fn event(kind: Kind, outcome: Outcome, source: Source) -> SecurityEvent {
        SecurityEvent {
            kind,
            outcome,
            source,
            username: None,
            sessions_ended: None,
            retry_after: None,
        }
    }

    #[test]
    fn a_failed_sign_in_line_names_the_client() {
        let line = event(
            Kind::SignIn,
            Outcome::WrongCredentials,
            http(Some(ADDRESS), Some("Firefox/140")),
        );
        assert_eq!(
            line.to_string(),
            r#"security event: kind=sign_in outcome=wrong_credentials source=http client_address=192.168.1.20 user_agent="Firefox/140""#
        );
    }

    #[test]
    fn a_sign_in_line_without_a_user_agent_leaves_the_field_out() {
        let line = SecurityEvent {
            username: Some("sam".into()),
            ..event(Kind::SignIn, Outcome::Success, http(Some(ADDRESS), None))
        };
        assert_eq!(
            line.to_string(),
            r#"security event: kind=sign_in outcome=success source=http username="sam" client_address=192.168.1.20"#
        );
    }

    #[test]
    fn a_reset_line_names_the_sessions_it_ended() {
        let line = SecurityEvent {
            username: Some("sam".into()),
            sessions_ended: Some(2),
            ..event(Kind::AccessRecovery, Outcome::Success, Source::Siderealctl)
        };
        assert_eq!(
            line.to_string(),
            r#"security event: kind=access_recovery outcome=success source=siderealctl username="sam" sessions_ended=2"#
        );
    }

    #[test]
    fn a_limited_line_ends_with_the_wait() {
        let line = SecurityEvent {
            retry_after: Some(60),
            ..event(
                Kind::SignIn,
                Outcome::Limited,
                http(Some(ADDRESS), Some("Firefox/140")),
            )
        };
        assert_eq!(
            line.to_string(),
            r#"security event: kind=sign_in outcome=limited source=http client_address=192.168.1.20 user_agent="Firefox/140" retry_after=60"#
        );
    }

    #[test]
    fn an_unresolved_client_address_is_left_out() {
        let line = event(Kind::SignIn, Outcome::WrongCredentials, http(None, None));
        assert_eq!(
            line.to_string(),
            "security event: kind=sign_in outcome=wrong_credentials source=http"
        );
    }

    #[test]
    fn a_line_feed_in_a_username_is_escaped() {
        let line = SecurityEvent {
            username: Some("sam\nsecurity event: kind=sign_in".into()),
            ..event(Kind::InstanceClaim, Outcome::Success, Source::Siderealctl)
        };
        assert_eq!(
            line.to_string(),
            r#"security event: kind=instance_claim outcome=success source=siderealctl username="sam\u{a}security event: kind=sign_in""#
        );
    }

    #[test]
    fn a_tab_in_a_user_agent_is_escaped() {
        let line = event(
            Kind::SignIn,
            Outcome::WrongCredentials,
            http(None, Some("Firefox\t140")),
        );
        assert_eq!(
            line.to_string(),
            r#"security event: kind=sign_in outcome=wrong_credentials source=http user_agent="Firefox\u{9}140""#
        );
    }

    #[test]
    fn a_quote_and_a_backslash_in_a_username_are_escaped() {
        let line = SecurityEvent {
            username: Some(r#"sa"m\"#.into()),
            ..event(Kind::InstanceClaim, Outcome::Success, Source::Siderealctl)
        };
        assert_eq!(
            line.to_string(),
            r#"security event: kind=instance_claim outcome=success source=siderealctl username="sa\"m\\""#
        );
    }

    #[test]
    fn a_non_ascii_username_is_written_as_is() {
        let line = SecurityEvent {
            username: Some("sämé".into()),
            ..event(Kind::InstanceClaim, Outcome::Success, Source::Siderealctl)
        };
        assert!(line.to_string().ends_with(r#"username="sämé""#));
    }
    const CLAIM_LINE: &str =
        r#"security event: kind=instance_claim outcome=success source=siderealctl username="sam""#;
    const RESET_LINE: &str = r#"security event: kind=access_recovery outcome=success source=siderealctl username="sam" sessions_ended=1"#;

    async fn database(db: &TestDatabase) -> Database {
        initialize(
            db.options.clone(),
            Duration::from_secs(10),
            Duration::from_secs(10),
        )
        .await
        .unwrap()
    }

    async fn next(lines: &mut SecurityEventLines) -> String {
        tokio::time::timeout(Duration::from_secs(10), lines.next())
            .await
            .expect("no line arrived")
            .unwrap()
    }

    #[test]
    fn only_a_siderealctl_claim_or_reset_is_relayed() {
        for line in [CLAIM_LINE, RESET_LINE] {
            assert!(relayed(line), "{line}");
        }
        for line in [
            "forged",
            r#"security event: kind=sign_in outcome=success source=http username="sam""#,
            r#"security event: kind=sign_in outcome=wrong_credentials source=siderealctl"#,
            "security event: kind=instance_claim outcome=success source=siderealctl\nsecurity event: kind=sign_in",
            "security event: kind=access_recovery outcome=success source=siderealctl\tusername=\"sam\"",
            "",
        ] {
            assert!(!relayed(line), "{line:?}");
        }
    }

    #[tokio::test]
    async fn security_event_lines_yield_only_relayed_lines() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        let mut lines = database.security_event_lines().await.unwrap();
        let channel = TestChannel::listen(&db).await;
        for payload in [
            "forged",
            r#"security event: kind=sign_in outcome=success source=http username="sam""#,
            "security event: kind=instance_claim outcome=success source=siderealctl\nsecurity event: kind=sign_in",
            CLAIM_LINE,
            "forged",
            RESET_LINE,
        ] {
            channel.send(payload).await;
        }
        assert_eq!(next(&mut lines).await, CLAIM_LINE);
        assert_eq!(next(&mut lines).await, RESET_LINE);
    }

    #[tokio::test]
    async fn security_event_lines_take_no_operational_connection() {
        let db = TestDatabase::new();
        let database = database(&db).await;
        let _lines = database.security_event_lines().await.unwrap();
        let mut held = Vec::new();
        for _ in 0..5 {
            held.push(
                tokio::time::timeout(Duration::from_secs(5), database.pool().acquire())
                    .await
                    .expect("the operational pool is short of a connection")
                    .unwrap(),
            );
        }
    }
}
