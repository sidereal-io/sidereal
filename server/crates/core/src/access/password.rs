//! Argon2id hashing, run on the blocking pool because each hash takes tens
//! of milliseconds.
use super::Unavailable;
use argon2::{Argon2, PasswordHasher};

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
