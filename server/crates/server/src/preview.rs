//! Runs the image pack's preview job in the `sidereal-preview-worker`
//! helper, one at a time (ADR-020).
//!
//! A run takes the one preview slot, then a supervisor thread starts its
//! deadline. The runner copies the candidate to an anonymous file in 64 KiB
//! chunks, starts the helper with that file as its standard input, and
//! passes what the helper writes on through the Asset context. The
//! supervisor owns the helper, its liveness pipe and the slot. On the
//! deadline, a cancellation or a failed write it kills and reaps the helper,
//! even while the runner is blocked in a write, and then frees the slot.
//! The first of those causes decides the run's reason.
use sidereal_pack_image::preview::{JobFailure, PreviewRunner};
use sidereal_plugin_abi::{AssetContext, ContextError, OperatorError};
use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    future::Future,
    io::{self, Read, Seek, SeekFrom, Write},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::{fs::OpenOptionsExt, process::CommandExt},
    },
    path::{Path, PathBuf},
    pin::pin,
    process::{Child, ChildStdout, Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex, MutexGuard,
    },
    task::{Context, Poll, Wake, Waker},
    time::{Duration, Instant},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const TIMED_OUT: &str = "the preview decoder timed out";
const STOPPED: &str = "the preview decoder stopped";
const UNREADABLE: &str = "the stored file could not be read";
const CANCELLED: &str = "the run was cancelled";

/// How much of the candidate one copy step reads, and of the helper's
/// output one read takes.
const CHUNK: usize = 64 * 1024;

/// How often the runner looks for a cancellation while it waits for output,
/// and the supervisor for the helper's exit.
const TICK: Duration = Duration::from_millis(20);

/// The helper's file descriptor for the liveness pipe's read end.
const LIVENESS: i32 = 3;

/// The helper's program and its arguments before the media type.
#[derive(Clone, Debug)]
pub struct WorkerCommand {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

impl WorkerCommand {
    /// The helper's file name.
    pub const HELPER: &'static str = "sidereal-preview-worker";

    /// The helper in the same folder as `server`, the server's executable.
    pub fn beside(server: &Path) -> Self {
        Self {
            program: server.with_file_name(Self::HELPER),
            args: Vec::new(),
        }
    }

    /// The helper beside the running executable.
    pub fn beside_current_exe() -> io::Result<Self> {
        Ok(Self::beside(&std::env::current_exe()?))
    }
}

/// Runs previews in the helper, one at a time across every caller.
pub struct WorkerRunner {
    command: WorkerCommand,
    deadline: Duration,
    slot: Arc<Semaphore>,
    /// A slot permit an upload took before it asked core to run the preview.
    admitted: Mutex<Option<OwnedSemaphorePermit>>,
    /// How many runs have begun copying their input.
    copies: AtomicUsize,
}

/// The slot an upload holds for the preview run it starts next. Dropping it
/// frees a slot no run took.
pub struct Admission<'a>(&'a WorkerRunner);

impl Drop for Admission<'_> {
    fn drop(&mut self) {
        lock(&self.0.admitted).take();
    }
}

impl WorkerRunner {
    /// How long a run may take, from when it takes the slot: two minutes.
    pub const DEADLINE: Duration = Duration::from_secs(120);

    pub fn new(command: WorkerCommand) -> Self {
        Self {
            command,
            deadline: Self::DEADLINE,
            slot: Arc::new(Semaphore::new(1)),
            admitted: Mutex::new(None),
            copies: AtomicUsize::new(0),
        }
    }

    /// The runner with another deadline, for tests.
    pub fn with_deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    /// Wait, without holding a thread, for the preview slot, and keep it for
    /// the next run.
    pub async fn admit(&self) -> Admission<'_> {
        let permit = self
            .slot
            .clone()
            .acquire_owned()
            .await
            .expect("the slot is never closed");
        *lock(&self.admitted) = Some(permit);
        Admission(self)
    }

    /// How many runs have begun copying their input. A run begins its copy
    /// only once it holds the slot.
    pub fn copies_begun(&self) -> usize {
        self.copies.load(Ordering::SeqCst)
    }

    /// The slot an upload took for this run, or else the slot once it is
    /// free.
    fn take_slot(&self) -> OwnedSemaphorePermit {
        if let Some(permit) = lock(&self.admitted).take() {
            return permit;
        }
        block_on(self.slot.clone().acquire_owned()).expect("the slot is never closed")
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Wait on this thread for `future`, which needs no runtime.
fn block_on<F: Future>(future: F) -> F::Output {
    struct Unpark(std::thread::Thread);
    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        std::thread::park();
    }
}

/// What stopped a run before its helper finished on its own. The first
/// cause is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cause {
    Deadline,
    Cancelled,
    /// The Asset context refused a write.
    Output,
}

