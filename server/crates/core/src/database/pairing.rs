//! Pair the database with a managed store. Runs under the startup lock, so
//! starts against one database are serialized; markers are still created
//! exclusively because another database could share the same directory.
use super::{DatabaseError, StartupError};
use crate::storage::{Contents, Created, Marker, Problem, StorageError, StorageRoot};
use sqlx::PgConnection;
use uuid::Uuid;

/// Re-reads after losing an exclusive-create race before giving up.
const ATTEMPTS: usize = 3;

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Continue,
    Create,
    Adopt(Uuid),
    Refuse(Problem),
}

/// The five startup rules, as a pure decision over what startup observed.
fn decide(paired: Option<Uuid>, marker: Marker, contents: Contents) -> Action {
    match (paired, marker, contents) {
        // 1. Paired and matching: other contents do not matter.
        (Some(id), Marker::Valid(found), _) if found == id => Action::Continue,
        // 2. Paired with an absent, malformed, or different marker.
        (Some(_), Marker::Absent, _) => Action::Refuse(Problem::MissingMarker),
        (Some(_), Marker::Invalid(problem), _) => Action::Refuse(Problem::InvalidMarker(problem)),
        (Some(_), Marker::Valid(_), _) => Action::Refuse(Problem::MismatchedMarker),
        // 3. Unpaired and empty.
        (None, _, Contents::Empty) => Action::Create,
        // 4. Unpaired with only a valid marker: an interrupted first start.
        (None, Marker::Valid(id), Contents::MarkerOnly) => Action::Adopt(id),
        (None, Marker::Invalid(problem), Contents::MarkerOnly) => {
            Action::Refuse(Problem::InvalidMarker(problem))
        }
        // The marker vanished between the two reads.
        (None, Marker::Absent, Contents::MarkerOnly) => Action::Refuse(Problem::Contended),
        // 5. Any other unpaired state.
        (None, _, Contents::Other) => Action::Refuse(Problem::Occupied),
    }
}

pub(super) async fn pair(conn: &mut PgConnection, root: &StorageRoot) -> Result<(), StartupError> {
    pair_with(conn, root, StorageRoot::create_marker).await
}

/// `create` is a seam so tests can lose the exclusive-create race on purpose.
async fn pair_with(
    conn: &mut PgConnection,
    root: &StorageRoot,
    create: impl Fn(&StorageRoot, Uuid) -> Result<Created, StorageError>,
) -> Result<(), StartupError> {
    let paired: Option<Uuid> =
        sqlx::query_scalar("SELECT storage_id FROM public.sidereal_metadata")
            .fetch_one(&mut *conn)
            .await
            .map_err(DatabaseError::from)?;
    if paired.is_some() {
        match decide(paired, root.marker()?, Contents::Other) {
            Action::Continue => {}
            Action::Refuse(problem) => return Err(root.error(problem).into()),
            Action::Create | Action::Adopt(_) => {
                unreachable!("a paired database is never re-paired")
            }
        }
    } else {
        unpaired(conn, root, create).await?;
    }
    // The UUID is committed or confirmed; a probe failure leaves it in place.
    root.probe()?;
    Ok(())
}

async fn unpaired(
    conn: &mut PgConnection,
    root: &StorageRoot,
    create: impl Fn(&StorageRoot, Uuid) -> Result<Created, StorageError>,
) -> Result<(), StartupError> {
    for _ in 0..ATTEMPTS {
        let marker = root.marker()?;
        match decide(None, marker, root.contents()?) {
            Action::Create => {
                let id = Uuid::new_v4();
                match create(root, id)? {
                    Created::Published => return record(conn, root, id).await,
                    // Someone else published first: re-read instead of using `id`.
                    Created::Exists => continue,
                }
            }
            Action::Adopt(id) => {
                root.sync_marker()?;
                return record(conn, root, id).await;
            }
            Action::Refuse(problem) => return Err(root.error(problem).into()),
            Action::Continue => unreachable!("an unpaired database has nothing to match"),
        }
    }
    Err(root.error(Problem::Contended).into())
}

/// Persist the synced marker's UUID. A denied update leaves a marker-only
/// store that the next start adopts once permissions are corrected.
async fn record(conn: &mut PgConnection, root: &StorageRoot, id: Uuid) -> Result<(), StartupError> {
    let updated =
        sqlx::query("UPDATE public.sidereal_metadata SET storage_id = $1 WHERE storage_id IS NULL")
            .bind(id)
            .execute(&mut *conn)
            .await
            .map_err(DatabaseError::from)?;
    if updated.rows_affected() == 1 {
        Ok(())
    } else {
        Err(root.error(Problem::Contended).into())
    }
}

#[cfg(test)]
mod tests;
