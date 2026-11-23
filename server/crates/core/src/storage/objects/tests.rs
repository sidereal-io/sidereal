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

/// Where an object with this hash lives under `root`.
fn object_path(root: &Path, hash: &str) -> PathBuf {
    root.join("store")
        .join(&hash[..2])
        .join(&hash[2..4])
        .join(hash)
}

/// A store whose hook records every step it runs.
fn recording(temp: &TempRoot, id: Uuid) -> (ObjectStore, Arc<std::sync::Mutex<Vec<Step>>>) {
    let steps = Arc::new(std::sync::Mutex::new(Vec::new()));
    let log = steps.clone();
    let hook: Hook = Arc::new(move |step| {
        log.lock().unwrap().push(step);
        Ok(())
    });
    (
        ObjectStore::open_with(&temp.root(), id, Some(hook)).unwrap(),
        steps,
    )
}

/// A store whose hook fails `failing` and runs every other step.
fn failing_at(temp: &TempRoot, id: Uuid, failing: Step) -> ObjectStore {
    let hook: Hook = Arc::new(move |step| {
        if step == failing {
            Err(io::Error::other("injected"))
        } else {
            Ok(())
        }
    });
    ObjectStore::open_with(&temp.root(), id, Some(hook)).unwrap()
}

fn store_files(temp: &TempRoot) -> Vec<PathBuf> {
    fn walk(path: &Path, files: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                walk(&entry.path(), files);
            } else {
                files.push(entry.path());
            }
        }
    }
    let mut files = Vec::new();
    walk(&temp.path.join("store"), &mut files);
    files.sort();
    files
}

/// Size, modification time and bytes, to show a file did not change.
fn fingerprint(path: &Path) -> (u64, std::time::SystemTime, Vec<u8>) {
    let metadata = std::fs::metadata(path).unwrap();
    (
        metadata.len(),
        metadata.modified().unwrap(),
        std::fs::read(path).unwrap(),
    )
}

mod object_publication {
    use super::*;
    use std::{
        os::unix::fs::{FileTypeExt, MetadataExt},
        sync::{mpsc, Barrier},
        time::{Duration, Instant},
    };

    const ABC: &str = "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

