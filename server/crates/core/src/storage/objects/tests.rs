use super::*;
use crate::storage::{tests::TempRoot, Problem, MARKER};
use std::os::unix::fs::symlink;

/// A root with a valid marker for `id`, as pairing leaves it.
fn marked(id: Uuid) -> TempRoot {
    let temp = TempRoot::new();
    assert_eq!(
        temp.root().create_marker(id),
        Ok(crate::storage::Created::Published)
    );
    temp
}

fn store(temp: &TempRoot) -> (ObjectStore, Uuid) {
    let id = Uuid::new_v4();
    assert_eq!(
        temp.root().create_marker(id),
        Ok(crate::storage::Created::Published)
    );
    (ObjectStore::open(&temp.root(), id).unwrap(), id)
}

fn names(path: &Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn staged(store: &ObjectStore, bytes: &[u8]) -> Staged {
    let mut staging = store.begin().unwrap();
    staging.write(bytes).unwrap();
    staging.finish().unwrap()
}

fn problem(result: Result<impl std::fmt::Debug, StorageError>) -> Problem {
    *result.unwrap_err().problem()
}

mod ingest_staging {
    use super::*;
    use crate::{database::initialize_with_storage, test_support::TestDatabase};
    use std::time::Duration;

    const SECONDS: Duration = Duration::from_secs(3);

    #[tokio::test]
    async fn a_newly_paired_root_holds_only_its_marker() {
        let db = TestDatabase::new();
        let temp = TempRoot::new();
        let (database, id) =
            initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &temp.root())
                .await
                .unwrap();
        database.close().await;
        ObjectStore::open(&temp.root(), id).unwrap();
        assert_eq!(names(&temp.path), [MARKER]);
    }

    #[test]
    fn the_first_staging_creates_tmp() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let _staged = staged(&store, b"abc");
        assert_eq!(names(&temp.path), [MARKER, "tmp"]);
    }

    #[test]
    fn a_staged_candidate_is_one_file_in_tmp_with_a_generated_name() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let _staged = staged(&store, b"abc");
        let entries = names(&temp.path.join("tmp"));
        assert_eq!(entries.len(), 1);
        assert!(Uuid::try_parse(&entries[0]).is_ok(), "{entries:?}");
        assert_eq!(
            std::fs::read(temp.path.join("tmp").join(&entries[0])).unwrap(),
            b"abc"
        );
    }

    #[test]
    fn staging_reports_the_blake3_hash_and_size_of_what_it_wrote() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let staged = staged(&store, b"abc");
        assert_eq!(
            staged.hash(),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
        assert_eq!(staged.size(), 3);
    }

    #[test]
    fn staging_accepts_chunks_written_one_after_another() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let mut staging = store.begin().unwrap();
        staging.write(b"a").unwrap();
        staging.write(b"bc").unwrap();
        let staged = staging.finish().unwrap();
        assert_eq!(
            staged.hash(),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
    }

    #[test]
    fn a_staged_candidate_reads_back_through_read_at() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let mut staged = staged(&store, b"abcdef");
        let mut buf = [0; 3];
        assert_eq!(staged.read_at(2, &mut buf).unwrap(), 3);
        assert_eq!(&buf, b"cde");
    }

    #[test]
    fn a_dropped_staging_file_is_removed() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        drop(staged(&store, b"abc"));
        let mut staging = store.begin().unwrap();
        staging.write(b"abc").unwrap();
        drop(staging);
        assert!(names(&temp.path.join("tmp")).is_empty());
    }

    #[test]
    fn exactly_100_mib_can_be_staged() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let mut staging = store.begin().unwrap();
        let chunk = vec![0x5a; 1024 * 1024];
        for _ in 0..100 {
            staging.write(&chunk).unwrap();
        }
        assert_eq!(staging.finish().unwrap().size(), 104_857_600);
    }

    #[test]
    fn one_byte_over_100_mib_is_refused() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let mut staging = store.begin().unwrap();
        let chunk = vec![0x5a; 1024 * 1024];
        for _ in 0..100 {
            staging.write(&chunk).unwrap();
        }
        assert!(matches!(staging.write(b"x"), Err(StageError::TooLarge)));
    }

    #[test]
    fn a_symlinked_storage_root_resolves_to_its_target() {
        let temp = TempRoot::new();
        std::fs::create_dir(temp.path.join("real")).unwrap();
        symlink(temp.path.join("real"), temp.path.join("link")).unwrap();
        let root = StorageRoot::new(&temp.path.join("link")).unwrap();
        assert_eq!(root.path(), temp.path.join("real"));
    }

    #[tokio::test]
    async fn a_symlinked_storage_root_pairs_and_stages_under_its_target() {
        let db = TestDatabase::new();
        let temp = TempRoot::new();
        let (real, link) = (temp.path.join("real"), temp.path.join("link"));
        std::fs::create_dir(&real).unwrap();
        symlink(&real, &link).unwrap();
        let root = StorageRoot::new(&link).unwrap();
        let (database, id) = initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &root)
            .await
            .unwrap();
        database.close().await;
        let store = ObjectStore::open(&root, id).unwrap();
        let _staged = staged(&store, b"abc");
        assert_eq!(names(&real), [MARKER, "tmp"]);
        assert!(std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[tokio::test]
    async fn a_root_replaced_after_pairing_is_refused() {
        let db = TestDatabase::new();
        let temp = TempRoot::new();
        let (current, moved) = (temp.path.join("storage"), temp.path.join("moved"));
        std::fs::create_dir(&current).unwrap();
        let root = StorageRoot::new(&current).unwrap();
        let (database, id) = initialize_with_storage(db.options.clone(), SECONDS, SECONDS, &root)
            .await
            .unwrap();
        database.close().await;
        std::fs::rename(&current, &moved).unwrap();
        std::fs::create_dir(&current).unwrap();
        StorageRoot::new(&current)
            .unwrap()
            .create_marker(Uuid::new_v4())
            .unwrap();
        let error = ObjectStore::open(&root, id).unwrap_err();
        assert_eq!(error.problem(), &Problem::Replaced);
        assert!(error.to_string().contains("STORAGE_ROOT"), "{error}");
        assert_eq!(names(&current), [MARKER]);
        assert_eq!(names(&moved), [MARKER]);
    }

    #[test]
    fn a_root_without_its_marker_is_refused() {
        let temp = TempRoot::new();
        assert_eq!(
            problem(ObjectStore::open(&temp.root(), Uuid::new_v4())),
            Problem::Replaced
        );
        assert!(names(&temp.path).is_empty());
    }

    #[test]
    fn a_symlinked_tmp_is_refused_without_touching_its_target() {
        let temp = marked(Uuid::nil());
        let outside = TempRoot::new();
        symlink(&outside.path, temp.path.join("tmp")).unwrap();
        let store = ObjectStore::open(&temp.root(), Uuid::nil()).unwrap();
        assert_eq!(problem(store.begin()), Problem::Unsafe(Entry::Tmp));
        assert!(names(&outside.path).is_empty());
    }

    #[test]
    fn a_tmp_that_is_a_file_is_refused() {
        let temp = marked(Uuid::nil());
        std::fs::write(temp.path.join("tmp"), b"keep").unwrap();
        let store = ObjectStore::open(&temp.root(), Uuid::nil()).unwrap();
        assert_eq!(problem(store.begin()), Problem::Unsafe(Entry::Tmp));
        assert_eq!(std::fs::read(temp.path.join("tmp")).unwrap(), b"keep");
    }

    #[test]
    fn an_existing_staging_entry_is_not_followed_or_overwritten() {
        let temp = marked(Uuid::nil());
        let outside = TempRoot::new();
        std::fs::write(outside.path.join("target"), b"keep").unwrap();
        std::fs::create_dir(temp.path.join("tmp")).unwrap();
        let name = Uuid::new_v4().to_string();
        symlink(
            outside.path.join("target"),
            temp.path.join("tmp").join(&name),
        )
        .unwrap();
        let store = ObjectStore::open(&temp.root(), Uuid::nil()).unwrap();
        assert_eq!(
            problem(store.begin_named(&name)),
            Problem::Unsafe(Entry::Staging)
        );
        assert_eq!(std::fs::read(outside.path.join("target")).unwrap(), b"keep");
        assert!(std::fs::symlink_metadata(temp.path.join("tmp").join(&name))
            .unwrap()
            .file_type()
            .is_symlink());
    }
}
