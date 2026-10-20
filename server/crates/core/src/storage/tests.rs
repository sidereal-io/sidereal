use super::*;
use std::{
    os::unix::fs::{symlink, PermissionsExt},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A throwaway storage root, removed (after restoring write access) on drop.
pub(crate) struct TempRoot {
    pub path: PathBuf,
}

impl TempRoot {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "sidereal-storage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir(&path).unwrap();
        Self { path }
    }

    pub fn root(&self) -> StorageRoot {
        StorageRoot::new(&self.path).unwrap()
    }

    pub fn marker_path(&self) -> PathBuf {
        self.path.join(MARKER)
    }

    /// Every entry name and file body, for before/after comparisons.
    pub fn snapshot(&self) -> Vec<(String, Option<Vec<u8>>)> {
        let mut entries: Vec<_> = std::fs::read_dir(&self.path)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let body = std::fs::symlink_metadata(entry.path())
                    .unwrap()
                    .is_file()
                    .then(|| std::fs::read(entry.path()).unwrap());
                (entry.file_name().into_string().unwrap(), body)
            })
            .collect();
        entries.sort();
        entries
    }

    pub fn mode(&self) -> u32 {
        std::fs::metadata(&self.path).unwrap().permissions().mode()
    }

    pub fn set_mode(&self, mode: u32) {
        std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(mode)).unwrap();
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o755));
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Root bypasses permission bits, so permission-denial tests cannot run as root.
pub(crate) fn running_as_root() -> bool {
    // SAFETY: geteuid has no preconditions and cannot fail.
    unsafe { libc::geteuid() == 0 }
}

fn marker_json(text: &str, root: &TempRoot) {
    std::fs::write(root.marker_path(), text).unwrap();
}

#[test]
fn roots_must_be_absolute_existing_directories() {
    let temp = TempRoot::new();
    assert_eq!(temp.root().path(), temp.path);
    for (path, problem) in [
        (PathBuf::new(), Problem::NotAbsolute),
        (PathBuf::from("relative/storage"), Problem::NotAbsolute),
        (temp.path.join("absent"), Problem::Missing),
    ] {
        assert_eq!(StorageRoot::new(&path).unwrap_err().problem(), &problem);
    }
    std::fs::write(temp.path.join("file"), "x").unwrap();
    assert_eq!(
        StorageRoot::new(&temp.path.join("file"))
            .unwrap_err()
            .problem(),
        &Problem::NotDirectory
    );
    // Validation never creates the missing directory.
    assert!(!temp.path.join("absent").exists());
}

#[test]
fn contents_distinguish_empty_marker_only_and_other() {
    let temp = TempRoot::new();
    let root = temp.root();
    assert_eq!(root.contents().unwrap(), Contents::Empty);
    marker_json("{}", &temp);
    assert_eq!(root.contents().unwrap(), Contents::MarkerOnly);
    std::fs::write(temp.path.join("other"), "x").unwrap();
    assert_eq!(root.contents().unwrap(), Contents::Other);
    std::fs::remove_file(temp.marker_path()).unwrap();
    assert_eq!(root.contents().unwrap(), Contents::Other);
}