    #[test]
    fn a_published_object_lives_at_its_blake3_path_with_identical_bytes() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        store.publish(staged(&store, b"abc")).unwrap();
        assert_eq!(std::fs::read(object_path(&temp.path, ABC)).unwrap(), b"abc");
    }

    #[test]
    fn the_first_publication_creates_store() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        store.publish(staged(&store, b"abc")).unwrap();
        assert_eq!(names(&temp.path), [MARKER, "store", "tmp"]);
    }

    #[test]
    fn publication_leaves_no_staging_file() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        store.publish(staged(&store, b"abc")).unwrap();
        assert!(names(&temp.path.join("tmp")).is_empty());
    }

    #[test]
    fn publication_syncs_the_file_then_the_folder_chain_deepest_first() {
        let temp = TempRoot::new();
        let id = Uuid::new_v4();
        temp.root().create_marker(id).unwrap();
        let (store, steps) = recording(&temp, id);
        let staged = staged(&store, b"abc");
        steps.lock().unwrap().clear();
        store.publish(staged).unwrap();
        assert_eq!(
            *steps.lock().unwrap(),
            [
                Step::SyncStaging,
                Step::Publish,
                Step::SyncDirectory(Level::SecondPrefix),
                Step::SyncDirectory(Level::FirstPrefix),
                Step::SyncDirectory(Level::Store),
                Step::SyncDirectory(Level::Root),
            ]
        );
    }

    /// Two inputs whose hashes share their first four hex digits.
    fn same_prefix() -> (Vec<u8>, Vec<u8>) {
        let first = b"0".to_vec();
        let prefix = blake3::hash(&first).to_hex()[..4].to_owned();
        let second = (1u64..)
            .map(|i| i.to_string().into_bytes())
            .find(|bytes| blake3::hash(bytes).to_hex()[..4] == prefix)
            .unwrap();
        (first, second)
    }

    #[test]
    fn a_second_hash_under_a_new_prefix_syncs_the_chain_itself() {
        let temp = TempRoot::new();
        let id = Uuid::new_v4();
        temp.root().create_marker(id).unwrap();
        let (a_bytes, b_bytes) = same_prefix();
        let (paused_tx, paused_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let release_rx = std::sync::Mutex::new(release_rx);
        let hook: Hook = Arc::new(move |step| {
            if step == Step::SyncDirectory(Level::SecondPrefix) {
                paused_tx.send(()).unwrap();
                release_rx.lock().unwrap().recv().unwrap();
            }
            Ok(())
        });
        let a = ObjectStore::open_with(&temp.root(), id, Some(hook)).unwrap();
        let a_staged = staged(&a, &a_bytes);
        let publisher = std::thread::spawn(move || a.publish(a_staged));
        paused_rx.recv().unwrap();

        let (b, steps) = recording(&temp, id);
        let b_staged = staged(&b, &b_bytes);
        steps.lock().unwrap().clear();
        b.publish(b_staged).unwrap();
        let directories: Vec<_> = steps
            .lock()
            .unwrap()
            .iter()
            .copied()
            .filter(|step| matches!(step, Step::SyncDirectory(_)))
            .collect();
        assert_eq!(
            directories,
            [
                Step::SyncDirectory(Level::SecondPrefix),
                Step::SyncDirectory(Level::FirstPrefix),
                Step::SyncDirectory(Level::Store),
                Step::SyncDirectory(Level::Root),
            ]
        );
        release_tx.send(()).unwrap();
        publisher.join().unwrap().unwrap();
    }

    #[test]
    fn matching_bytes_published_again_reuse_the_object_unchanged() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        store.publish(staged(&store, b"abc")).unwrap();
        let path = object_path(&temp.path, ABC);
        let (inode, before) = (std::fs::metadata(&path).unwrap().ino(), fingerprint(&path));
        let published = store.publish(staged(&store, b"abc")).unwrap();
        assert!(published.reused);
        assert_eq!(std::fs::metadata(&path).unwrap().ino(), inode);
        assert_eq!(fingerprint(&path), before);
        assert_eq!(store_files(&temp), [path]);
        assert!(names(&temp.path.join("tmp")).is_empty());
    }

    #[test]
    fn concurrent_matching_publications_keep_one_complete_object() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let bytes: Vec<u8> = (0..4 * 1024 * 1024).map(|i| (i % 251) as u8).collect();
        let barrier = Arc::new(Barrier::new(4));
        let publishers: Vec<_> = (0..4)
            .map(|_| {
                let (store, bytes, barrier) = (store.clone(), bytes.clone(), barrier.clone());
                std::thread::spawn(move || {
                    let staged = staged(&store, &bytes);
                    barrier.wait();
                    store.publish(staged).map(|p| p.hash)
                })
            })
            .collect();
        let hashes: Vec<_> = publishers
            .into_iter()
            .map(|p| p.join().unwrap().unwrap())
            .collect();
        assert!(hashes.iter().all(|h| *h == hashes[0]));
        let files = store_files(&temp);
        assert_eq!(files, [object_path(&temp.path, &hashes[0])]);
        assert_eq!(std::fs::read(&files[0]).unwrap(), bytes);
    }

    #[test]
    fn a_same_size_object_with_changed_bytes_is_refused_unchanged() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let path = object_path(&temp.path, ABC);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"xyz").unwrap();
        let before = fingerprint(&path);
        assert_eq!(
            problem(store.publish(staged(&store, b"abc"))),
            Problem::ObjectMismatch
        );
        assert_eq!(fingerprint(&path), before);
    }

    #[test]
    fn a_much_larger_object_is_refused_without_reading_it() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let path = object_path(&temp.path, ABC);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Sparse, so it costs no disk; reading it would take far too long.
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(1 << 40).unwrap();
        drop(file);
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        let start = Instant::now();
        assert_eq!(
            problem(store.publish(staged(&store, b"abc"))),
            Problem::ObjectMismatch
        );
        assert!(start.elapsed() < Duration::from_secs(5));
        let metadata = std::fs::metadata(&path).unwrap();
        assert_eq!(
            (metadata.len(), metadata.modified().unwrap()),
            (1 << 40, modified)
        );
    }

    #[test]
    fn a_symlink_at_the_object_path_is_refused_without_touching_its_target() {
        let temp = TempRoot::new();
        let outside = TempRoot::new();
        std::fs::write(outside.path.join("target"), b"abc").unwrap();
        let (store, _) = store(&temp);
        let path = object_path(&temp.path, ABC);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        symlink(outside.path.join("target"), &path).unwrap();
        let before = fingerprint(&outside.path.join("target"));
        assert_eq!(
            problem(store.publish(staged(&store, b"abc"))),
            Problem::Unsafe(Entry::Object)
        );
        assert_eq!(fingerprint(&outside.path.join("target")), before);
        assert!(std::fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn a_symlinked_store_is_refused_without_touching_its_target() {
        let temp = TempRoot::new();
        let outside = TempRoot::new();
        let (store, _) = store(&temp);
        symlink(&outside.path, temp.path.join("store")).unwrap();
        assert_eq!(
            problem(store.publish(staged(&store, b"abc"))),
            Problem::Unsafe(Entry::Store)
        );
        assert!(names(&outside.path).is_empty());
    }

    #[test]
    fn a_symlinked_prefix_folder_is_refused_without_touching_its_target() {
        let temp = TempRoot::new();
        let outside = TempRoot::new();
        let (store, _) = store(&temp);
        std::fs::create_dir_all(temp.path.join("store").join(&ABC[..2])).unwrap();
        symlink(
            &outside.path,
            temp.path.join("store").join(&ABC[..2]).join(&ABC[2..4]),
        )
        .unwrap();
        assert_eq!(
            problem(store.publish(staged(&store, b"abc"))),
            Problem::Unsafe(Entry::Prefix)
        );
        assert!(names(&outside.path).is_empty());
    }

    #[test]
    fn a_folder_at_the_object_path_is_refused() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        std::fs::create_dir_all(object_path(&temp.path, ABC)).unwrap();
        assert_eq!(
            problem(store.publish(staged(&store, b"abc"))),
            Problem::Unsafe(Entry::Object)
        );
    }

    #[test]
    fn a_fifo_with_no_writer_at_the_object_path_is_refused_promptly() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let path = object_path(&temp.path, ABC);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let c = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: `c` is a NUL-terminated path.
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o644) }, 0);
        let staged = staged(&store, b"abc");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || tx.send(store.publish(staged).map(|_| ())));
        let outcome = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("publication blocked");
        assert_eq!(problem(outcome), Problem::Unsafe(Entry::Object));
        assert!(std::fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_fifo());
    }

    #[test]
    fn a_failed_file_sync_publishes_nothing() {
        let temp = TempRoot::new();
        let id = Uuid::new_v4();
        temp.root().create_marker(id).unwrap();
        let store = failing_at(&temp, id, Step::SyncStaging);
        assert_eq!(
            problem(store.publish(staged(&store, b"abc"))),
            Problem::Io(Operation::SyncStaging, io::ErrorKind::Other)
        );
        assert!(store_files(&temp).is_empty());
        assert!(names(&temp.path.join("tmp")).is_empty());
    }

    #[test]
    fn a_failed_publish_step_publishes_nothing() {
        let temp = TempRoot::new();
        let id = Uuid::new_v4();
        temp.root().create_marker(id).unwrap();
        let store = failing_at(&temp, id, Step::Publish);
        assert_eq!(
            problem(store.publish(staged(&store, b"abc"))),
            Problem::Io(Operation::PublishObject, io::ErrorKind::Other)
        );
        assert!(store_files(&temp).is_empty());
    }

    #[test]
    fn a_failed_folder_sync_fails_publication_and_keeps_the_object() {
        let temp = TempRoot::new();
        let id = Uuid::new_v4();
        temp.root().create_marker(id).unwrap();
        let store = failing_at(&temp, id, Step::SyncDirectory(Level::Store));
        assert_eq!(
            problem(store.publish(staged(&store, b"abc"))),
            Problem::Io(Operation::SyncDirectory, io::ErrorKind::Other)
        );
        assert_eq!(std::fs::read(object_path(&temp.path, ABC)).unwrap(), b"abc");
    }

    #[test]
    fn a_failed_reuse_sync_fails_publication_and_keeps_the_object() {
        let temp = TempRoot::new();
        let id = Uuid::new_v4();
        temp.root().create_marker(id).unwrap();
        let (store, _) = recording(&temp, id);
        store.publish(staged(&store, b"abc")).unwrap();
        let store = failing_at(&temp, id, Step::SyncObject);
        assert_eq!(
            problem(store.publish(staged(&store, b"abc"))),
            Problem::Io(Operation::SyncObject, io::ErrorKind::Other)
        );
        assert_eq!(std::fs::read(object_path(&temp.path, ABC)).unwrap(), b"abc");
    }
}

