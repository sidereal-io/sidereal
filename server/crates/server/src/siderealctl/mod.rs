//! `siderealctl`, the operator's command line on the box. Every database effect
//! is a `core` method the HTTP routes could also call.
use crate::config::DatabaseConfig;
use args::{Options, Parsed, Usage, USAGE};
use prompts::{Refusal, State};
use sidereal_core::{
    access::{ClaimError, Reset, ResetError, Unavailable},
    database::{Database, DatabaseError},
};
use std::io::{self, Write};

pub mod args;
pub mod prompts;
pub mod terminal;

/// Where the command asks its questions. A read answers `Ok(None)` when
/// stdin closes before a line, which is not the same as an empty line.
pub trait Terminal {
    /// Whether stdin is a terminal a person types into.
    fn is_interactive(&self) -> bool;
    fn prompt(&mut self, text: &str) -> io::Result<()>;
    fn read_line(&mut self) -> io::Result<Option<String>>;
    /// Read a line without echoing it.
    fn read_hidden(&mut self) -> io::Result<Option<String>>;
}

/// Run one command line and answer its exit status: 0 done, 1 refused, 2 a
/// usage error. `config` is read only once the command line is accepted, so
/// help and usage never need a database.
pub async fn run(
    args: impl IntoIterator<Item = String>,
    config: impl FnOnce() -> Result<DatabaseConfig, DatabaseError>,
    terminal: &mut impl Terminal,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    let options = match args::parse(args) {
        Ok(Parsed::Help) => {
            let _ = write!(out, "{USAGE}");
            return 0;
        }
        Ok(Parsed::ResetAdminPassword(options)) => options,
        Err(Usage(message)) => return usage(err, &message),
    };
    let database = match config() {
        Ok(config) => Database::open(config.options, config.startup_timeout).await,
        Err(error) => Err(error),
    };
    let database = match database {
        Ok(database) => database,
        Err(error) => return refuse(err, &error.to_string()),
    };
    let status = reset_admin_password(&database, &options, terminal, out, err).await;
    database.close().await;
    status
}

/// Read the state, ask, check, and only then make one `core` call.
async fn reset_admin_password(
    database: &Database,
    options: &Options,
    terminal: &mut impl Terminal,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    let state = match database.administrator().await {
        Ok(None) => State::Create,
        Ok(Some(administrator)) => State::Reset { administrator },
        Err(Unavailable) => return refuse(err, UNAVAILABLE),
    };
    let _ = match &state {
        State::Create => writeln!(out, "No administrator exists. Creating one."),
        State::Reset { administrator } => writeln!(
            out,
            "Administrator {administrator} exists. Setting a new password and ending every session."
        ),
    };
    let _ = out.flush();
    let credentials = match prompts::credentials(&state, options, terminal) {
        Ok(credentials) => credentials,
        Err(Refusal::Usage(message)) => return usage(err, &message),
        Err(Refusal::Refused(message)) => return refuse(err, &message),
    };
    let done = match state {
        State::Create => match database
            .claim(&credentials.username, &credentials.password)
            .await
        {
            Ok(user) => format!("Created administrator {}.", user.username),
            Err(ClaimError::Claimed) => {
                return refuse(
                    err,
                    "an administrator now exists; run the command again to set its password",
                )
            }
            Err(ClaimError::Invalid) => return refuse(err, "the credentials break a rule"),
            Err(ClaimError::Unavailable) => return refuse(err, UNAVAILABLE),
        },
        State::Reset { .. } => match database.reset_admin_password(&credentials.password).await {
            Ok(Reset { username, sessions }) => format!(
                "Reset the password of administrator {username} and ended {sessions} sessions."
            ),
            Err(ResetError::NoAdministrator) => {
                return refuse(
                    err,
                    "the administrator no longer exists; run the command again to create one",
                )
            }
            Err(ResetError::Invalid) => return refuse(err, "the password breaks a rule"),
            Err(ResetError::Unavailable) => return refuse(err, UNAVAILABLE),
        },
    };
    // Printed only now, so a password that was never stored is never shown.
    if credentials.generated {
        let _ = writeln!(out, "Generated password: {}", credentials.password);
    }
    let _ = writeln!(out, "{done}");
    0
}

const UNAVAILABLE: &str = "database unavailable: check that PostgreSQL is running, then try again";

fn usage(err: &mut impl Write, message: &str) -> u8 {
    let _ = write!(err, "siderealctl: {message}\n\n{USAGE}");
    2
}

fn refuse(err: &mut impl Write, message: &str) -> u8 {
    let _ = writeln!(err, "{message}");
    1
}
