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
    ffi::{CStr, CString, OsStr, OsString},
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
        unix::{ffi::OsStrExt, fs::FileExt},
    },
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

/// The largest file one upload may hold: 100 MiB.
pub const MAX_UPLOAD_BYTES: u64 = 104_857_600;

const TMP: &str = "tmp";
const STORE: &str = "store";

/// One filesystem step that a test can fail or pause.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    WriteStaging,
    SyncStaging,
    Publish,
    SyncObject,
    SyncDirectory(Level),
}

/// A folder in the chain that publication syncs, deepest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    /// `store/{h[0:2]}/{h[2:4]}`, which holds the object.
    SecondPrefix,
    /// `store/{h[0:2]}`.
    FirstPrefix,
    Store,
    Root,
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
                published: false,
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
    /// Set once the file has been renamed into the store.
    published: bool,
}

impl Drop for Temporary {
    fn drop(&mut self) {
        if self.published {
            return;
        }
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

/// What publication did with a staged candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Published {
    pub hash: String,
    pub size: u64,
    /// A matching object was already stored, and this ingest reused it.
    pub reused: bool,
}

impl ObjectStore {
    /// Make a staged candidate a durable object at
    /// `store/{h[0:2]}/{h[2:4]}/{h}`, never replacing an existing entry.
    ///
    /// The staged file is synced before it is published. An existing object
    /// is reused only when its size and BLAKE3 hash match. Every call then
    /// syncs the whole folder chain itself, deepest first, whoever created
    /// it. An object is never removed, even when a later step fails.
    pub fn publish(&self, mut staged: Staged) -> Result<Published, StorageError> {
        let hash = staged.hash.clone();
        let size = staged.size;
        self.step(Step::SyncStaging)
            .and_then(|()| staged.temporary.file.sync_all())
            .map_err(self.io(Operation::SyncStaging))?;
        let store = self.folder(self.root(), STORE, Entry::Store)?;
        let first = self.folder(store.as_fd(), &hash[..2], Entry::Prefix)?;
        let second = self.folder(first.as_fd(), &hash[2..4], Entry::Prefix)?;
        if device(staged.temporary.tmp.as_fd()).map_err(self.io(Operation::OpenDirectory))?
            != device(second.as_fd()).map_err(self.io(Operation::OpenDirectory))?
        {
            return Err(self.error(Problem::CrossDevice));
        }
        let renamed = self.step(Step::Publish).and_then(|()| {
            rename_no_replace(
                staged.temporary.tmp.as_fd(),
                &staged.temporary.name,
                second.as_fd(),
                &hash,
            )
        });
        let reused = match renamed {
            Ok(()) => {
                staged.temporary.published = true;
                false
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                self.reuse(second.as_fd(), &hash, size)?;
                true
            }
            Err(e)
                if matches!(
                    e.raw_os_error(),
                    Some(libc::EINVAL | libc::ENOSYS | libc::ENOTSUP)
                ) =>
            {
                return Err(self.error(Problem::NoReplaceUnsupported))
            }
            Err(e) if e.raw_os_error() == Some(libc::EXDEV) => {
                return Err(self.error(Problem::CrossDevice))
            }
            Err(e) => return Err(self.io(Operation::PublishObject)(e)),
        };
        drop(staged);
        for (level, folder) in [
            (Level::SecondPrefix, second.as_fd()),
            (Level::FirstPrefix, first.as_fd()),
            (Level::Store, store.as_fd()),
            (Level::Root, self.root()),
        ] {
            self.step(Step::SyncDirectory(level))
                .and_then(|()| sync_folder(folder))
                .map_err(self.io(Operation::SyncDirectory))?;
        }
        Ok(Published { hash, size, reused })
    }

