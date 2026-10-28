//! Argon2id hashing, run on the blocking pool because each hash takes tens
//! of milliseconds.
use super::Unavailable;
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use tokio::sync::OnceCell;

static DUMMY: OnceCell<String> = OnceCell::const_new();

pub(super) async fn hash(password: &str) -> Result<String, Unavailable> {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(|_| Unavailable)?
    .map_err(|_| Unavailable)
}

/// Whether `password` matches `hash`. An unreadable hash never matches.
pub(super) async fn verify(password: &str, hash: String) -> Result<bool, Unavailable> {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .verify_password(password.as_bytes(), hash.as_str())
            .is_ok()
    })
    .await
    .map_err(|_| Unavailable)
}

/// A hash no user has, computed once, to verify against when a sign-in names
/// an unknown username.
pub(super) async fn dummy() -> Result<String, Unavailable> {
    DUMMY
        .get_or_try_init(|| hash("sidereal-unknown-user"))
        .await
        .cloned()
}
