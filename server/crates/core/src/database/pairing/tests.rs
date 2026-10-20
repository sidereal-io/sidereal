use super::*;
use crate::{
    database::{initialize, initialize_with_storage},
    storage::{
        tests::{running_as_root, TempRoot},
        MarkerProblem, Operation, MARKER,
    },
    test_support::TestDatabase,
};
use sqlx::{Connection, Executor};
use std::{io, time::Duration};

const SECONDS: Duration = Duration::from_secs(3);

async fn start(db: &TestDatabase, root: &TempRoot) -> Result<(), StartupError> {
    let database =
        initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &root.root()).await?;
    assert!(database.ready().await);
    database.close().await;
    Ok(())
}

async fn database_only(db: &TestDatabase) {
    initialize(db.options.clone(), SECONDS, SECONDS)
        .await
        .unwrap()
        .close()
        .await;
}

async fn stored_id(db: &TestDatabase) -> Option<Uuid> {
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    let id = sqlx::query_scalar("SELECT storage_id FROM public.sidereal_metadata")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    conn.close().await.unwrap();
    id
}

fn marker_id(root: &TempRoot) -> Uuid {
    match root.root().marker().unwrap() {
        Marker::Valid(id) => id,
        other => panic!("expected a valid marker, found {other:?}"),
    }
}

fn valid_marker(root: &TempRoot, id: Uuid) -> String {
    let text = format!(r#"{{"format_version":1,"storage_id":"{id}"}}"#);
    std::fs::write(root.path.join(MARKER), &text).unwrap();
    text
}

fn refused(result: Result<(), StartupError>) -> Problem {
    match result {
        Err(StartupError::Storage(error)) => *error.problem(),
        other => panic!("expected a storage refusal, found {other:?}"),
    }
}

#[test]
fn decisions_follow_the_five_rules() {
    let id = Uuid::new_v4();
    let other = Uuid::new_v4();
    let invalid = Marker::Invalid(MarkerProblem::Malformed);
    for contents in [Contents::Empty, Contents::MarkerOnly, Contents::Other] {
        assert_eq!(
            decide(Some(id), Marker::Valid(id), contents),
            Action::Continue
        );
        assert_eq!(
            decide(Some(id), Marker::Absent, contents),
            Action::Refuse(Problem::MissingMarker)
        );
        assert_eq!(
            decide(Some(id), invalid, contents),
            Action::Refuse(Problem::InvalidMarker(MarkerProblem::Malformed))
        );
        assert_eq!(
            decide(Some(id), Marker::Valid(other), contents),
            Action::Refuse(Problem::MismatchedMarker)
        );
    }
    assert_eq!(
        decide(None, Marker::Absent, Contents::Empty),
        Action::Create
    );
    assert_eq!(
        decide(None, Marker::Valid(id), Contents::MarkerOnly),
        Action::Adopt(id)
    );
    assert_eq!(
        decide(None, invalid, Contents::MarkerOnly),
        Action::Refuse(Problem::InvalidMarker(MarkerProblem::Malformed))
    );
    for marker in [Marker::Absent, Marker::Valid(id), invalid] {
        assert_eq!(
            decide(None, marker, Contents::Other),
            Action::Refuse(Problem::Occupied)
        );
    }
}

#[tokio::test]
async fn first_start_pairs_and_restart_preserves_both_identities() {
    let db = TestDatabase::new();
    let root = TempRoot::new();
    start(&db, &root).await.unwrap();
    let id = stored_id(&db).await.unwrap();
    assert_eq!(marker_id(&root), id);
    // Only the marker remains: the access probe removed itself.
    let snapshot = root.snapshot();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].0, MARKER);
    start(&db, &root).await.unwrap();
    assert_eq!(stored_id(&db).await, Some(id));
    assert_eq!(snapshot, root.snapshot());
}

#[tokio::test]
async fn database_only_startup_and_readiness_never_touch_storage() {
    let db = TestDatabase::new();
    database_only(&db).await;
    assert_eq!(stored_id(&db).await, None);
    let root = TempRoot::new();
    start(&db, &root).await.unwrap();
    let id = stored_id(&db).await;
    // A paired database still serves database-only health without storage.
    std::fs::remove_file(root.path.join(MARKER)).unwrap();
    let database = initialize(db.options.clone(), SECONDS, SECONDS)
        .await
        .unwrap();
    assert!(database.ready().await);
    database.close().await;
    assert!(root.snapshot().is_empty());
    assert_eq!(stored_id(&db).await, id);
}

