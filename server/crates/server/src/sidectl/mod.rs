//! `sidectl`, the operator's command line on the box. Every database effect
//! is a `core` method the HTTP routes could also call.
use crate::config::DatabaseConfig;
use args::{Parsed, Usage, USAGE};
use sidereal_core::database::DatabaseError;
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
    let _ = (config, terminal);
    match args::parse(args) {
        Ok(Parsed::Help) => {
            let _ = write!(out, "{USAGE}");
            0
        }
        Err(Usage(message)) => usage(err, &message),
        Ok(Parsed::ResetAdminPassword(_)) => 1,
    }
}

fn usage(err: &mut impl Write, message: &str) -> u8 {
    let _ = write!(err, "sidectl: {message}\n\n{USAGE}");
    2
}