/// How the helper ended.
#[derive(Clone, Copy, Debug)]
enum Exit {
    Status(ExitStatus),
    /// The supervisor killed it, or could not tell how it ended.
    Killed,
}

#[derive(Default)]
struct State {
    cause: Option<Cause>,
    /// The running helper and its liveness pipe's write end, once launched.
    child: Option<(Child, OwnedFd)>,
    exit: Option<Exit>,
    /// The runner is done with the run.
    finished: bool,
}

/// What the runner and the supervisor share for one run.
#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

impl Shared {
    fn state(&self) -> MutexGuard<'_, State> {
        lock(&self.state)
    }

    /// Record `cause` unless an earlier one stopped the run.
    fn stop(&self, cause: Cause) {
        self.state().cause.get_or_insert(cause);
        self.changed.notify_all();
    }

    fn cause(&self) -> Option<Cause> {
        self.state().cause
    }

    fn finish(&self) {
        self.state().finished = true;
        self.changed.notify_all();
    }

    /// Wait until the supervisor has reaped the helper.
    fn wait_for_exit(&self) {
        let mut state = self.state();
        while state.exit.is_none() {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}

/// The supervisor: start the deadline, then kill and reap the helper on the
/// deadline or another cause, or reap it when it exits. It frees the slot
/// only after reaping, or at the deadline when no helper was launched.
fn supervise(shared: Arc<Shared>, slot: OwnedSemaphorePermit, deadline: Duration) {
    let deadline = Instant::now() + deadline;
    let mut state = shared.state();
    loop {
        if state.cause.is_none() && Instant::now() >= deadline {
            state.cause = Some(Cause::Deadline);
        }
        let stopping = state.cause.is_some();
        if let Some((child, _)) = state.child.as_mut() {
            let ended = if stopping {
                let _ = child.kill();
                let _ = child.wait();
                Some(Exit::Killed)
            } else {
                match child.try_wait() {
                    Ok(Some(status)) => Some(Exit::Status(status)),
                    Ok(None) => None,
                    Err(_) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        Some(Exit::Killed)
                    }
                }
            };
            if let Some(exit) = ended {
                state.exit = Some(exit);
                // The liveness pipe's write end closes with the helper gone.
                state.child = None;
                break;
            }
        } else if state.cause.is_some() || state.finished {
            break;
        }
        let wait = deadline
            .saturating_duration_since(Instant::now())
            .clamp(Duration::from_millis(1), TICK);
        state = shared
            .changed
            .wait_timeout(state, wait)
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .0;
    }
    drop(state);
    shared.changed.notify_all();
    drop(slot);
}

impl PreviewRunner for WorkerRunner {
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        let media_type = context.media_type().unwrap_or_default().to_owned();
        let slot = self.take_slot();
        let shared = Arc::new(Shared::default());
        let supervisor = {
            let shared = shared.clone();
            let deadline = self.deadline;
            std::thread::Builder::new()
                .name("preview-supervisor".into())
                .spawn(move || supervise(shared, slot, deadline))
                .map_err(|_| OperatorError::Failed(STOPPED.into()))?
        };
        let outcome = self.drive(context, &shared, &media_type);
        shared.finish();
        drop(supervisor);
        outcome
    }
}

impl WorkerRunner {
    /// Copy, launch, and pass the output on; then decide the run's outcome.
    fn drive(
        &self,
        context: &mut AssetContext<'_>,
        shared: &Shared,
        media_type: &str,
    ) -> Result<(), OperatorError> {
        self.copies.fetch_add(1, Ordering::SeqCst);
        let file = match copy(context, shared)? {
            Some(file) => file,
            None => return verdict(shared, media_type, None),
        };
        let Some(mut stdout) = self.launch(file, media_type, shared)? else {
            return verdict(shared, media_type, None);
        };
        let refused = drain(context, shared, &mut stdout);
        shared.wait_for_exit();
        verdict(shared, media_type, refused)
    }

