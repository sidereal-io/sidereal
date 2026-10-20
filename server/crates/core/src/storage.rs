//! Managed storage roots: root validation, the identity marker, and access
//! probes. This module owns filesystem policy only; the database module pairs
//! a root with the database. Marker contents never select paths, commands, or
//! SQL, and the server never creates, chowns, or chmods a storage root.
use serde_json::Value;
use std::{
    fmt,
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

/// The marker's file name at the storage root.
pub const MARKER: &str = ".sidereal-storage.json";
const MARKER_LIMIT: u64 = 4096;

/// A validated, container-visible storage root. It existed as a directory when
/// validated; mount changes after startup are outside this check.
#[derive(Clone, Debug)]
pub struct StorageRoot {
    path: PathBuf,
}

impl StorageRoot {
    /// Accept only an absolute path to an existing directory. Never creates it.
    pub fn new(path: &Path) -> Result<Self, StorageError> {
        let error = |problem| StorageError::new(path, problem);
        if !path.is_absolute() {
            return Err(error(Problem::NotAbsolute));
        }
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => Ok(Self {
                path: path.to_owned(),
            }),
            Ok(_) => Err(error(Problem::NotDirectory)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(error(Problem::Missing)),
            Err(e) => Err(error(Problem::Io(Operation::InspectRoot, e.kind()))),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn error(&self, problem: Problem) -> StorageError {
        StorageError::new(&self.path, problem)
    }

    fn io(&self, operation: Operation) -> impl Fn(io::Error) -> StorageError + '_ {
        move |e| self.error(Problem::Io(operation, e.kind()))
    }
}

/// What the root holds besides `.` and `..`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Contents {
    Empty,
    MarkerOnly,
    Other,
}

/// The marker as read from disk. Its body is never retained or displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Marker {
    Absent,
    Valid(Uuid),
    Invalid(MarkerProblem),
}

impl StorageRoot {
    pub(crate) fn contents(&self) -> Result<Contents, StorageError> {
        let mut entries = std::fs::read_dir(&self.path).map_err(self.io(Operation::ListRoot))?;
        let Some(first) = entries.next() else {
            return Ok(Contents::Empty);
        };
        let first = first.map_err(self.io(Operation::ListRoot))?;
        if first.file_name() == MARKER && entries.next().is_none() {
            Ok(Contents::MarkerOnly)
        } else {
            Ok(Contents::Other)
        }
    }

    /// Read the marker without following a symlink or blocking on a special file.
    pub(crate) fn marker(&self) -> Result<Marker, StorageError> {
        let path = self.path.join(MARKER);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Ok(Marker::Invalid(MarkerProblem::Symlink))
            }
            Ok(metadata) if !metadata.is_file() => {
                return Ok(Marker::Invalid(MarkerProblem::NotRegular))
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Marker::Absent),
            Err(e) => return Err(self.io(Operation::ReadMarker)(e)),
        }
        // The file can change between the check above and this open, so the
        // open refuses symlinks and special-file blocking, and fstat rechecks.
        let mut file = match open_marker(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Marker::Absent),
            Err(e) if is_symlink_error(&e) => return Ok(Marker::Invalid(MarkerProblem::Symlink)),
            Err(e) if is_special_file_error(&e) => {
                return Ok(Marker::Invalid(MarkerProblem::NotRegular))
            }
            Err(e) => return Err(self.io(Operation::ReadMarker)(e)),
        };
        let metadata = file.metadata().map_err(self.io(Operation::ReadMarker))?;
        if !metadata.is_file() {
            return Ok(Marker::Invalid(MarkerProblem::NotRegular));
        }
        if metadata.len() > MARKER_LIMIT {
            return Ok(Marker::Invalid(MarkerProblem::TooLarge));
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(MARKER_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(self.io(Operation::ReadMarker))?;
        if bytes.len() as u64 > MARKER_LIMIT {
            return Ok(Marker::Invalid(MarkerProblem::TooLarge));
        }
        Ok(match parse_marker(&bytes) {
            Ok(id) => Marker::Valid(id),
            Err(problem) => Marker::Invalid(problem),
        })
    }
}

/// The result of exclusive marker creation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Created {
    /// This call wrote and synced the marker and its directory entry.
    Published,
    /// A marker already existed; its bytes were not touched.
    Exists,
}