#[tokio::test]
async fn marker_only_storage_recovers_with_the_same_identity() {
    let db = TestDatabase::new();
    database_only(&db).await;
    let root = TempRoot::new();
    let id = Uuid::new_v4();
    // A compatible extra field must survive adoption byte for byte.
    let text = format!(r#"{{"format_version":1,"storage_id":"{id}","note":"kept"}}"#);
    std::fs::write(root.path.join(MARKER), &text).unwrap();
    start(&db, &root).await.unwrap();
    assert_eq!(stored_id(&db).await, Some(id));
    assert_eq!(
        std::fs::read_to_string(root.path.join(MARKER)).unwrap(),
        text
    );
}

#[tokio::test]
async fn paired_refusals_create_no_file_and_keep_both_identities() {
    let db = TestDatabase::new();
    let paired = TempRoot::new();
    start(&db, &paired).await.unwrap();
    let id = stored_id(&db).await;

    let empty = TempRoot::new();
    assert_eq!(refused(start(&db, &empty).await), Problem::MissingMarker);
    assert!(empty.snapshot().is_empty());

    let malformed = TempRoot::new();
    std::fs::write(malformed.path.join(MARKER), "{\"format_version\":1,").unwrap();
    let before = malformed.snapshot();
    assert_eq!(
        refused(start(&db, &malformed).await),
        Problem::InvalidMarker(MarkerProblem::Malformed)
    );
    assert_eq!(before, malformed.snapshot());

    let other = TempRoot::new();
    valid_marker(&other, Uuid::new_v4());
    let before = other.snapshot();
    assert_eq!(refused(start(&db, &other).await), Problem::MismatchedMarker);
    assert_eq!(before, other.snapshot());

    assert_eq!(stored_id(&db).await, id);
}

#[tokio::test]
async fn unpaired_refusals_adopt_nothing() {
    for setup in [
        |root: &TempRoot| std::fs::write(root.path.join("image.fits"), "keep").unwrap(),
        |root: &TempRoot| std::fs::create_dir(root.path.join("lost+found")).unwrap(),
        |root: &TempRoot| {
            valid_marker(root, Uuid::new_v4());
            std::fs::write(root.path.join("image.fits"), "keep").unwrap();
        },
        |root: &TempRoot| std::fs::write(root.path.join(".sidereal-probe-leftover"), "").unwrap(),
    ] {
        let db = TestDatabase::new();
        let root = TempRoot::new();
        setup(&root);
        let before = root.snapshot();
        assert_eq!(refused(start(&db, &root).await), Problem::Occupied);
        assert_eq!(before, root.snapshot());
        assert_eq!(stored_id(&db).await, None);
    }
}

#[tokio::test]
async fn interrupted_marker_writes_recover_or_fail_closed() {
    // Complete marker, null UUID: the next start recovers it.
    let db = TestDatabase::new();
    database_only(&db).await;
    let root = TempRoot::new();
    let id = Uuid::new_v4();
    let text = valid_marker(&root, id);
    start(&db, &root).await.unwrap();
    assert_eq!(stored_id(&db).await, Some(id));
    assert_eq!(
        std::fs::read_to_string(root.path.join(MARKER)).unwrap(),
        text
    );

    // Truncated marker, null UUID: refuse and change nothing.
    let db = TestDatabase::new();
    let root = TempRoot::new();
    std::fs::write(root.path.join(MARKER), &text[..text.len() / 2]).unwrap();
    let before = root.snapshot();
    assert_eq!(
        refused(start(&db, &root).await),
        Problem::InvalidMarker(MarkerProblem::Malformed)
    );
    assert_eq!(before, root.snapshot());
    assert_eq!(stored_id(&db).await, None);
}

#[tokio::test]
async fn losing_the_create_race_rereads_instead_of_using_its_own_id() {
    let db = TestDatabase::new();
    database_only(&db).await;
    let root = TempRoot::new();
    let winner = Uuid::new_v4();
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    pair_with(&mut conn, &root.root(), |storage, id| {
        assert_ne!(id, winner);
        // Another process publishes between this start's read and its create.
        valid_marker(&root, winner);
        storage.create_marker(id)
    })
    .await
    .unwrap();
    conn.close().await.unwrap();
    assert_eq!(stored_id(&db).await, Some(winner));
    assert_eq!(marker_id(&root), winner);
}

#[tokio::test]
async fn concurrent_starts_with_one_root_converge() {
    let db = TestDatabase::new();
    let root = TempRoot::new();
    let (first, second) = tokio::join!(start(&db, &root), start(&db, &root));
    first.unwrap();
    second.unwrap();
    assert_eq!(stored_id(&db).await, Some(marker_id(&root)));
    assert_eq!(root.snapshot().len(), 1);
}

#[tokio::test]
async fn concurrent_starts_with_different_roots_cannot_replace_a_pairing() {
    let db = TestDatabase::new();
    let (a, b) = (TempRoot::new(), TempRoot::new());
    let (first, second) = tokio::join!(start(&db, &a), start(&db, &b));
    let (winner, loser, failure) = match (first, second) {
        (Ok(()), failure) => (&a, &b, failure),
        (failure, Ok(())) => (&b, &a, failure),
        other => panic!("one start must pair: {other:?}"),
    };
    assert_eq!(refused(failure), Problem::MissingMarker);
    assert!(loser.snapshot().is_empty());
    assert_eq!(stored_id(&db).await, Some(marker_id(winner)));
}

#[tokio::test]
async fn denied_updates_recover_from_the_marker_only_state() {
    let db = TestDatabase::new();
    database_only(&db).await;
    let mut conn = PgConnection::connect_with(&db.options).await.unwrap();
    // Move ownership so the test role's owner rights cannot mask the revoke.
    conn.execute("ALTER TABLE public.sidereal_metadata OWNER TO pg_database_owner; REVOKE UPDATE ON public.sidereal_metadata FROM pg_database_owner")
        .await
        .unwrap();
    let root = TempRoot::new();
    assert_eq!(
        start(&db, &root).await,
        Err(StartupError::Database(DatabaseError::Permission))
    );
    assert_eq!(stored_id(&db).await, None);
    let id = marker_id(&root);
    assert_eq!(root.snapshot().len(), 1);
    conn.execute("GRANT UPDATE ON public.sidereal_metadata TO pg_database_owner")
        .await
        .unwrap();
    start(&db, &root).await.unwrap();
    assert_eq!(stored_id(&db).await, Some(id));
    assert_eq!(marker_id(&root), id);
    conn.close().await.unwrap();
}

#[tokio::test]
async fn probe_failures_keep_the_committed_pairing() {
    if running_as_root() {
        return;
    }
    let db = TestDatabase::new();
    let root = TempRoot::new();
    start(&db, &root).await.unwrap();
    let id = stored_id(&db).await;
    // A leftover probe in a matching store neither blocks startup nor is removed.
    std::fs::write(root.path.join(".sidereal-probe-leftover"), "keep").unwrap();
    start(&db, &root).await.unwrap();
    let before = root.snapshot();
    assert_eq!(before.len(), 2);
    root.set_mode(0o555);
    assert_eq!(
        refused(start(&db, &root).await),
        Problem::Io(Operation::CreateProbe, io::ErrorKind::PermissionDenied)
    );
    assert_eq!(stored_id(&db).await, id);
    assert_eq!(before, root.snapshot());
    assert_eq!(root.mode() & 0o777, 0o555);
}

#[tokio::test]
async fn unwritable_first_storage_stays_unpaired() {
    if running_as_root() {
        return;
    }
    let db = TestDatabase::new();
    let root = TempRoot::new();
    root.set_mode(0o555);
    assert_eq!(
        refused(start(&db, &root).await),
        Problem::Io(Operation::CreateMarker, io::ErrorKind::PermissionDenied)
    );
    assert!(root.snapshot().is_empty());
    assert_eq!(stored_id(&db).await, None);
}
