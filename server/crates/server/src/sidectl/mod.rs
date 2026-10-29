//! `sidectl`, the operator's command line on the box. Every database effect
//! is a `core` method the HTTP routes could also call.
use std::io;

pub mod args;
pub mod prompts;

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
