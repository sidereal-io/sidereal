//! Argon2id hashing, run on the blocking pool because each hash takes tens
//! of milliseconds.
use super::Unavailable;
use argon2::{Argon2, PasswordHasher, PasswordVerifier};
use base64ct::{Base64UrlUnpadded, Encoding};
use tokio::sync::OnceCell;

static DUMMY: OnceCell<String> = OnceCell::const_new();

#[cfg(test)]
tokio::task_local! {
    pub(super) static VERIFICATIONS: std::cell::Cell<usize>;
}

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
    #[cfg(test)]
    let _ = VERIFICATIONS.try_with(|count| count.set(count.get() + 1));
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
/// an unknown username. Its input is random: no password matches it, and
/// verifying against it costs the same as verifying a real hash.
pub(super) async fn dummy() -> Result<String, Unavailable> {
    DUMMY
        .get_or_try_init(|| async {
            let mut bytes = [0; 32];
            rand::fill(&mut bytes);
            hash(&Base64UrlUnpadded::encode_string(&bytes)).await
        })
        .await
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_dummy_is_one_argon2id_hash_for_the_whole_process() {
        let first = dummy().await.unwrap();
        assert!(first.starts_with("$argon2id$"));
        assert_eq!(dummy().await.unwrap(), first);
    }
}