    /// Check that the object already at `hash` holds exactly the candidate's
    /// bytes, then sync it. Its size is compared before any byte is read, and
    /// the read stops one byte past the candidate's size.
    fn reuse(&self, folder: BorrowedFd<'_>, hash: &str, size: u64) -> Result<(), StorageError> {
        let unsafe_object = || self.error(Problem::Unsafe(Entry::Object));
        let found = stat_at(folder, hash).map_err(self.io(Operation::ReadObject))?;
        if found.st_mode & libc::S_IFMT != libc::S_IFREG {
            return Err(unsafe_object());
        }
        if found.st_size as u64 != size {
            return Err(self.error(Problem::ObjectMismatch));
        }
        let mut file = open_file_at(folder, hash).map_err(|e| {
            if is_refused_entry(&e) {
                unsafe_object()
            } else {
                self.io(Operation::ReadObject)(e)
            }
        })?;
        let opened = file.metadata().map_err(self.io(Operation::ReadObject))?;
        {
            use std::os::unix::fs::MetadataExt;
            if !opened.is_file() || opened.ino() != found.st_ino as u64 || opened.len() != size {
                return Err(unsafe_object());
            }
        }
        let mut hasher = blake3::Hasher::new();
        let read = io::copy(&mut (&mut file).take(size + 1), &mut hasher)
            .map_err(self.io(Operation::ReadObject))?;
        if read != size || hasher.finalize().to_hex().as_str() != hash {
            return Err(self.error(Problem::ObjectMismatch));
        }
        self.step(Step::SyncObject)
            .and_then(|()| file.sync_all())
            .map_err(self.io(Operation::SyncObject))
    }
}

impl ObjectStore {
    /// Open the stored object for `hash` to read it, answering the open file
    /// and its size. No symbolic link is followed and nothing other than a
    /// regular file is opened, so a FIFO is refused at once instead of
    /// blocking. The bytes are not hashed again.
    pub fn open_object(&self, hash: &str) -> Result<(File, u64), StorageError> {
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(self.error(Problem::Unsafe(Entry::Object)));
        }
        let refused = |entry| {
            move |e: io::Error| {
                if e.kind() != io::ErrorKind::NotFound && is_refused_entry(&e) {
                    self.error(Problem::Unsafe(entry))
                } else {
                    self.io(Operation::ReadObject)(e)
                }
            }
        };
        let store = open_folder_at(self.root(), STORE).map_err(refused(Entry::Store))?;
        let first = open_folder_at(store.as_fd(), &hash[..2]).map_err(refused(Entry::Prefix))?;
        let second = open_folder_at(first.as_fd(), &hash[2..4]).map_err(refused(Entry::Prefix))?;
        let found = stat_at(second.as_fd(), hash).map_err(self.io(Operation::ReadObject))?;
        if found.st_mode & libc::S_IFMT != libc::S_IFREG {
            return Err(self.error(Problem::Unsafe(Entry::Object)));
        }
        let file = open_file_at(second.as_fd(), hash).map_err(refused(Entry::Object))?;
        let opened = file.metadata().map_err(self.io(Operation::ReadObject))?;
        {
            use std::os::unix::fs::MetadataExt;
            if !opened.is_file() || opened.ino() != found.st_ino as u64 {
                return Err(self.error(Problem::Unsafe(Entry::Object)));
            }
        }
        Ok((file, opened.len()))
    }
}

/// What startup cleanup did with the entries left in `tmp`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cleanup {
    /// How many entries it removed.
    pub removed: usize,
    /// The subfolders it left, by their raw names.
    pub kept: Vec<OsString>,
}

impl ObjectStore {
    /// Remove the staging files that an interrupted ingest left in `tmp`.
    /// Creates nothing: a root without `tmp` is left as it is.
    pub fn remove_leftover_staging(&self) -> Result<Cleanup, StorageError> {
        let tmp = match open_folder_at(self.root(), TMP) {
            Ok(tmp) => tmp,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Cleanup::default()),
            // A link or a file named `tmp` is never followed or removed.
            Err(e) if is_refused_entry(&e) => return Err(self.error(Problem::Unsafe(Entry::Tmp))),
            Err(e) => return Err(self.io(Operation::OpenDirectory)(e)),
        };
        let mut cleanup = Cleanup::default();
        for name in list(tmp.as_fd()).map_err(self.io(Operation::ListTmp))? {
            // Inspected without following a link or opening the entry, so a
            // FIFO cannot block and a link's target is never touched.
            let found = stat_raw(tmp.as_fd(), &name).map_err(self.io(Operation::RemoveStaging))?;
            if found.st_mode & libc::S_IFMT == libc::S_IFDIR {
                cleanup
                    .kept
                    .push(OsStr::from_bytes(name.to_bytes()).to_owned());
                continue;
            }
            unlink_at(tmp.as_fd(), &name).map_err(self.io(Operation::RemoveStaging))?;
            cleanup.removed += 1;
        }
        Ok(cleanup)
    }
}