#[test]
fn valid_markers_accept_unknown_fields_without_changing_bytes() {
    let temp = TempRoot::new();
    let root = temp.root();
    assert_eq!(root.marker().unwrap(), Marker::Absent);
    let id = Uuid::new_v4();
    let text = format!(r#"{{"format_version":1,"storage_id":"{id}","future":{{"x":[1]}}}}"#);
    marker_json(&text, &temp);
    assert_eq!(root.marker().unwrap(), Marker::Valid(id));
    assert_eq!(std::fs::read_to_string(temp.marker_path()).unwrap(), text);
    // Exactly the size limit is still accepted.
    let padded = format!(
        r#"{{"format_version":1,"storage_id":"{id}","pad":"{}"}}"#,
        "x".repeat(4096 - 100)
    );
    let padded = format!("{padded}{}", " ".repeat(4096 - padded.len()));
    assert_eq!(padded.len(), 4096);
    marker_json(&padded, &temp);
    assert_eq!(root.marker().unwrap(), Marker::Valid(id));
}

#[test]
fn invalid_markers_are_rejected_and_preserved() {
    let id = Uuid::new_v4();
    for (text, problem) in [
        ("{".to_owned(), MarkerProblem::Malformed),
        (String::new(), MarkerProblem::Malformed),
        ("[]".to_owned(), MarkerProblem::InvalidField),
        (
            format!(r#"{{"storage_id":"{id}"}}"#),
            MarkerProblem::InvalidField,
        ),
        (
            format!(r#"{{"format_version":"1","storage_id":"{id}"}}"#),
            MarkerProblem::InvalidField,
        ),
        (
            format!(r#"{{"format_version":2,"storage_id":"{id}"}}"#),
            MarkerProblem::UnsupportedVersion,
        ),
        (
            r#"{"format_version":1,"storage_id":"not-a-uuid"}"#.to_owned(),
            MarkerProblem::InvalidField,
        ),
        (
            r#"{"format_version":1}"#.to_owned(),
            MarkerProblem::InvalidField,
        ),
        (
            format!(
                r#"{{"format_version":1,"storage_id":"{id}"}}{}"#,
                " ".repeat(4096)
            ),
            MarkerProblem::TooLarge,
        ),
    ] {
        let temp = TempRoot::new();
        marker_json(&text, &temp);
        assert_eq!(temp.root().marker().unwrap(), Marker::Invalid(problem));
        assert_eq!(std::fs::read_to_string(temp.marker_path()).unwrap(), text);
    }
}

#[test]
fn symlinks_and_special_files_are_rejected_without_blocking() {
    let id = Uuid::new_v4();
    // A symlink to a valid marker elsewhere, and a dangling one.
    let temp = TempRoot::new();
    let elsewhere = TempRoot::new();
    let target = elsewhere.path.join("target.json");
    let text = format!(r#"{{"format_version":1,"storage_id":"{id}"}}"#);
    std::fs::write(&target, &text).unwrap();
    symlink(&target, temp.marker_path()).unwrap();
    assert_eq!(
        temp.root().marker().unwrap(),
        Marker::Invalid(MarkerProblem::Symlink)
    );
    assert_eq!(std::fs::read_to_string(&target).unwrap(), text);
    std::fs::remove_file(&target).unwrap();
    assert_eq!(
        temp.root().marker().unwrap(),
        Marker::Invalid(MarkerProblem::Symlink)
    );

    let temp = TempRoot::new();
    std::fs::create_dir(temp.marker_path()).unwrap();
    assert_eq!(
        temp.root().marker().unwrap(),
        Marker::Invalid(MarkerProblem::NotRegular)
    );

    // A FIFO with no writer would block a plain open forever.
    let fifo = TempRoot::new();
    let name =
        std::ffi::CString::new(fifo.marker_path().into_os_string().into_encoded_bytes()).unwrap();
    // SAFETY: `name` is a valid NUL-terminated path for the call's duration.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let socket = TempRoot::new();
    let _listener = std::os::unix::net::UnixListener::bind(socket.marker_path()).unwrap();
    for temp in [fifo, socket] {
        let root = temp.root();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || tx.send(root.marker()).unwrap());
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5))
                .expect("reading a special-file marker blocked")
                .unwrap(),
            Marker::Invalid(MarkerProblem::NotRegular)
        );
    }
}

#[test]
fn unreadable_markers_report_the_read_operation() {
    if running_as_root() {
        return;
    }
    let temp = TempRoot::new();
    marker_json("{}", &temp);
    std::fs::set_permissions(temp.marker_path(), std::fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(
        temp.root().marker().unwrap_err().problem(),
        &Problem::Io(Operation::ReadMarker, io::ErrorKind::PermissionDenied)
    );
}

#[test]
fn marker_creation_is_exclusive_and_synced() {
    let temp = TempRoot::new();
    let root = temp.root();
    let id = Uuid::new_v4();
    assert_eq!(root.create_marker(id).unwrap(), Created::Published);
    assert_eq!(root.marker().unwrap(), Marker::Valid(id));
    assert_eq!(root.contents().unwrap(), Contents::MarkerOnly);
    let before = temp.snapshot();
    // A second creation, as a losing racer would attempt, changes nothing.
    assert_eq!(root.create_marker(Uuid::new_v4()).unwrap(), Created::Exists);
    assert_eq!(before, temp.snapshot());
    root.sync_marker().unwrap();
    assert_eq!(before, temp.snapshot());

    // Invalid or dangling entries are never overwritten either.
    let temp = TempRoot::new();
    marker_json("{", &temp);
    assert_eq!(temp.root().create_marker(id).unwrap(), Created::Exists);
    assert_eq!(std::fs::read_to_string(temp.marker_path()).unwrap(), "{");
    let temp = TempRoot::new();
    symlink(temp.path.join("absent"), temp.marker_path()).unwrap();
    assert_eq!(temp.root().create_marker(id).unwrap(), Created::Exists);
    assert!(!temp.path.join("absent").exists());
}

#[test]
fn write_and_sync_failures_never_report_publication() {
    fn failing(_: &File) -> io::Result<()> {
        Err(io::Error::other("injected sync failure"))
    }
    let temp = TempRoot::new();
    assert_eq!(
        temp.root()
            .publish_marker(Uuid::new_v4(), failing)
            .unwrap_err()
            .problem(),
        &Problem::Io(Operation::SyncMarker, io::ErrorKind::Other)
    );
    // File sync succeeds but directory sync fails: publication and marker-only
    // recovery must both fail, so the database is never paired.
    fn directory_fails(file: &File) -> io::Result<()> {
        if file.metadata()?.is_dir() {
            Err(io::Error::other("injected directory sync failure"))
        } else {
            file.sync_all()
        }
    }
    let temp = TempRoot::new();
    let directory_failure = Problem::Io(Operation::SyncDirectory, io::ErrorKind::Other);
    assert_eq!(
        temp.root()
            .publish_marker(Uuid::new_v4(), directory_fails)
            .unwrap_err()
            .problem(),
        &directory_failure
    );
    assert_eq!(
        temp.root()
            .sync_existing_marker(directory_fails)
            .unwrap_err()
            .problem(),
        &directory_failure
    );
    if running_as_root() {
        return;
    }
    let temp = TempRoot::new();
    temp.set_mode(0o555);
    let error = temp.root().create_marker(Uuid::new_v4()).unwrap_err();
    assert_eq!(
        error.problem(),
        &Problem::Io(Operation::CreateMarker, io::ErrorKind::PermissionDenied)
    );
    assert!(temp.snapshot().is_empty());
    assert_eq!(temp.mode() & 0o777, 0o555);
}

#[test]
fn probes_remove_only_their_own_files() {
    let temp = TempRoot::new();
    let root = temp.root();
    root.probe().unwrap();
    assert!(temp.snapshot().is_empty());
    // Existing files, including a leftover probe, stay exactly as they were.
    let id = Uuid::new_v4();
    root.create_marker(id).unwrap();
    std::fs::write(temp.path.join(".sidereal-probe-leftover"), "keep").unwrap();
    std::fs::write(temp.path.join("image.fits"), "keep").unwrap();
    let before = temp.snapshot();
    root.probe().unwrap();
    root.probe().unwrap();
    assert_eq!(before, temp.snapshot());
}

#[test]
fn probe_failures_change_no_ownership_permissions_or_files() {
    if running_as_root() {
        return;
    }
    let temp = TempRoot::new();
    let root = temp.root();
    root.create_marker(Uuid::new_v4()).unwrap();
    temp.set_mode(0o555);
    let before = temp.snapshot();
    let owner = std::os::unix::fs::MetadataExt::uid(&std::fs::metadata(&temp.path).unwrap());
    let error = root.probe().unwrap_err();
    assert_eq!(
        error.problem(),
        &Problem::Io(Operation::CreateProbe, io::ErrorKind::PermissionDenied)
    );
    assert!(error.to_string().contains("create an access probe file"));
    assert_eq!(before, temp.snapshot());
    assert_eq!(temp.mode() & 0o777, 0o555);
    assert_eq!(
        owner,
        std::os::unix::fs::MetadataExt::uid(&std::fs::metadata(&temp.path).unwrap())
    );
}

#[test]
fn diagnostics_escape_paths_and_omit_marker_bodies() {
    let path = Path::new("/var/lib/sidereal/storage\nforged: log line\u{1b}[31m");
    let rendered = StorageError::new(path, Problem::MissingMarker).to_string();
    assert!(!rendered.contains('\n') && !rendered.contains('\u{1b}'));
    assert!(rendered.contains(r#""/var/lib/sidereal/storage\nforged: log line\u{1b}[31m""#));
    // Missing markers on paired stores direct mount recovery, not reinitialization.
    assert!(rendered.contains("storage marker is missing"));
    assert!(rendered.contains("mount the storage directory that belongs to this database"));

    let secret = "distinctive-marker-body-secret";
    for body in [
        format!(r#"{{"format_version":9,"storage_id":"{secret}"}}"#),
        format!("{{{secret}"),
    ] {
        let temp = TempRoot::new();
        marker_json(&body, &temp);
        let Marker::Invalid(problem) = temp.root().marker().unwrap() else {
            panic!("marker should be invalid");
        };
        let rendered = temp
            .root()
            .error(Problem::InvalidMarker(problem))
            .to_string();
        assert!(rendered.contains(&format!("{:?}", temp.path)));
        assert!(!rendered.contains(secret));
    }

    let operations = [
        Operation::InspectRoot,
        Operation::ListRoot,
        Operation::ReadMarker,
        Operation::CreateMarker,
        Operation::WriteMarker,
        Operation::SyncMarker,
        Operation::SyncDirectory,
        Operation::CreateProbe,
        Operation::WriteProbe,
        Operation::SyncProbe,
        Operation::DeleteProbe,
    ];
    for operation in operations {
        let rendered = StorageError::new(
            Path::new("/storage"),
            Problem::Io(operation, io::ErrorKind::PermissionDenied),
        )
        .to_string();
        assert!(rendered.starts_with(r#"storage "/storage": cannot "#));
        assert!(rendered.contains(&operation.to_string()));
        assert!(rendered.contains("restart"));
    }
    for problem in [
        Problem::NotAbsolute,
        Problem::Missing,
        Problem::NotDirectory,
        Problem::MismatchedMarker,
        Problem::Occupied,
        Problem::Contended,
    ] {
        let rendered = StorageError::new(Path::new("/storage"), problem).to_string();
        assert!(rendered.starts_with(r#"storage "/storage": "#));
        // Every diagnostic carries a recovery action after the problem.
        assert!(rendered.contains("; "));
    }
}
