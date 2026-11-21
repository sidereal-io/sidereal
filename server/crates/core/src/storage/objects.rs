//! The object store: staged ingest and content-addressed originals inside a
//! paired storage root (ADR-011).
//!
//! The store opens the resolved root once and keeps that open folder. Every
//! later operation works relative to it, never through a path that a
//! symbolic link could redirect. Below the root no symbolic link is followed:
//! `tmp`, `store`, each prefix folder and each file are opened with
//! `O_NOFOLLOW` and checked for their type.
//!
//! Names come only from core: staging files are named by a fresh UUID and
//! objects by their BLAKE3 hash. A display filename never becomes a path.
use super::{Entry, Operation, Problem, StorageError, StorageRoot, MARKER, MARKER_LIMIT};
use std::{
    ffi::CString,
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
        unix::fs::FileExt,
    },
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

/// The largest file one upload may hold: 100 MiB.
pub const MAX_UPLOAD_BYTES: u64 = 104_857_600;

const TMP: &str = "tmp";

/// One filesystem step that a test can fail or pause.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    WriteStaging,
}

/// Runs before each [`Step`]; an error fails that step.
pub(crate) type Hook = Arc<dyn Fn(Step) -> io::Result<()> + Send + Sync>;

/// Why staging stopped.
#[derive(Debug)]
pub enum StageError {
    /// The candidate is larger than [`MAX_UPLOAD_BYTES`].
    TooLarge,
    Storage(StorageError),
}

impl From<StorageError> for StageError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

struct Inner {
    root: OwnedFd,
    path: PathBuf,
    hook: Option<Hook>,
}

/// The object store of one paired storage root. Cloning shares it.
#[derive(Clone)]
pub struct ObjectStore {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for ObjectStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObjectStore")
            .field("path", &self.inner.path)
            .finish_non_exhaustive()
    }
}

impl ObjectStore {
    /// Open the root that pairing checked, and confirm through that open
    /// folder that it still holds the marker for `paired`. Creates nothing.
    pub fn open(root: &StorageRoot, paired: Uuid) -> Result<Self, StorageError> {
        Self::open_with(root, paired, None)
    }

    pub(crate) fn open_with(
        root: &StorageRoot,
        paired: Uuid,
        hook: Option<Hook>,
    ) -> Result<Self, StorageError> {
        let path = root.path().to_owned();
        let error = |problem| StorageError::new(&path, problem);
        // The root is already resolved, so it is opened without O_NOFOLLOW.
        let fd = open_raw(
            None,
            &path_cstring(&path),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            0,
        )
        .map_err(|e| error(Problem::Io(Operation::OpenRoot, e.kind())))?;
        let store = Self {
            inner: Arc::new(Inner {
                root: fd,
                path: path.clone(),
                hook,
            }),
        };
        match store.marker() {
            Ok(Some(id)) if id == paired => Ok(store),
            Ok(_) => Err(error(Problem::Replaced)),
            Err(e) => Err(error(Problem::Io(Operation::ReadMarker, e.kind()))),
        }
    }

    /// The marker's UUID, read through the open root. `None` when it is
    /// absent, not a regular file, or not a valid marker.
    fn marker(&self) -> io::Result<Option<Uuid>> {
        let mut file = match open_file_at(self.root(), MARKER) {
            Ok(file) => file,
            Err(e) if is_refused_entry(&e) => return Ok(None),
            Err(e) => return Err(e),
        };
        if !file.metadata()?.is_file() {
            return Ok(None);
        }
        let mut bytes = Vec::new();
        (&mut file).take(MARKER_LIMIT + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MARKER_LIMIT {
            return Ok(None);
        }
        Ok(super::parse_marker(&bytes).ok())
    }

    fn root(&self) -> BorrowedFd<'_> {
        self.inner.root.as_fd()
    }

    fn error(&self, problem: Problem) -> StorageError {
        StorageError::new(&self.inner.path, problem)
    }

    fn io(&self, operation: Operation) -> impl Fn(io::Error) -> StorageError + '_ {
        move |e| self.error(Problem::Io(operation, e.kind()))
    }

    fn step(&self, step: Step) -> io::Result<()> {
        match &self.inner.hook {
            Some(hook) => hook(step),
            None => Ok(()),
        }
    }

    /// Open a managed folder below `parent`, creating it when absent.
    fn folder(
        &self,
        parent: BorrowedFd<'_>,
        name: &str,
        entry: Entry,
    ) -> Result<OwnedFd, StorageError> {
        let c = name_cstring(name);
        // SAFETY: `parent` is an open directory and `c` a NUL-terminated name.
        if unsafe { libc::mkdirat(parent.as_raw_fd(), c.as_ptr(), 0o755) } != 0 {
            let e = io::Error::last_os_error();
            if e.kind() != io::ErrorKind::AlreadyExists {
                return Err(self.io(Operation::OpenDirectory)(e));
            }
        }
        open_folder_at(parent, name).map_err(|e| {
            if is_refused_entry(&e) {
                self.error(Problem::Unsafe(entry))
            } else {
                self.io(Operation::OpenDirectory)(e)
            }
        })
    }

    /// Start staging one candidate in a new, exclusively created file.
    pub fn begin(&self) -> Result<Staging, StorageError> {
        self.begin_named(&Uuid::new_v4().to_string())
    }

    /// [`ObjectStore::begin`] with a chosen name, so a test can collide.
    pub(crate) fn begin_named(&self, name: &str) -> Result<Staging, StorageError> {
        let tmp = self.folder(self.root(), TMP, Entry::Tmp)?;
        let file = open_raw(
            Some(tmp.as_fd()),
            &name_cstring(name),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o644,
        )
        .map_err(|e| {
            if e.kind() == io::ErrorKind::AlreadyExists || is_refused_entry(&e) {
                self.error(Problem::Unsafe(Entry::Staging))
            } else {
                self.io(Operation::CreateStaging)(e)
            }
        })?;
        Ok(Staging {
            file: Some(Temporary {
                store: self.clone(),
                tmp,
                name: name.to_owned(),
                file: File::from(file),
            }),
            size: 0,
        })
    }
}