impl StorageRoot {
    /// Create, write, and sync a new marker without ever replacing one.
    /// Any error leaves the database unpaired; a partial marker fails closed.
    pub(crate) fn create_marker(&self, id: Uuid) -> Result<Created, StorageError> {
        self.publish_marker(id, File::sync_all)
    }

    fn publish_marker(
        &self,
        id: Uuid,
        sync: fn(&File) -> io::Result<()>,
    ) -> Result<Created, StorageError> {
        let mut file = match create_exclusive(&self.path.join(MARKER)) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return Ok(Created::Exists),
            Err(e) => return Err(self.io(Operation::CreateMarker)(e)),
        };
        file.write_all(&marker_bytes(id))
            .map_err(self.io(Operation::WriteMarker))?;
        sync(&file).map_err(self.io(Operation::SyncMarker))?;
        self.sync_directory(sync)?;
        Ok(Created::Published)
    }

    /// Make an existing marker and its directory entry durable before the
    /// database records its identity.
    pub(crate) fn sync_marker(&self) -> Result<(), StorageError> {
        self.sync_existing_marker(File::sync_all)
    }

    fn sync_existing_marker(&self, sync: fn(&File) -> io::Result<()>) -> Result<(), StorageError> {
        let file = open_marker(&self.path.join(MARKER)).map_err(self.io(Operation::SyncMarker))?;
        sync(&file).map_err(self.io(Operation::SyncMarker))?;
        self.sync_directory(sync)
    }

    fn sync_directory(&self, sync: fn(&File) -> io::Result<()>) -> Result<(), StorageError> {
        let directory = File::open(&self.path).map_err(self.io(Operation::SyncDirectory))?;
        sync(&directory).map_err(self.io(Operation::SyncDirectory))
    }
}

const PROBE_PREFIX: &str = ".sidereal-probe-";

impl StorageRoot {
    /// Check create, write, sync, and delete access with a uniquely named,
    /// exclusively created probe. Only the probe this call created is removed.
    pub(crate) fn probe(&self) -> Result<(), StorageError> {
        let path = self.path.join(format!("{PROBE_PREFIX}{}", Uuid::new_v4()));
        let mut file = create_exclusive(&path).map_err(self.io(Operation::CreateProbe))?;
        let written = file
            .write_all(b"sidereal storage access probe\n")
            .map_err(self.io(Operation::WriteProbe))
            .and_then(|()| file.sync_all().map_err(self.io(Operation::SyncProbe)));
        drop(file);
        if let Err(error) = written {
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
        std::fs::remove_file(&path).map_err(self.io(Operation::DeleteProbe))?;
        self.sync_directory(File::sync_all)
    }
}

/// Unknown fields are ignored so later formats can add them compatibly.
fn parse_marker(bytes: &[u8]) -> Result<Uuid, MarkerProblem> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| MarkerProblem::Malformed)?;
    let object = value.as_object().ok_or(MarkerProblem::InvalidField)?;
    match object.get("format_version").and_then(Value::as_i64) {
        Some(1) => {}
        Some(_) => return Err(MarkerProblem::UnsupportedVersion),
        None => return Err(MarkerProblem::InvalidField),
    }
    object
        .get("storage_id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::try_parse(id).ok())
        .ok_or(MarkerProblem::InvalidField)
}

fn marker_bytes(id: Uuid) -> Vec<u8> {
    let mut bytes = serde_json::json!({ "format_version": 1, "storage_id": id.to_string() })
        .to_string()
        .into_bytes();
    bytes.push(b'\n');
    bytes
}

/// `create_new` fails on any existing entry, including a dangling symlink.
fn create_exclusive(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o644).custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path)
}

#[cfg(unix)]
fn open_marker(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
}
#[cfg(not(unix))]
fn open_marker(path: &Path) -> io::Result<File> {
    OpenOptions::new().read(true).open(path)
}

#[cfg(unix)]
fn is_symlink_error(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ELOOP)
}
#[cfg(not(unix))]
fn is_symlink_error(_: &io::Error) -> bool {
    false
}

