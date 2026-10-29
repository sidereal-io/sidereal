//! Users, their passwords and their sessions. HTTP lives in the server crate;
//! this module is reachable without it, so `sidectl` can share it.
use std::fmt;

mod claim;
mod password;
mod session;

pub use claim::ClaimError;
pub use session::{Token, SESSION_LIFETIME};

/// What a user may do. This release has one role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        (value == "admin").then_some(Self::Admin)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub username: String,
    pub role: Role,
}

/// PostgreSQL could not be read or written. It carries no driver detail, so
/// nothing about the query or its values can reach a caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unavailable;

impl fmt::Display for Unavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unavailable")
    }
}

impl std::error::Error for Unavailable {}

impl From<sqlx::Error> for Unavailable {
    fn from(_: sqlx::Error) -> Self {
        Self
    }
}