mod staging_cleanup {
    use super::*;

    /// A paired root whose `tmp` holds the regular files named `files`.
    fn leftovers(files: &[&str]) -> (TempRoot, ObjectStore) {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        std::fs::create_dir(temp.path.join("tmp")).unwrap();
        for name in files {
            std::fs::write(temp.path.join("tmp").join(name), b"left").unwrap();
        }
        (temp, store)
    }

    #[test]
    fn leftover_staging_files_are_removed() {
        let (temp, store) = leftovers(&["a", "b", "c"]);
        store.remove_leftover_staging().unwrap();
        assert!(names(&temp.path.join("tmp")).is_empty());
    }

    #[test]
    fn cleanup_reports_how_many_files_it_removed() {
        let (_temp, store) = leftovers(&["a", "b", "c"]);
        assert_eq!(store.remove_leftover_staging().unwrap().removed, 3);
    }

    #[test]
    fn cleanup_leaves_the_marker_and_store_unchanged() {
        let (temp, store) = leftovers(&["a"]);
        store.publish(staged(&store, b"abc")).unwrap();
        let objects = store_files(&temp);
        let marker = std::fs::read(temp.path.join(MARKER)).unwrap();
        store.remove_leftover_staging().unwrap();
        assert_eq!(store_files(&temp), objects);
        assert_eq!(std::fs::read(temp.path.join(MARKER)).unwrap(), marker);
    }