    /// Start the helper over `file` and hand it to the supervisor. `None`
    /// when the run stopped first.
    fn launch(
        &self,
        file: File,
        media_type: &str,
        shared: &Shared,
    ) -> Result<Option<ChildStdout>, OperatorError> {
        let stopped = |_| OperatorError::Failed(STOPPED.into());
        let (read, write) = liveness_pipe().map_err(stopped)?;
        let mut state = shared.state();
        if state.cause.is_some() {
            return Ok(None);
        }
        let mut command = Command::new(&self.command.program);
        command
            .args(&self.command.args)
            .arg(media_type)
            .env_clear()
            .stdin(Stdio::from(file))
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let raw_read = read.as_raw_fd();
        // SAFETY: only async-signal-safe calls run between fork and exec.
        unsafe {
            command.pre_exec(move || {
                if raw_read == LIVENESS {
                    let flags = libc::fcntl(LIVENESS, libc::F_GETFD);
                    libc::fcntl(LIVENESS, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
                } else if libc::dup2(raw_read, LIVENESS) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().map_err(stopped)?;
        drop(command);
        drop(read);
        let stdout = child.stdout.take().expect("stdout is piped");
        state.child = Some((child, write));
        drop(state);
        shared.changed.notify_all();
        Ok(Some(stdout))
    }
}

/// Copy the candidate to an anonymous file, rewound. `None` when the run
/// stopped during the copy: the deadline passed or the host cancelled.
fn copy(context: &mut AssetContext<'_>, shared: &Shared) -> Result<Option<File>, OperatorError> {
    let stopped = |_| OperatorError::Failed(STOPPED.into());
    let mut file = anonymous_file().map_err(stopped)?;
    let mut chunk = vec![0; CHUNK];
    let mut offset = 0;
    loop {
        if shared.cause().is_some() {
            return Ok(None);
        }
        if context.is_cancelled() {
            shared.stop(Cause::Cancelled);
            return Ok(None);
        }
        // A read or write that fails after the run stopped keeps the stop's
        // reason, such as the deadline that passed while it was blocked.
        let read = match context.read_candidate(offset, &mut chunk) {
            Ok(read) => read,
            Err(_) if shared.cause().is_some() => return Ok(None),
            Err(_) => return Err(OperatorError::Failed(UNREADABLE.into())),
        };
        if read == 0 {
            break;
        }
        match file.write_all(&chunk[..read]) {
            Ok(()) => {}
            Err(_) if shared.cause().is_some() => return Ok(None),
            Err(error) => return Err(stopped(error)),
        }
        offset += read as u64;
    }
    // A read that returned after the deadline must not launch a helper.
    if shared.cause().is_some() {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(0)).map_err(stopped)?;
    Ok(Some(file))
}

/// A new file in the temporary folder that no name refers to.
fn anonymous_file() -> io::Result<File> {
    let folder = std::env::temp_dir();
    loop {
        let path = folder.join(format!(".sidereal-preview-{}", uuid::Uuid::new_v4()));
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => {
                std::fs::remove_file(&path)?;
                return Ok(file);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}

/// A pipe whose two ends close on exec.
fn liveness_pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0; 2];
    #[cfg(target_os = "linux")]
    // SAFETY: `fds` has room for both descriptors.
    let made = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) };
    #[cfg(not(target_os = "linux"))]
    // SAFETY: as above; each end is then marked close-on-exec.
    let made = unsafe {
        let made = libc::pipe(fds.as_mut_ptr());
        if made == 0 {
            libc::fcntl(fds[0], libc::F_SETFD, libc::FD_CLOEXEC);
            libc::fcntl(fds[1], libc::F_SETFD, libc::FD_CLOEXEC);
        }
        made
    };
    if made != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: both descriptors are new and owned here.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

/// Pass the helper's output on through the Asset context until it ends or
/// the run stops. A write the context refused stops the run; its error is
/// the result.
fn drain(
    context: &mut AssetContext<'_>,
    shared: &Shared,
    stdout: &mut ChildStdout,
) -> Option<ContextError> {
    let mut chunk = vec![0; CHUNK];
    loop {
        if shared.cause().is_some() {
            return None;
        }
        if context.is_cancelled() {
            shared.stop(Cause::Cancelled);
            return None;
        }
        let mut ready = libc::pollfd {
            fd: stdout.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one valid `pollfd`.
        let polled = unsafe { libc::poll(&mut ready, 1, TICK.as_millis() as i32) };
        if polled == 0
            || (polled < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted)
        {
            continue;
        }
        let read = match stdout.read(&mut chunk) {
            Ok(0) | Err(_) => return None,
            Ok(read) => read,
        };
        if let Err(error) = context.write_preview(&chunk[..read]) {
            shared.stop(Cause::Output);
            return Some(error);
        }
    }
}

/// The run's outcome: the first cause that stopped it, or else how the
/// helper ended.
fn verdict(
    shared: &Shared,
    media_type: &str,
    refused: Option<ContextError>,
) -> Result<(), OperatorError> {
    let state = shared.state();
    match state.cause {
        Some(Cause::Deadline) => Err(OperatorError::Failed(TIMED_OUT.into())),
        Some(Cause::Cancelled) => Err(OperatorError::Failed(CANCELLED.into())),
        Some(Cause::Output) => Err(refused.map_or(OperatorError::Output, OperatorError::from)),
        None => match state.exit {
            Some(Exit::Status(status)) if status.success() => Ok(()),
            Some(Exit::Status(status)) => Err(OperatorError::Failed(
                status
                    .code()
                    .and_then(JobFailure::from_status)
                    .map_or_else(|| STOPPED.to_owned(), |f| f.reason(media_type)),
            )),
            _ => Err(OperatorError::Failed(STOPPED.into())),
        },
    }
}