/// The names in an open folder, except `.` and `..`, as raw bytes.
fn list(folder: BorrowedFd<'_>) -> io::Result<Vec<CString>> {
    // `fdopendir` takes ownership of its descriptor, so it gets a duplicate.
    // SAFETY: `folder` is an open descriptor.
    let fd = unsafe { libc::fcntl(folder.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `fd` is an open directory descriptor that nothing else owns.
    let dir = unsafe { libc::fdopendir(fd) };
    if dir.is_null() {
        let error = io::Error::last_os_error();
        // SAFETY: `fdopendir` failed, so `fd` is still ours to close.
        unsafe { libc::close(fd) };
        return Err(error);
    }
    // A duplicate shares the file offset, so start from the first entry.
    // SAFETY: `dir` is an open directory stream.
    unsafe { libc::rewinddir(dir) };
    let mut names = Vec::new();
    let result = loop {
        clear_errno();
        // SAFETY: `dir` is an open directory stream.
        let entry = unsafe { libc::readdir(dir) };
        if entry.is_null() {
            let error = io::Error::last_os_error();
            break match error.raw_os_error() {
                Some(0) => Ok(()),
                _ => Err(error),
            };
        }
        // SAFETY: `readdir` returned an entry whose name is NUL-terminated
        // and valid until the next call on `dir`.
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name != c"." && name != c".." {
            names.push(name.to_owned());
        }
    };
    // SAFETY: `dir` is open, and closing it also closes `fd`.
    unsafe { libc::closedir(dir) };
    result.map(|()| names)
}

/// Reset `errno`, so a null `readdir` tells the end apart from an error.
fn clear_errno() {
    // SAFETY: errno is thread-local and writable.
    #[cfg(target_os = "linux")]
    unsafe {
        *libc::__errno_location() = 0;
    }
    // SAFETY: errno is thread-local and writable.
    #[cfg(target_os = "macos")]
    unsafe {
        *libc::__error() = 0;
    }
}

/// Remove one entry that is not a folder, without following a link.
fn unlink_at(folder: BorrowedFd<'_>, name: &CStr) -> io::Result<()> {
    // SAFETY: `folder` is an open directory and `name` is NUL-terminated.
    if unsafe { libc::unlinkat(folder.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn device(fd: BorrowedFd<'_>) -> io::Result<libc::dev_t> {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: `fd` is open and `stat` has room for one `struct stat`.
    if unsafe { libc::fstat(fd.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fstat succeeded and filled `stat`.
    Ok(unsafe { stat.assume_init() }.st_dev)
}

/// Inspect an entry without following a symbolic link or opening it.
fn stat_at(folder: BorrowedFd<'_>, name: &str) -> io::Result<libc::stat> {
    stat_raw(folder, &name_cstring(name))
}

/// [`stat_at`] for a name read from a folder, which may not be UTF-8.
fn stat_raw(folder: BorrowedFd<'_>, c: &CStr) -> io::Result<libc::stat> {
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: `folder` is an open directory, `c` NUL-terminated, and `stat`
    // has room for one `struct stat`.
    if unsafe {
        libc::fstatat(
            folder.as_raw_fd(),
            c.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fstatat succeeded and filled `stat`.
    Ok(unsafe { stat.assume_init() })
}

fn sync_folder(folder: BorrowedFd<'_>) -> io::Result<()> {
    // SAFETY: `folder` is an open directory descriptor.
    if unsafe { libc::fsync(folder.as_raw_fd()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Rename that fails with `AlreadyExists` instead of replacing an entry.
#[cfg(target_os = "linux")]
fn rename_no_replace(
    from_folder: BorrowedFd<'_>,
    from: &str,
    to_folder: BorrowedFd<'_>,
    to: &str,
) -> io::Result<()> {
    let (from, to) = (name_cstring(from), name_cstring(to));
    // SAFETY: both folders are open directories and both names NUL-terminated.
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            from_folder.as_raw_fd(),
            from.as_ptr(),
            to_folder.as_raw_fd(),
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Rename that fails with `AlreadyExists` instead of replacing an entry.
/// A filesystem without exclusive rename answers `ENOTSUP`.
#[cfg(target_os = "macos")]
fn rename_no_replace(
    from_folder: BorrowedFd<'_>,
    from: &str,
    to_folder: BorrowedFd<'_>,
    to: &str,
) -> io::Result<()> {
    let (from, to) = (name_cstring(from), name_cstring(to));
    // SAFETY: both folders are open directories and both names NUL-terminated.
    let result = unsafe {
        libc::renameatx_np(
            from_folder.as_raw_fd(),
            from.as_ptr(),
            to_folder.as_raw_fd(),
            to.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// No supported no-replace rename here: publication fails closed.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn rename_no_replace(_: BorrowedFd<'_>, _: &str, _: BorrowedFd<'_>, _: &str) -> io::Result<()> {
    Err(io::Error::from_raw_os_error(libc::ENOSYS))
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