    #[test]
    fn a_root_without_tmp_reports_nothing_removed() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        assert_eq!(store.remove_leftover_staging().unwrap(), Cleanup::default());
    }

    #[test]
    fn a_root_without_tmp_is_left_without_tmp() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        store.remove_leftover_staging().unwrap();
        assert_eq!(names(&temp.path), [MARKER]);
    }

    #[test]
    fn a_symlink_in_tmp_is_removed() {
        let (temp, store) = leftovers(&[]);
        let outside = TempRoot::new();
        std::fs::write(outside.path.join("target"), b"keep").unwrap();
        symlink(
            outside.path.join("target"),
            temp.path.join("tmp").join("link"),
        )
        .unwrap();
        store.remove_leftover_staging().unwrap();
        assert!(names(&temp.path.join("tmp")).is_empty());
    }

    #[test]
    fn a_symlink_target_outside_the_root_keeps_its_bytes() {
        let (temp, store) = leftovers(&[]);
        let outside = TempRoot::new();
        std::fs::write(outside.path.join("target"), b"keep").unwrap();
        symlink(
            outside.path.join("target"),
            temp.path.join("tmp").join("link"),
        )
        .unwrap();
        let before = fingerprint(&outside.path.join("target"));
        store.remove_leftover_staging().unwrap();
        assert_eq!(fingerprint(&outside.path.join("target")), before);
    }

    #[test]
    fn a_fifo_with_no_writer_in_tmp_is_removed_promptly() {
        let (temp, store) = leftovers(&[]);
        let fifo = temp.path.join("tmp").join("fifo");
        let c = CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: `c` is a NUL-terminated path.
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o644) }, 0);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || tx.send(store.remove_leftover_staging()));
        let cleanup = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("cleanup blocked")
            .unwrap();
        assert_eq!(cleanup.removed, 1);
        assert!(names(&temp.path.join("tmp")).is_empty());
    }

    #[test]
    fn a_subfolder_of_tmp_is_left_with_its_contents() {
        let (temp, store) = leftovers(&["a"]);
        std::fs::create_dir(temp.path.join("tmp").join("kept")).unwrap();
        std::fs::write(temp.path.join("tmp").join("kept").join("inside"), b"keep").unwrap();
        store.remove_leftover_staging().unwrap();
        assert_eq!(
            std::fs::read(temp.path.join("tmp").join("kept").join("inside")).unwrap(),
            b"keep"
        );
    }

    #[test]
    fn a_left_subfolder_is_not_counted_as_removed() {
        let (temp, store) = leftovers(&["a"]);
        std::fs::create_dir(temp.path.join("tmp").join("kept")).unwrap();
        assert_eq!(store.remove_leftover_staging().unwrap().removed, 1);
    }

    #[test]
    fn cleanup_names_each_subfolder_it_left() {
        let (temp, store) = leftovers(&[]);
        std::fs::create_dir(temp.path.join("tmp").join("kept")).unwrap();
        assert_eq!(
            store.remove_leftover_staging().unwrap().kept,
            [std::ffi::OsString::from("kept")]
        );
    }

    #[test]
    fn a_subfolder_name_is_reported_as_its_raw_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let (temp, store) = leftovers(&[]);
        let name = std::ffi::OsStr::from_bytes(b"k\xffept");
        std::fs::create_dir(temp.path.join("tmp").join(name)).unwrap();
        assert_eq!(store.remove_leftover_staging().unwrap().kept, [name]);
    }

    #[test]
    fn a_symlinked_tmp_is_refused() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let outside = TempRoot::new();
        symlink(&outside.path, temp.path.join("tmp")).unwrap();
        assert_eq!(
            problem(store.remove_leftover_staging()),
            Problem::Unsafe(Entry::Tmp)
        );
    }

    #[test]
    fn a_symlinked_tmp_and_its_target_are_left_unchanged() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        let outside = TempRoot::new();
        std::fs::write(outside.path.join("file"), b"keep").unwrap();
        symlink(&outside.path, temp.path.join("tmp")).unwrap();
        let _ = store.remove_leftover_staging();
        assert_eq!(
            std::fs::read_link(temp.path.join("tmp")).unwrap(),
            outside.path
        );
        assert_eq!(std::fs::read(outside.path.join("file")).unwrap(), b"keep");
    }

    #[test]
    fn a_tmp_that_is_a_file_is_refused_unchanged() {
        let temp = TempRoot::new();
        let (store, _) = store(&temp);
        std::fs::write(temp.path.join("tmp"), b"keep").unwrap();
        assert_eq!(
            problem(store.remove_leftover_staging()),
            Problem::Unsafe(Entry::Tmp)
        );
        assert_eq!(std::fs::read(temp.path.join("tmp")).unwrap(), b"keep");
    }
}