/// Opening a socket fails with ENXIO; treat it like any other special file.
#[cfg(unix)]
fn is_special_file_error(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(libc::ENXIO | libc::ENODEV))
}
#[cfg(not(unix))]
fn is_special_file_error(_: &io::Error) -> bool {
    false
}

/// A storage failure with the container-visible root it concerns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageError {
    path: PathBuf,
    problem: Problem,
}

impl StorageError {
    fn new(path: &Path, problem: Problem) -> Self {
        Self {
            path: path.to_owned(),
            problem,
        }
    }

    pub fn problem(&self) -> &Problem {
        &self.problem
    }
}

/// Why storage is unusable. Displays carry no marker bodies or credentials.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    NotAbsolute,
    Missing,
    NotDirectory,
    Io(Operation, io::ErrorKind),
    MissingMarker,
    InvalidMarker(MarkerProblem),
    MismatchedMarker,
    Occupied,
    Contended,
}

/// The filesystem step that failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    InspectRoot,
    ListRoot,
    ReadMarker,
    CreateMarker,
    WriteMarker,
    SyncMarker,
    SyncDirectory,
    CreateProbe,
    WriteProbe,
    SyncProbe,
    DeleteProbe,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerProblem {
    Symlink,
    NotRegular,
    TooLarge,
    Malformed,
    UnsupportedVersion,
    InvalidField,
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InspectRoot => "inspect the storage directory",
            Self::ListRoot => "list the storage directory",
            Self::ReadMarker => "read the storage marker",
            Self::CreateMarker => "create the storage marker",
            Self::WriteMarker => "write the storage marker",
            Self::SyncMarker => "sync the storage marker",
            Self::SyncDirectory => "sync the storage directory",
            Self::CreateProbe => "create an access probe file",
            Self::WriteProbe => "write an access probe file",
            Self::SyncProbe => "sync an access probe file",
            Self::DeleteProbe => "delete an access probe file",
        })
    }
}

impl fmt::Display for MarkerProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Symlink => "it is a symbolic link",
            Self::NotRegular => "it is not a regular file",
            Self::TooLarge => "it is larger than 4096 bytes",
            Self::Malformed => "it is not valid JSON",
            Self::UnsupportedVersion => "its format_version is not supported",
            Self::InvalidField => "format_version or storage_id is missing or invalid",
        })
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Debug formatting quotes the path and escapes control characters, so a
        // crafted directory name cannot forge log lines.
        write!(f, "storage {:?}: ", self.path)?;
        match self.problem {
            Problem::NotAbsolute => f.write_str(
                "STORAGE_ROOT must be an absolute container path; set it to the directory where storage is mounted, or unset it to run without managed storage",
            ),
            Problem::Missing => f.write_str(
                "storage directory does not exist; mount the storage directory at this path (the server never creates it)",
            ),
            Problem::NotDirectory => f.write_str(
                "storage path is not a directory; mount a storage directory at this path",
            ),
            Problem::Io(operation, kind) => {
                write!(f, "cannot {operation} ({kind}); ")?;
                f.write_str(match operation {
                    Operation::SyncMarker | Operation::SyncDirectory | Operation::SyncProbe => "use storage that supports reliable file and directory sync, then restart",
                    _ => "grant the server's user access to this directory, then restart; ownership and permissions were not changed",
                })
            }
            Problem::MissingMarker => f.write_str(
                "storage marker is missing, but the database is paired with managed storage; mount the storage directory that belongs to this database at this path, then restart; do not create a new marker or reinitialize storage",
            ),
            Problem::InvalidMarker(problem) => write!(
                f,
                "storage marker is invalid: {problem}; restore the original marker or mount the storage that belongs to this database; startup never replaces a marker",
            ),
            Problem::MismatchedMarker => f.write_str(
                "storage marker belongs to different storage than this database; mount the storage that belongs to this database; startup never replaces either identity",
            ),
            Problem::Occupied => f.write_str(
                "storage directory holds other files, and the database is not paired with storage yet; select an empty dedicated directory, or restore the database paired with this storage; existing files were not changed",
            ),
            Problem::Contended => f.write_str(
                "storage changed while startup was pairing it; stop other servers that use this directory, then restart",
            ),
        }
    }
}

impl std::error::Error for StorageError {}

#[cfg(test)]
pub(crate) mod tests;