/// A staging file this request created. Dropping it removes the file.
struct Temporary {
    store: ObjectStore,
    tmp: OwnedFd,
    name: String,
    file: File,
}

impl Drop for Temporary {
    fn drop(&mut self) {
        let c = name_cstring(&self.name);
        // SAFETY: `tmp` is an open directory and `c` a NUL-terminated name.
        // Only the name this request created exclusively is removed.
        unsafe { libc::unlinkat(self.tmp.as_raw_fd(), c.as_ptr(), 0) };
    }
}

/// A candidate being written into staging, in bounded chunks.
pub struct Staging {
    file: Option<Temporary>,
    size: u64,
}

impl std::fmt::Debug for Staging {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Staging")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

impl Staging {
    fn temporary(&mut self) -> &mut Temporary {
        self.file.as_mut().expect("staging is open until finished")
    }

    /// Append one chunk. Refuses, without writing it, a chunk that would make
    /// the candidate larger than [`MAX_UPLOAD_BYTES`].
    pub fn write(&mut self, chunk: &[u8]) -> Result<(), StageError> {
        if self.size + chunk.len() as u64 > MAX_UPLOAD_BYTES {
            return Err(StageError::TooLarge);
        }
        let temporary = self.temporary();
        let store = temporary.store.clone();
        store
            .step(Step::WriteStaging)
            .and_then(|()| temporary.file.write_all(chunk))
            .map_err(store.io(Operation::WriteStaging))?;
        self.size += chunk.len() as u64;
        Ok(())
    }

    /// The bytes written so far.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Read the staged file back to compute its BLAKE3 hash and size.
    pub fn finish(mut self) -> Result<Staged, StorageError> {
        let temporary = self.file.take().expect("staging is open until finished");
        let mut hasher = blake3::Hasher::new();
        let mut buf = vec![0; 64 * 1024];
        let mut offset = 0;
        loop {
            let n = temporary
                .file
                .read_at(&mut buf, offset)
                .map_err(temporary.store.io(Operation::ReadStaging))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            offset += n as u64;
        }
        if offset != self.size {
            return Err(temporary.store.io(Operation::ReadStaging)(io::Error::from(
                io::ErrorKind::UnexpectedEof,
            )));
        }
        Ok(Staged {
            hash: hasher.finalize().to_hex().to_string(),
            size: offset,
            temporary,
        })
    }
}

/// A complete staged candidate with its hash. Dropping it removes the file.
pub struct Staged {
    temporary: Temporary,
    hash: String,
    size: u64,
}

impl std::fmt::Debug for Staged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Staged")
            .field("hash", &self.hash)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

impl Staged {
    /// The lowercase hexadecimal BLAKE3 hash of the staged bytes.
    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    /// Read staged bytes from `offset`, for a Source's context.
    pub fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        self.temporary.file.read_at(buf, offset)
    }
}

fn path_cstring(path: &Path) -> CString {
    use std::os::unix::ffi::OsStrExt;
    CString::new(path.as_os_str().as_bytes()).expect("a resolved path holds no NUL byte")
}

/// Names are core-generated UUIDs, hashes or fixed folder names.
fn name_cstring(name: &str) -> CString {
    debug_assert!(!name.contains('/') && !name.is_empty());
    CString::new(name).expect("a managed name holds no NUL byte")
}

fn open_raw(
    dir: Option<BorrowedFd<'_>>,
    name: &CString,
    flags: i32,
    mode: u32,
) -> io::Result<OwnedFd> {
    let dir = dir.map_or(libc::AT_FDCWD, |d| d.as_raw_fd());
    // SAFETY: `dir` is AT_FDCWD or an open directory, `name` is NUL-terminated.
    let fd = unsafe { libc::openat(dir, name.as_ptr(), flags, mode as libc::c_uint) };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: `openat` returned a new descriptor that nothing else owns.
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }
}

fn open_folder_at(parent: BorrowedFd<'_>, name: &str) -> io::Result<OwnedFd> {
    open_raw(
        Some(parent),
        &name_cstring(name),
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        0,
    )
}

/// Open a file for reading without following a symbolic link or blocking on
/// a FIFO with no writer.
fn open_file_at(parent: BorrowedFd<'_>, name: &str) -> io::Result<File> {
    open_raw(
        Some(parent),
        &name_cstring(name),
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        0,
    )
    .map(File::from)
}

/// An entry that `O_NOFOLLOW`, `O_DIRECTORY` or a special file refused.
fn is_refused_entry(error: &io::Error) -> bool {
    matches!(
        error.raw_os_error(),
        Some(libc::ELOOP | libc::ENOTDIR | libc::ENXIO | libc::ENODEV)
    ) || error.kind() == io::ErrorKind::NotFound
}

#[cfg(test)]
mod tests;
