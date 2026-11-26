//! The preview helper and its runner, with no database. The real helper is
//! the `sidereal-preview-worker` binary built with the server; small shell
//! scripts stand in for a helper that stalls, closes its output or fails.
use sidereal_pack_image::preview::PreviewRunner;
use sidereal_plugin_abi::{
    AssetContext, AssetProposal, Candidate, ExecutionError, Operator, OperatorError, Output,
    Registry,
};
use sidereal_server::preview::{WorkerCommand, WorkerRunner};
use std::{
    ffi::OsString,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

const HELPER: &str = env!("CARGO_BIN_EXE_sidereal-preview-worker");
const MIB: u64 = 1024 * 1024;
const BUDGET_STATUS: i32 = 70;
const TIMED_OUT: &str = "the preview decoder timed out";
const STOPPED: &str = "the preview decoder stopped";

/// The deadline the tests give a run.
const DEADLINE: Duration = Duration::from_secs(2);

// ---- the helper on its own ----

/// Run the real helper with `args` and empty input; its exit status and
/// what it wrote.
fn helper(args: &[&str]) -> (Option<i32>, String) {
    let output = Command::new(HELPER)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

#[test]
fn the_helper_charges_its_whole_stack_and_maps_the_rest_of_512_mib() {
    let (status, out) = helper(&["--self-check", "describe", "image/png"]);
    assert_eq!(status, Some(0), "{out}");
    let field = |name: &str| -> u64 {
        out.split_whitespace()
            .find_map(|part| part.strip_prefix(&format!("{name}=")))
            .unwrap_or_else(|| panic!("no {name} in {out:?}"))
            .parse()
            .unwrap()
    };
    let (stack, heap, page, budget) = (
        field("stack"),
        field("heap"),
        field("page"),
        field("budget"),
    );
    assert_eq!(budget, 512 * MIB);
    assert!((2 * MIB..=4 * MIB).contains(&stack), "stack {stack}");
    assert_eq!(heap % page, 0);
    assert!(stack + heap <= budget);
    assert!(budget - stack - heap < page, "{out}");
}

#[test]
fn a_zeroed_allocation_over_a_reduced_budget_exits_with_the_budget_status() {
    let (status, _) = helper(&[
        "--budget",
        "16777216",
        "--self-check",
        "zeroed",
        "image/png",
    ]);
    assert_eq!(status, Some(BUDGET_STATUS));
}

#[test]
fn growth_that_must_hold_the_old_and_new_buffer_exits_with_the_budget_status() {
    let (status, _) = helper(&["--budget", "16777216", "--self-check", "grow", "image/png"]);
    assert_eq!(status, Some(BUDGET_STATUS));
    let (status, _) = helper(&["--budget", "16777216", "--self-check", "fits", "image/png"]);
    assert_eq!(status, Some(0));
}

/// A helper whose liveness pipe's write end the test holds, as a server
/// would. Dropping it is what the helper sees when its parent exits.
#[test]
fn the_helper_exits_when_its_parent_goes_away() {
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let (read, write) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
    let raw_read = read.as_raw_fd();
    let raw_write = write.as_raw_fd();
    let mut command = Command::new(HELPER);
    command
        .args(["--self-check", "stall", "image/png"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    unsafe {
        command.pre_exec(move || {
            libc::close(raw_write);
            if libc::dup2(raw_read, 3) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().unwrap();
    drop(read);
    std::thread::sleep(Duration::from_millis(300));
    assert!(child.try_wait().unwrap().is_none(), "it stopped too soon");
    drop(write);
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "it outlived its parent"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

// ---- the runner ----

/// An Operator that proposes a JPEG preview and lets the runner make it.
struct Previews(Arc<WorkerRunner>);

impl Operator for Previews {
    fn id(&self) -> &str {
        "test.preview"
    }
    fn version(&self) -> &str {
        "1"
    }
    fn writes(&self) -> &[&str] {
        &[]
    }
    fn accepts(&self) -> &[&str] {
        &["image/png"]
    }
    fn makes_previews(&self) -> bool {
        true
    }
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        context.propose_preview(AssetProposal::new("image", "image/jpeg"))?;
        self.0.run(context)
    }
}

fn registry(runner: &Arc<WorkerRunner>) -> Registry {
    let mut registry = Registry::new();
    registry.register_operator(Box::new(Previews(runner.clone())));
    registry
}

struct Bytes(Vec<u8>);

impl Candidate for Bytes {
    fn size(&self) -> u64 {
        self.0.len() as u64
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        let start = (offset as usize).min(self.0.len());
        let n = buf.len().min(self.0.len() - start);
        buf[..n].copy_from_slice(&self.0[start..start + n]);
        Ok(n)
    }
}

/// The host's destination: it keeps every byte.
#[derive(Default)]
struct Kept(Vec<u8>);

impl Output for Kept {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.0.extend_from_slice(bytes);
        Ok(())
    }
}

/// Run the preview over `candidate` with `runner`.
fn run_with(
    runner: &Arc<WorkerRunner>,
    candidate: &mut dyn Candidate,
    output: &mut dyn Output,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), ExecutionError> {
    registry(runner)
        .execute_operator(
            "test.preview",
            candidate,
            "image/png",
            output,
            &mut |_| {},
            cancelled,
        )
        .map(drop)
}

fn run(runner: &Arc<WorkerRunner>, bytes: &[u8]) -> (Result<(), ExecutionError>, Vec<u8>) {
    let mut output = Kept::default();
    let outcome = run_with(runner, &mut Bytes(bytes.to_vec()), &mut output, &|| false);
    (outcome, output.0)
}

fn failed(reason: &str) -> Result<(), ExecutionError> {
    Err(ExecutionError::Failed(reason.into()))
}

/// A folder for one test's files, removed afterwards.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("preview-worker-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn pid_file(&self) -> PathBuf {
        self.0.join("pid")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A runner whose helper is `/bin/sh -c script`, with the media type as its
/// first argument, and the tests' deadline.
fn script_runner(script: &str) -> Arc<WorkerRunner> {
    Arc::new(
        WorkerRunner::new(WorkerCommand {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), script.into(), "worker".into()],
        })
        .with_deadline(DEADLINE),
    )
}

/// A script that records its process id in `scratch`, then runs `rest`.
fn recording(scratch: &Scratch, rest: &str) -> String {
    format!("echo $$ > '{}'; {rest}", scratch.pid_file().display())
}

/// The process id the script recorded, once it has.
fn recorded_pid(scratch: &Scratch) -> i32 {
    let start = Instant::now();
    loop {
        if let Ok(text) = std::fs::read_to_string(scratch.pid_file()) {
            if let Ok(pid) = text.trim().parse() {
                return pid;
            }
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "the helper never started"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Whether process `pid` still exists. A reaped process does not.
fn exists(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[test]
fn a_helper_that_copies_its_input_returns_every_byte() {
    let runner = script_runner("exec cat");
    let input: Vec<u8> = (0..300_000u32).map(|n| n as u8).collect();
    let (outcome, output) = run(&runner, &input);
    assert_eq!(outcome, Ok(()));
    assert_eq!(output, input);
}

#[test]
fn the_helper_gets_the_media_type_and_no_environment() {
    let runner = script_runner("printf '%s|%s' \"$1\" \"${HOME:-none}\"");
    let (outcome, output) = run(&runner, b"bytes");
    assert_eq!(outcome, Ok(()));
    assert_eq!(output, b"image/png|none");
}

#[test]
fn an_allocation_refusal_stays_in_the_helper() {
    let runner = Arc::new(
        WorkerRunner::new(WorkerCommand {
            program: HELPER.into(),
            args: ["--budget", "16777216", "--self-check", "zeroed"]
                .map(OsString::from)
                .to_vec(),
        })
        .with_deadline(DEADLINE),
    );
    let (outcome, output) = run(&runner, b"png bytes");
    assert_eq!(
        outcome,
        failed("the image needs more than 512 MiB to decode")
    );
    assert!(output.is_empty());
    // This process is unharmed, and the next run takes the slot.
    let (outcome, _) = run(&script_runner("exec cat"), b"next");
    assert_eq!(outcome, Ok(()));
}

#[test]
fn a_helper_that_fails_after_returning_part_of_its_output_fails_the_run() {
    let (outcome, output) = run(&script_runner("printf 'part of a jpeg'; exit 1"), b"bytes");
    assert_eq!(outcome, failed(STOPPED));
    assert_eq!(output, b"part of a jpeg");
    let (outcome, _) = run(&script_runner("printf 'part'; exit 73"), b"bytes");
    assert_eq!(outcome, failed("the PNG image could not be decoded"));
    let (outcome, _) = run(&script_runner("kill -9 $$"), b"bytes");
    assert_eq!(outcome, failed(STOPPED));
}

#[test]
fn a_cancelled_run_stops_and_reaps_its_helper() {
    let scratch = Scratch::new();
    let runner = script_runner(&recording(&scratch, "exec sleep 30"));
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let worker = {
        let runner = runner.clone();
        std::thread::spawn(move || {
            run_with(
                &runner,
                &mut Bytes(b"bytes".to_vec()),
                &mut Kept::default(),
                &|| flag.load(Ordering::SeqCst),
            )
        })
    };
    let pid = recorded_pid(&scratch);
    let start = Instant::now();
    cancel.store(true, Ordering::SeqCst);
    assert_eq!(worker.join().unwrap(), Err(ExecutionError::Cancelled));
    assert!(start.elapsed() < DEADLINE, "it waited for the deadline");
    assert!(!exists(pid), "the helper was not reaped");
}

#[test]
fn a_helper_silent_until_its_deadline_times_out_and_is_reaped() {
    let scratch = Scratch::new();
    let runner = script_runner(&recording(&scratch, "exec sleep 30"));
    let start = Instant::now();
    let (outcome, _) = run(&runner, b"bytes");
    assert_eq!(outcome, failed(TIMED_OUT));
    assert!(start.elapsed() >= DEADLINE);
    assert!(start.elapsed() < DEADLINE + Duration::from_secs(3));
    assert!(!exists(recorded_pid(&scratch)));
}

#[test]
fn a_helper_that_closes_its_output_but_never_exits_times_out() {
    let scratch = Scratch::new();
    let runner = script_runner(&recording(
        &scratch,
        "printf 'some'; exec >&-; exec sleep 30",
    ));
    let (outcome, output) = run(&runner, b"bytes");
    assert_eq!(outcome, failed(TIMED_OUT));
    assert_eq!(output, b"some");
    assert!(!exists(recorded_pid(&scratch)));
}

/// A candidate whose first read blocks until `release` gets a message.
struct Stalls {
    release: Mutex<mpsc::Receiver<()>>,
    started: mpsc::Sender<()>,
}

impl Candidate for Stalls {
    fn size(&self) -> u64 {
        5
    }
    fn read_at(&mut self, _: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        let _ = self.started.send(());
        let _ = self.release.lock().unwrap().recv();
        buf[..5].copy_from_slice(b"bytes");
        Ok(5)
    }
}

#[test]
fn a_stalled_input_copy_times_out_and_frees_the_slot_for_the_next_preview() {
    let scratch = Scratch::new();
    let launches = scratch.0.join("launches");
    let runner = script_runner(&format!(
        "echo launched >> '{}'; exec cat",
        launches.display()
    ));
    let (release, wait) = mpsc::channel();
    let (started, has_started) = mpsc::channel();
    let first = {
        let runner = runner.clone();
        std::thread::spawn(move || {
            let mut candidate = Stalls {
                release: Mutex::new(wait),
                started,
            };
            let start = Instant::now();
            let outcome = run_with(&runner, &mut candidate, &mut Kept::default(), &|| false);
            (outcome, start.elapsed())
        })
    };
    has_started.recv().unwrap();
    std::thread::sleep(DEADLINE + Duration::from_millis(500));
    // The slot is free: a second preview runs to the end while the first
    // copy is still blocked, and other work of the host goes on.
    let (outcome, output) = run(&runner, b"second");
    assert_eq!((outcome, output), (Ok(()), b"second".to_vec()));
    let download = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(tokio::fs::read(HELPER))
        .unwrap();
    assert!(!download.is_empty());
    release.send(()).unwrap();
    let (outcome, elapsed) = first.join().unwrap();
    assert_eq!(outcome, failed(TIMED_OUT));
    assert!(elapsed >= DEADLINE);
    // Only the second preview launched a helper.
    assert_eq!(std::fs::read_to_string(&launches).unwrap(), "launched\n");
}

/// A destination whose first write blocks until `release` says how it ends.
struct Blocks {
    release: Mutex<mpsc::Receiver<bool>>,
    entered: mpsc::Sender<()>,
}

impl Output for Blocks {
    fn write(&mut self, _: &[u8]) -> std::io::Result<()> {
        let _ = self.entered.send(());
        match self.release.lock().unwrap().recv() {
            Ok(true) => Ok(()),
            _ => Err(std::io::Error::other("the disk is full")),
        }
    }
}

/// Run a preview whose helper writes, then stalls, into a destination that
/// blocks. At the deadline, check the helper is reaped and a second preview
/// can run before the write returns; then end the write with `succeeds`.
fn blocked_write_at_the_deadline(succeeds: bool) -> Result<(), ExecutionError> {
    let scratch = Scratch::new();
    let runner = script_runner(&recording(&scratch, "printf 'jpeg'; exec sleep 30"));
    let (release, wait) = mpsc::channel();
    let (entered, has_entered) = mpsc::channel();
    let first = {
        let runner = runner.clone();
        std::thread::spawn(move || {
            let mut output = Blocks {
                release: Mutex::new(wait),
                entered,
            };
            run_with(&runner, &mut Bytes(b"bytes".to_vec()), &mut output, &|| {
                false
            })
        })
    };
    has_entered.recv().unwrap();
    let pid = recorded_pid(&scratch);
    std::thread::sleep(DEADLINE + Duration::from_millis(500));
    assert!(!exists(pid), "the helper outlived its deadline");
    // The slot is free before the write returns: a second preview takes it
    // and runs its own helper to its own deadline.
    let (outcome, _) = run(&runner, b"second");
    assert_eq!(
        outcome,
        failed(TIMED_OUT),
        "the second preview ran its own helper"
    );
    release.send(succeeds).unwrap();
    first.join().unwrap()
}

#[test]
fn a_write_blocked_past_the_deadline_records_the_timeout_when_it_returns() {
    assert_eq!(blocked_write_at_the_deadline(true), failed(TIMED_OUT));
}

#[test]
fn a_write_blocked_past_the_deadline_that_then_fails_records_the_timeout() {
    assert_eq!(blocked_write_at_the_deadline(false), failed(TIMED_OUT));
}

/// A destination that refuses every write.
struct Refuses;

impl Output for Refuses {
    fn write(&mut self, _: &[u8]) -> std::io::Result<()> {
        Err(std::io::Error::other("the disk is full"))
    }
}

#[test]
fn a_write_failure_before_the_deadline_stops_the_helper_as_an_output_failure() {
    let scratch = Scratch::new();
    let runner = script_runner(&recording(&scratch, "printf 'jpeg'; exec sleep 30"));
    let start = Instant::now();
    let outcome = run_with(
        &runner,
        &mut Bytes(b"bytes".to_vec()),
        &mut Refuses,
        &|| false,
    );
    assert_eq!(outcome, Err(ExecutionError::Output));
    assert!(start.elapsed() < DEADLINE);
    assert!(!exists(recorded_pid(&scratch)));
}

#[test]
fn a_candidate_read_failure_records_that_the_stored_file_could_not_be_read() {
    struct Unreadable;
    impl Candidate for Unreadable {
        fn size(&self) -> u64 {
            10
        }
        fn read_at(&mut self, _: u64, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("the disk could not be read"))
        }
    }
    let outcome = run_with(
        &script_runner("exec cat"),
        &mut Unreadable,
        &mut Kept::default(),
        &|| false,
    );
    assert_eq!(outcome, failed("the stored file could not be read"));
}

/// The helper resolves beside the running executable.
#[test]
fn the_production_command_names_the_helper_beside_the_server() {
    let command = WorkerCommand::beside(Path::new("/opt/sidereal/bin/sidereal-server"));
    assert_eq!(
        command.program,
        PathBuf::from("/opt/sidereal/bin/sidereal-preview-worker")
    );
    assert!(command.args.is_empty());
}
