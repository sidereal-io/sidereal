//! The preview helper and its runner, with no database. The real helper is
//! the `sidereal-preview-worker` binary built with the server; small shell
//! scripts stand in for a helper that stalls, closes its output or fails.
#[path = "../../packs/image/tests/common/tiff.rs"]
mod tiff;

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
    // Both ends close on exec, so no helper another test starts meanwhile
    // keeps the write end open.
    let mut fds = [0; 2];
    #[cfg(target_os = "linux")]
    assert_eq!(unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) }, 0);
    #[cfg(not(target_os = "linux"))]
    unsafe {
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        libc::fcntl(fds[0], libc::F_SETFD, libc::FD_CLOEXEC);
        libc::fcntl(fds[1], libc::F_SETFD, libc::FD_CLOEXEC);
    }
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
            // `dup2` clears close-on-exec on descriptor 3, but does nothing
            // when the read end already is 3.
            if raw_read == 3 {
                let flags = libc::fcntl(3, libc::F_GETFD);
                libc::fcntl(3, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
            } else if libc::dup2(raw_read, 3) < 0 {
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

/// A candidate whose first read blocks until `release` gets a message, then
/// returns its bytes, or fails when `fails` is set.
struct Stalls {
    release: Mutex<mpsc::Receiver<()>>,
    started: mpsc::Sender<()>,
    fails: bool,
}

impl Candidate for Stalls {
    fn size(&self) -> u64 {
        5
    }
    fn read_at(&mut self, _: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        let _ = self.started.send(());
        let _ = self.release.lock().unwrap().recv();
        if self.fails {
            return Err(std::io::Error::other("the disk went away"));
        }
        buf[..5].copy_from_slice(b"bytes");
        Ok(5)
    }
}

/// Upload A's run frees the slot, upload B takes it, and then A's admission
/// is dropped. B must keep its slot until its own run takes it.
#[tokio::test(flavor = "multi_thread")]
async fn an_earlier_uploads_admission_leaves_a_later_uploads_slot_alone() {
    let runner = script_runner("exec cat");
    let first = runner.admit().await;
    let ran = {
        let runner = runner.clone();
        tokio::task::spawn_blocking(move || run(&runner, b"first"))
            .await
            .unwrap()
    };
    assert_eq!(ran, (Ok(()), b"first".to_vec()));
    let second = runner.admit().await;
    drop(first);
    let third = tokio::time::timeout(Duration::from_millis(200), runner.admit()).await;
    assert!(
        third.is_err(),
        "a third upload took the second upload's slot"
    );
    drop(second);
}

#[test]
fn a_stalled_input_read_that_fails_after_the_deadline_keeps_the_timeout_reason() {
    let runner = script_runner("exec cat");
    let (release, wait) = mpsc::channel();
    let (started, has_started) = mpsc::channel();
    let first = {
        let runner = runner.clone();
        std::thread::spawn(move || {
            let mut candidate = Stalls {
                release: Mutex::new(wait),
                started,
                fails: true,
            };
            run_with(&runner, &mut candidate, &mut Kept::default(), &|| false)
        })
    };
    has_started.recv().unwrap();
    std::thread::sleep(DEADLINE + Duration::from_millis(500));
    release.send(()).unwrap();
    assert_eq!(first.join().unwrap(), failed(TIMED_OUT));
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
                fails: false,
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

// ---- the preview job, through the image pack's Operator ----

/// The image pack, whose previews run in the real helper.
fn image_registry() -> Registry {
    let runner = Arc::new(WorkerRunner::new(WorkerCommand {
        program: HELPER.into(),
        args: Vec::new(),
    }));
    let mut registry = Registry::new();
    registry
        .register_pack(&sidereal_pack_image::ImagePack::new(runner))
        .unwrap();
    registry
}

/// The preview of `bytes`, a version of `media_type`, or why it failed.
fn preview(bytes: &[u8], media_type: &str) -> Result<Vec<u8>, ExecutionError> {
    let mut output = Kept::default();
    let run = image_registry().execute_operator(
        sidereal_pack_image::MAKE_PREVIEW,
        &mut Bytes(bytes.to_vec()),
        media_type,
        &mut output,
        &mut |_| {},
        &|| false,
    )?;
    assert_eq!(run.preview, Some(AssetProposal::new("image", "image/jpeg")));
    assert_eq!(run.preview_bytes, output.0.len() as u64);
    Ok(output.0)
}

/// A decoded JPEG: its size, its number of components, and its pixels as
/// RGB.
struct Decoded {
    width: usize,
    height: usize,
    components: u8,
    rgb: Vec<u8>,
}

impl Decoded {
    fn pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let at = (y * self.width + x) * 3;
        [self.rgb[at], self.rgb[at + 1], self.rgb[at + 2]]
    }
}

fn decode(jpeg: &[u8]) -> Decoded {
    use zune_jpeg::zune_core::{
        bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions,
    };
    let options = DecoderOptions::default()
        .set_max_width(65_535)
        .set_max_height(65_535)
        .jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(jpeg), options);
    decoder.decode_headers().unwrap();
    let components = decoder.info().unwrap().components;
    let (width, height) = decoder.dimensions().unwrap();
    let rgb = decoder.decode().unwrap();
    Decoded {
        width,
        height,
        components,
        rgb,
    }
}

/// The marker of the first frame header in `jpeg`.
fn frame_marker(jpeg: &[u8]) -> u8 {
    let mut at = 2;
    loop {
        let marker = jpeg[at + 1];
        if (0xc0..=0xcf).contains(&marker) && ![0xc4, 0xc8, 0xcc].contains(&marker) {
            return marker;
        }
        at += 2 + usize::from(u16::from_be_bytes([jpeg[at + 2], jpeg[at + 3]]));
    }
}

fn near(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(&x, y)| x.abs_diff(y) <= 8)
}

/// A PNG of `width` by `height` in `color` at `depth`, whose samples are
/// `data`.
fn png(
    width: u32,
    height: u32,
    color: png::ColorType,
    depth: png::BitDepth,
    data: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(color);
    encoder.set_depth(depth);
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(data).unwrap();
    writer.finish().unwrap();
    out
}

/// An 8-bit RGB image whose pixel at x, y is `pixel(x, y)`.
fn rgb_samples(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
    let mut data = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            data.extend_from_slice(&pixel(x, y));
        }
    }
    data
}

fn gradient(x: u32, y: u32) -> [u8; 3] {
    [(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8]
}

/// A JPEG of an 8-bit RGB image, at `sampling`, progressive or baseline,
/// with `app1` as an EXIF segment when given.
fn jpeg(
    width: u16,
    height: u16,
    sampling: jpeg_encoder::SamplingFactor,
    progressive: bool,
    app1: Option<&[u8]>,
) -> Vec<u8> {
    let data = rgb_samples(width.into(), height.into(), gradient);
    let mut out = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut out, 90);
    encoder.set_sampling_factor(sampling);
    encoder.set_progressive(progressive);
    if let Some(app1) = app1 {
        encoder.add_app_segment(1, app1.to_vec()).unwrap();
    }
    encoder
        .encode(&data, width, height, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    out
}

fn baseline(width: u16, height: u16) -> Vec<u8> {
    jpeg(
        width,
        height,
        jpeg_encoder::SamplingFactor::R_4_2_0,
        false,
        None,
    )
}

fn undecodable(format: &str) -> Result<Vec<u8>, ExecutionError> {
    Err(ExecutionError::Failed(format!(
        "the {format} image could not be decoded"
    )))
}

const BUDGET: &str = "the image needs more than 512 MiB to decode";

#[test]
fn png_and_jpeg_an_rgb_png_gets_a_baseline_jpeg_of_the_same_size() {
    let bytes = png(
        4096,
        2160,
        png::ColorType::Rgb,
        png::BitDepth::Eight,
        &rgb_samples(4096, 2160, gradient),
    );
    let jpeg = preview(&bytes, "image/png").unwrap();
    assert_eq!(frame_marker(&jpeg), 0xc0, "not a baseline JPEG");
    let decoded = decode(&jpeg);
    assert_eq!(
        (decoded.width, decoded.height, decoded.components),
        (4096, 2160, 3)
    );
    for (x, y) in [(10, 10), (300, 1000), (4000, 2100)] {
        assert!(
            near(decoded.pixel(x, y), gradient(x as u32, y as u32)),
            "{x}, {y}"
        );
    }
}

#[test]
fn png_and_jpeg_a_gray_png_gets_a_gray_jpeg() {
    let data: Vec<u8> = (0..64 * 64).map(|n| (n % 64 * 4) as u8).collect();
    let jpeg = preview(
        &png(
            64,
            64,
            png::ColorType::Grayscale,
            png::BitDepth::Eight,
            &data,
        ),
        "image/png",
    )
    .unwrap();
    let decoded = decode(&jpeg);
    assert_eq!(
        (decoded.width, decoded.height, decoded.components),
        (64, 64, 1)
    );
}

#[test]
fn png_and_jpeg_a_16_bit_png_scales_to_8_bits_with_no_brightening() {
    let mut data = Vec::new();
    for y in 0..48u32 {
        for _ in 0..48u32 {
            let value: u16 = [0, 32_768, 1_000][(y / 16) as usize];
            data.extend_from_slice(&value.to_be_bytes());
        }
    }
    let jpeg = preview(
        &png(
            48,
            48,
            png::ColorType::Grayscale,
            png::BitDepth::Sixteen,
            &data,
        ),
        "image/png",
    )
    .unwrap();
    let decoded = decode(&jpeg);
    for (y, shown) in [(8, 0u8), (24, 128), (40, 4)] {
        assert!(
            near(decoded.pixel(24, y), [shown; 3]),
            "{:?}",
            decoded.pixel(24, y)
        );
    }
}

#[test]
fn png_and_jpeg_a_transparent_png_pixel_shows_over_black() {
    let mut data = Vec::new();
    for _ in 0..64u32 {
        for x in 0..64u32 {
            data.extend_from_slice(if x < 32 {
                &[255, 255, 255, 0]
            } else {
                &[255, 128, 0, 255]
            });
        }
    }
    let jpeg = preview(
        &png(64, 64, png::ColorType::Rgba, png::BitDepth::Eight, &data),
        "image/png",
    )
    .unwrap();
    let decoded = decode(&jpeg);
    assert!(
        near(decoded.pixel(8, 32), [0, 0, 0]),
        "{:?}",
        decoded.pixel(8, 32)
    );
    assert!(
        near(decoded.pixel(56, 32), [255, 128, 0]),
        "{:?}",
        decoded.pixel(56, 32)
    );
}

/// An EXIF segment whose orientation says to rotate the image 90 degrees.
fn exif_rotated() -> Vec<u8> {
    let mut exif =
        b"Exif\0\0MM\0*\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01\0\x06\0\0\0\0\0\0".to_vec();
    exif.resize(64, 0);
    exif
}

#[test]
fn png_and_jpeg_a_jpeg_with_an_exif_rotation_keeps_its_stored_width_and_height() {
    let bytes = jpeg(
        600,
        400,
        jpeg_encoder::SamplingFactor::R_4_2_0,
        false,
        Some(&exif_rotated()),
    );
    let decoded = decode(&preview(&bytes, "image/jpeg").unwrap());
    assert_eq!((decoded.width, decoded.height), (600, 400));
}

#[test]
fn png_and_jpeg_a_progressive_jpeg_of_several_scans_gets_a_preview() {
    let bytes = jpeg(
        1920,
        1080,
        jpeg_encoder::SamplingFactor::R_4_2_0,
        true,
        None,
    );
    assert_eq!(frame_marker(&bytes), 0xc2);
    let decoded = decode(&preview(&bytes, "image/jpeg").unwrap());
    assert_eq!((decoded.width, decoded.height), (1920, 1080));
    assert!(near(decoded.pixel(500, 500), gradient(500, 500)));
}

/// A PNG's signature and its 13-byte header chunk only.
fn png_header(width: u32, height: u32, color: u8) -> Vec<u8> {
    let whole = png(1, 1, png::ColorType::Rgb, png::BitDepth::Eight, &[0, 0, 0]);
    let mut bytes = whole[..33].to_vec();
    bytes[16..20].copy_from_slice(&width.to_be_bytes());
    bytes[20..24].copy_from_slice(&height.to_be_bytes());
    bytes[25] = color;
    let crc = crc32(&bytes[12..29]);
    bytes[29..33].copy_from_slice(&crc.to_be_bytes());
    bytes
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[test]
fn png_and_jpeg_a_png_over_the_budget_is_refused_before_decoding() {
    let mut bytes = png_header(20_000, 10_000, 6);
    bytes
        .extend_from_slice(&png(1, 1, png::ColorType::Rgb, png::BitDepth::Eight, &[0, 0, 0])[33..]);
    assert_eq!(
        preview(&bytes, "image/png"),
        Err(ExecutionError::Failed(BUDGET.into()))
    );
}

/// A JPEG's SOI, a baseline three-component frame header of `width` by
/// `height`, and EOI.
fn jpeg_frame_only(width: u16, height: u16) -> Vec<u8> {
    let mut bytes = vec![0xff, 0xd8, 0xff, 0xc0, 0, 17, 8];
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&[3, 1, 0x11, 0, 2, 0x11, 0, 3, 0x11, 0]);
    bytes.extend_from_slice(&[0xff, 0xda, 0, 12, 3, 1, 0, 2, 0x11, 3, 0x11, 0, 63, 0]);
    bytes.extend_from_slice(&[0xff, 0xd9]);
    bytes
}

#[test]
fn png_and_jpeg_a_jpeg_over_the_budget_is_refused_before_decoding() {
    assert_eq!(
        preview(&jpeg_frame_only(60_000, 60_000), "image/jpeg"),
        Err(ExecutionError::Failed(BUDGET.into()))
    );
}

#[test]
fn png_and_jpeg_a_png_cut_off_after_its_header_fails() {
    assert_eq!(
        preview(&png_header(64, 64, 2), "image/png"),
        undecodable("PNG")
    );
}

#[test]
fn png_and_jpeg_a_png_with_complete_pixels_but_no_iend_fails() {
    let whole = png(
        64,
        64,
        png::ColorType::Rgb,
        png::BitDepth::Eight,
        &rgb_samples(64, 64, gradient),
    );
    let mut bytes = whole[..whole.len() - 12].to_vec();
    assert_eq!(&whole[whole.len() - 8..whole.len() - 4], b"IEND");
    let text = b"tEXtkey\0value";
    bytes.extend_from_slice(&((text.len() - 4) as u32).to_be_bytes());
    bytes.extend_from_slice(text);
    bytes.extend_from_slice(&crc32(text).to_be_bytes());
    assert_eq!(preview(&bytes, "image/png"), undecodable("PNG"));
}

#[test]
fn png_and_jpeg_a_jpeg_with_no_readable_frame_header_fails() {
    let mut bytes = vec![0xff, 0xd8, 0xff, 0xfe, 0x03, 0xe6];
    bytes.resize(1000, b'x');
    assert_eq!(preview(&bytes, "image/jpeg"), undecodable("JPEG"));
}

#[test]
fn png_and_jpeg_a_jpeg_with_no_eoi_fails() {
    let whole = baseline(800, 600);
    assert_eq!(
        preview(&whole[..whole.len() - 2], "image/jpeg"),
        undecodable("JPEG")
    );
    let progressive = jpeg(640, 480, jpeg_encoder::SamplingFactor::R_4_2_0, true, None);
    let cut = &progressive[..progressive.len() * 2 / 3];
    assert_eq!(preview(cut, "image/jpeg"), undecodable("JPEG"));
}

#[test]
fn png_and_jpeg_an_exif_thumbnails_eoi_inside_a_cut_off_file_fails() {
    let thumbnail = baseline(16, 16);
    let mut exif = b"Exif\0\0".to_vec();
    exif.extend_from_slice(&thumbnail);
    let whole = jpeg(
        800,
        600,
        jpeg_encoder::SamplingFactor::R_4_2_0,
        false,
        Some(&exif),
    );
    assert!(whole.windows(2).filter(|w| w == &[0xff, 0xd9]).count() >= 2);
    let cut = &whole[..whole.len() - 100];
    assert_eq!(preview(cut, "image/jpeg"), undecodable("JPEG"));
}

#[test]
fn png_and_jpeg_a_jpeg_whose_last_scan_loses_its_final_8_bytes_gets_a_full_preview() {
    let whole = baseline(800, 600);
    let mut bytes = whole[..whole.len() - 10].to_vec();
    bytes.extend_from_slice(&[0xff, 0xd9]);
    let decoded = decode(&preview(&bytes, "image/jpeg").unwrap());
    assert_eq!((decoded.width, decoded.height), (800, 600));
}

#[test]
fn png_and_jpeg_a_jpeg_that_names_an_undefined_huffman_table_fails() {
    let mut bytes = baseline(64, 64);
    let sos = bytes.windows(2).position(|w| w == [0xff, 0xda]).unwrap();
    // The first component's table selectors: DC and AC table 3, which no
    // DHT segment defines.
    bytes[sos + 6] = 0x33;
    assert_eq!(preview(&bytes, "image/jpeg"), undecodable("JPEG"));
}

const SEPARATE_444: &[u8] =
    include_bytes!("../../packs/image/tests/fixtures/jpeg-444-separate-scans.jpg");
const SEPARATE_420: &[u8] =
    include_bytes!("../../packs/image/tests/fixtures/jpeg-420-separate-scans.jpg");
const SEPARATE_444_MEANS: &[u8] =
    include_bytes!("../../packs/image/tests/fixtures/jpeg-444-separate-scans.means");

#[test]
fn png_and_jpeg_a_subsampled_jpeg_with_separate_component_scans_is_refused() {
    assert_eq!(
        preview(SEPARATE_420, "image/jpeg"),
        Err(ExecutionError::Failed(
            "the JPEG image type is not supported".into()
        ))
    );
}

/// Damage decides the reason before the layout does.
#[test]
fn png_and_jpeg_a_cut_off_subsampled_jpeg_with_separate_scans_could_not_be_decoded() {
    assert_eq!(&SEPARATE_420[SEPARATE_420.len() - 2..], &[0xff, 0xd9]);
    assert_eq!(
        preview(&SEPARATE_420[..SEPARATE_420.len() - 2], "image/jpeg"),
        Err(ExecutionError::Failed(
            "the JPEG image could not be decoded".into()
        ))
    );
}

#[test]
fn png_and_jpeg_a_444_jpeg_with_separate_component_scans_matches_the_reference_decode() {
    let decoded = decode(&preview(SEPARATE_444, "image/jpeg").unwrap());
    assert_eq!((decoded.width, decoded.height), (640, 480));
    let mut at = 0;
    for by in 0..30 {
        for bx in 0..40 {
            for channel in 0..3 {
                let mut sum = 0u32;
                for y in by * 16..by * 16 + 16 {
                    for x in bx * 16..bx * 16 + 16 {
                        sum += u32::from(decoded.pixel(x, y)[channel]);
                    }
                }
                let mean = (sum as f64 / 256.0).round() as u8;
                let reference = SEPARATE_444_MEANS[at];
                assert!(
                    mean.abs_diff(reference) <= 8,
                    "block {bx}, {by}, channel {channel}: {mean} against {reference}"
                );
                at += 1;
            }
        }
    }
}

/// `jpeg` with comment segments after its SOI, so that its first SOS
/// header ends at byte `end`.
fn first_scan_header_ending_at(jpeg: &[u8], end: usize) -> Vec<u8> {
    let sos = jpeg.windows(2).position(|w| w == [0xff, 0xda]).unwrap();
    let header_end = sos + 2 + usize::from(u16::from_be_bytes([jpeg[sos + 2], jpeg[sos + 3]]));
    let mut padding = end - header_end;
    let mut bytes = jpeg[..2].to_vec();
    while padding > 0 {
        let take = match padding {
            n if n > 65_537 + 4 => 65_537,
            n if n > 65_537 => n - 4,
            n => n,
        };
        bytes.extend_from_slice(&[0xff, 0xfe]);
        bytes.extend_from_slice(&((take - 2) as u16).to_be_bytes());
        bytes.resize(bytes.len() + take - 4, b'x');
        padding -= take;
    }
    bytes.extend_from_slice(&jpeg[2..]);
    bytes
}

const LIMIT: usize = 8 * 1024 * 1024;

#[test]
fn png_and_jpeg_a_jpeg_whose_eoi_lies_after_8_mib_gets_a_preview() {
    let bytes = first_scan_header_ending_at(&baseline(640, 480), LIMIT);
    let sos = bytes.windows(2).rposition(|w| w == [0xff, 0xda]).unwrap();
    assert_eq!(
        sos + 2 + usize::from(u16::from_be_bytes([bytes[sos + 2], bytes[sos + 3]])),
        LIMIT
    );
    assert!(bytes.len() > LIMIT + 1000);
    let decoded = decode(&preview(&bytes, "image/jpeg").unwrap());
    assert_eq!((decoded.width, decoded.height), (640, 480));
}

#[test]
fn png_and_jpeg_a_jpeg_whose_first_scan_header_ends_one_byte_past_8_mib_fails() {
    let bytes = first_scan_header_ending_at(&baseline(640, 480), LIMIT + 1);
    assert_eq!(preview(&bytes, "image/jpeg"), undecodable("JPEG"));
}

// ---- TIFF previews ----

use tiff::{
    compress, floating_point_row, horizontal, tiles_of, Image, Tag, Tiff, BITS_PER_SAMPLE,
    COMPRESSION, DEFLATE_CODE, EXTRA_SAMPLES, LZW_CODE, PACKBITS_CODE, PHOTOMETRIC, ROWS_PER_STRIP,
    SAMPLES_PER_PIXEL, SAMPLE_FORMAT, SHORT, TILE_LENGTH, TILE_WIDTH, UNCOMPRESSED,
};

const TIFF_UNSUPPORTED: &str = "the TIFF image type is not supported";
const TOO_WIDE: &str = "the image is wider or taller than a browser-ready JPEG can hold";

fn tiff_failed(reason: &str) -> Result<Vec<u8>, ExecutionError> {
    Err(ExecutionError::Failed(reason.into()))
}

/// Samples of `width` by `height` pixels, `samples` each, from
/// `value(x, y, sample)`, as `size`-byte numbers in the file's byte order.
fn samples_of(
    width: usize,
    height: usize,
    samples: usize,
    size: usize,
    big_endian: bool,
    value: impl Fn(usize, usize, usize) -> u64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(width * height * samples * size);
    for y in 0..height {
        for x in 0..width {
            for sample in 0..samples {
                let bytes = value(x, y, sample).to_be_bytes();
                let field = &bytes[8 - size..];
                if big_endian {
                    out.extend_from_slice(field);
                } else {
                    out.extend(field.iter().rev());
                }
            }
        }
    }
    out
}

/// An image of chunky samples in one strip, compressed with `compression`.
fn strip_image(
    width: u64,
    height: u64,
    bits: u64,
    samples: u64,
    photometric: u64,
    compression: u64,
    data: Vec<u8>,
) -> Image {
    let mut image = Image::chunky(
        width,
        height,
        bits,
        samples,
        photometric,
        compress(&data, compression),
    );
    image = image.with(COMPRESSION, SHORT, &[compression]);
    image
}

fn rgb8_pixels(width: usize, height: usize) -> Vec<u8> {
    samples_of(width, height, 3, 1, false, |x, y, s| {
        u64::from(gradient(x as u32, y as u32)[s])
    })
}

#[test]
fn tiff_an_8_bit_rgb_tiff_gets_a_color_preview_of_its_size() {
    let bytes = Tiff::classic(vec![strip_image(
        800,
        600,
        8,
        3,
        2,
        UNCOMPRESSED,
        rgb8_pixels(800, 600),
    )])
    .bytes();
    let decoded = decode(&preview(&bytes, "image/tiff").unwrap());
    assert_eq!(
        (decoded.width, decoded.height, decoded.components),
        (800, 600, 3)
    );
    assert!(near(decoded.pixel(400, 300), gradient(400, 300)));
}

#[test]
fn tiff_the_first_of_several_images_gives_the_preview() {
    let bytes = Tiff::classic(vec![
        strip_image(800, 600, 8, 3, 2, PACKBITS_CODE, rgb8_pixels(800, 600)),
        strip_image(200, 150, 8, 3, 2, UNCOMPRESSED, rgb8_pixels(200, 150)),
    ])
    .bytes();
    let decoded = decode(&preview(&bytes, "image/tiff").unwrap());
    assert_eq!((decoded.width, decoded.height), (800, 600));
}

#[test]
fn tiff_a_big_endian_big_tiff_of_16_bit_rgb_with_deflate_gets_a_preview() {
    let data = samples_of(256, 128, 3, 2, true, |x, y, s| {
        [x * 256, y * 512, 40_000][s] as u64
    });
    let bytes = Tiff::classic(vec![strip_image(256, 128, 16, 3, 2, DEFLATE_CODE, data)])
        .big_endian()
        .big_tiff()
        .bytes();
    assert_eq!(&bytes[..4], b"MM\0+");
    let decoded = decode(&preview(&bytes, "image/tiff").unwrap());
    assert_eq!((decoded.width, decoded.height), (256, 128));
    let expected = |x: usize, y: usize| {
        [x * 256, y * 512, 40_000].map(|v| ((v as f64) * 255.0 / 65_535.0).round() as u8)
    };
    assert!(
        near(decoded.pixel(100, 50), expected(100, 50)),
        "{:?}",
        decoded.pixel(100, 50)
    );
}

#[test]
fn tiff_an_lzw_tiff_in_tiles_with_a_horizontal_predictor_gets_a_preview() {
    let (width, height, tile) = (80usize, 48usize, 32usize);
    let pixels = rgb8_pixels(width, height);
    let tiles: Vec<Vec<u8>> = tiles_of(&pixels, width, height, 3, tile, tile)
        .into_iter()
        .map(|mut data| {
            horizontal(&mut data, tile * 3, 3, 1, false);
            compress(&data, LZW_CODE)
        })
        .collect();
    let image = Image {
        tags: Image::chunky(width as u64, height as u64, 8, 3, 2, Vec::new())
            .without(ROWS_PER_STRIP)
            .coded(LZW_CODE, Some(2))
            .with(TILE_WIDTH, SHORT, &[tile as u64])
            .with(TILE_LENGTH, SHORT, &[tile as u64])
            .tags,
        strips: None,
        tiles: Some(tiles),
    };
    let decoded = decode(&preview(&Tiff::classic(vec![image]).bytes(), "image/tiff").unwrap());
    assert_eq!((decoded.width, decoded.height), (80, 48));
    for (x, y) in [(5, 5), (70, 40), (40, 20)] {
        assert!(
            near(decoded.pixel(x, y), gradient(x as u32, y as u32)),
            "{x}, {y}"
        );
    }
}

#[test]
fn tiff_more_than_64_tiles_get_a_preview() {
    let (width, height, tile) = (160usize, 160usize, 16usize);
    let pixels = samples_of(width, height, 1, 1, false, |x, _, _| (x % 256) as u64);
    let tiles = tiles_of(&pixels, width, height, 1, tile, tile);
    assert_eq!(tiles.len(), 100);
    let image = Image {
        tags: Image::chunky(width as u64, height as u64, 8, 1, 1, Vec::new())
            .without(ROWS_PER_STRIP)
            .with(TILE_WIDTH, SHORT, &[tile as u64])
            .with(TILE_LENGTH, SHORT, &[tile as u64])
            .tags,
        strips: None,
        tiles: Some(tiles),
    };
    let decoded = decode(&preview(&Tiff::classic(vec![image]).bytes(), "image/tiff").unwrap());
    assert_eq!(
        (decoded.width, decoded.height, decoded.components),
        (160, 160, 1)
    );
}

fn gray_regions(photometric: u64, bits: u64, size: usize, values: &[u64]) -> Vec<u8> {
    let (width, height) = (16 * values.len(), 16);
    let data = samples_of(width, height, 1, size, false, |x, _, _| values[x / 16]);
    let mut image = strip_image(
        width as u64,
        height as u64,
        bits,
        1,
        photometric,
        UNCOMPRESSED,
        data,
    );
    if size == 4 {
        image = image.with(SAMPLE_FORMAT, SHORT, &[3]);
    }
    preview(&Tiff::classic(vec![image]).bytes(), "image/tiff").unwrap()
}

/// The gray value at the middle of each 16-pixel region.
fn region_values(jpeg: &[u8], regions: usize) -> Vec<u8> {
    let decoded = decode(jpeg);
    assert_eq!(decoded.components, 1);
    (0..regions)
        .map(|r| decoded.pixel(r * 16 + 8, 8)[0])
        .collect()
}

fn all_near(found: &[u8], expected: &[u8]) {
    assert_eq!(found.len(), expected.len());
    for (f, e) in found.iter().zip(expected) {
        assert!(f.abs_diff(*e) <= 8, "{found:?} against {expected:?}");
    }
}

#[test]
fn tiff_an_8_bit_gray_tiff_gets_a_gray_preview() {
    all_near(
        &region_values(&gray_regions(1, 8, 1, &[0, 128, 255]), 3),
        &[0, 128, 255],
    );
}

#[test]
fn tiff_a_white_is_zero_tiff_shows_0_as_white_and_255_as_black() {
    all_near(
        &region_values(&gray_regions(0, 8, 1, &[0, 255]), 2),
        &[255, 0],
    );
}

#[test]
fn tiff_a_16_bit_tiff_scales_to_8_bits() {
    all_near(
        &region_values(&gray_regions(1, 16, 2, &[0, 32_768, 65_535]), 3),
        &[0, 128, 255],
    );
}

#[test]
fn tiff_a_32_bit_decimal_tiff_clips_to_0_and_1() {
    let values = [-0.5f32, 0.5, 2.0, f32::NAN].map(|v| u64::from(v.to_bits()));
    all_near(
        &region_values(&gray_regions(1, 32, 4, &values), 4),
        &[0, 128, 255, 0],
    );
}

#[test]
fn tiff_unstretched_16_bit_data_stays_dark() {
    let data = samples_of(64, 64, 3, 2, false, |x, y, s| {
        ((x * 7 + y * 3 + s * 100) % 1001) as u64
    });
    let jpeg = preview(
        &Tiff::classic(vec![strip_image(64, 64, 16, 3, 2, UNCOMPRESSED, data)]).bytes(),
        "image/tiff",
    )
    .unwrap();
    let decoded = decode(&jpeg);
    assert!(
        decoded.rgb.iter().all(|&v| v <= 4 + 8),
        "a sample is brighter than 12"
    );
}

/// A Deflate-compressed float gray image of `width` by `height` in tiles of
/// `tile` by `tile`, with the floating-point predictor.
fn float_tiles(
    width: usize,
    height: usize,
    tile: usize,
    value: impl Fn(usize, usize) -> f32,
) -> Vec<u8> {
    let mut values = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            values.push(value(x, y));
        }
    }
    let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_ne_bytes()).collect();
    let tiles: Vec<Vec<u8>> = tiles_of(&bytes, width, height, 4, tile, tile)
        .into_iter()
        .map(|data| {
            let mut predicted = Vec::new();
            for row in data.chunks(tile * 4) {
                let row: Vec<f32> = row
                    .chunks(4)
                    .map(|b| f32::from_ne_bytes(b.try_into().unwrap()))
                    .collect();
                predicted.extend(floating_point_row(&row));
            }
            compress(&predicted, DEFLATE_CODE)
        })
        .collect();
    let image = Image {
        tags: Image::chunky(width as u64, height as u64, 32, 1, 1, Vec::new())
            .without(ROWS_PER_STRIP)
            .with(SAMPLE_FORMAT, SHORT, &[3])
            .coded(DEFLATE_CODE, Some(3))
            .with(TILE_WIDTH, SHORT, &[tile as u64])
            .with(TILE_LENGTH, SHORT, &[tile as u64])
            .tags,
        strips: None,
        tiles: Some(tiles),
    };
    Tiff::classic(vec![image]).bytes()
}

#[test]
fn tiff_a_float_tiff_with_a_padded_edge_tile_keeps_its_size() {
    let bytes = float_tiles(40, 24, 32, |x, _| x as f32 / 40.0);
    let decoded = decode(&preview(&bytes, "image/tiff").unwrap());
    assert_eq!((decoded.width, decoded.height), (40, 24));
    assert!(
        decoded.pixel(20, 10)[0].abs_diff(128) <= 8,
        "{:?}",
        decoded.pixel(20, 10)
    );
}

/// An 800 by 600 8-bit image with one strip of its samples, with `change`
/// applied to its tags.
fn eight_bit(samples: u64, photometric: u64, change: impl FnOnce(Image) -> Image) -> Vec<u8> {
    let data = vec![100u8; (800 * 600 * samples) as usize];
    Tiff::classic(vec![change(Image::chunky(
        800,
        600,
        8,
        samples,
        photometric,
        data,
    ))])
    .bytes()
}

#[test]
fn tiff_of_another_type_fails_as_not_supported() {
    for (what, bytes) in [
        ("CMYK", eight_bit(4, 5, |i| i)),
        ("JPEG-compressed", eight_bit(3, 2, |i| i.coded(7, None))),
        (
            "RGB with alpha",
            eight_bit(4, 2, |i| i.with(EXTRA_SAMPLES, SHORT, &[2])),
        ),
        ("signed 16-bit", {
            let data = vec![0u8; 800 * 600 * 2];
            Tiff::classic(vec![Image::chunky(800, 600, 16, 1, 1, data).with(
                SAMPLE_FORMAT,
                SHORT,
                &[2],
            )])
            .bytes()
        }),
        (
            "a float predictor on integers",
            eight_bit(1, 1, |i| i.coded(LZW_CODE, Some(3))),
        ),
        ("a horizontal predictor on floats", {
            let data = vec![0u8; 800 * 600 * 4];
            Tiff::classic(vec![Image::chunky(800, 600, 32, 1, 1, data)
                .with(SAMPLE_FORMAT, SHORT, &[3])
                .coded(LZW_CODE, Some(2))])
            .bytes()
        }),
        (
            "a predictor with no compression",
            eight_bit(1, 1, |i| i.coded(UNCOMPRESSED, Some(2))),
        ),
        (
            "planar samples",
            eight_bit(3, 2, |i| i.with(284, SHORT, &[2])),
        ),
        (
            "12-bit samples",
            eight_bit(1, 1, |i| i.with(BITS_PER_SAMPLE, SHORT, &[12])),
        ),
        (
            "mixed sample sizes",
            eight_bit(3, 2, |i| i.with(BITS_PER_SAMPLE, SHORT, &[8, 8, 16])),
        ),
        (
            "more than 64 samples",
            eight_bit(1, 1, |i| i.with(BITS_PER_SAMPLE, SHORT, &[8; 65])),
        ),
    ] {
        assert_eq!(
            preview(&bytes, "image/tiff"),
            tiff_failed(TIFF_UNSUPPORTED),
            "{what}"
        );
    }
}

/// A PackBits-compressed image of `width` by `height`, every sample the
/// same, in strips of 16 rows: real pixel data in a small file.
fn uniform(width: u64, height: u64, bits: u64, samples: u64, photometric: u64) -> Image {
    let row = vec![7u8; (width * samples * bits / 8) as usize];
    let strip: Vec<u8> = row.repeat(16);
    let packed = compress(&strip, PACKBITS_CODE);
    let strips = vec![packed; height.div_ceil(16) as usize];
    let mut image = Image::chunky(width, height, bits, samples, photometric, Vec::new())
        .with(ROWS_PER_STRIP, SHORT, &[16])
        .coded(PACKBITS_CODE, None);
    image.strips = Some(strips);
    image
}

#[test]
fn tiff_a_cmyk_tiff_over_the_budget_fails_as_not_supported() {
    let bytes = Tiff::classic(vec![uniform(16_000, 12_000, 8, 4, 5)]).bytes();
    assert_eq!(preview(&bytes, "image/tiff"), tiff_failed(TIFF_UNSUPPORTED));
}

#[test]
fn tiff_a_side_over_65500_fails_and_one_of_65500_gets_a_preview() {
    let wide = Tiff::classic(vec![uniform(65_501, 100, 8, 1, 1)]).bytes();
    assert_eq!(preview(&wide, "image/tiff"), tiff_failed(TOO_WIDE));
    let widest = Tiff::classic(vec![uniform(65_500, 100, 8, 1, 1)]).bytes();
    let decoded = decode(&preview(&widest, "image/tiff").unwrap());
    assert_eq!((decoded.width, decoded.height), (65_500, 100));
}

#[test]
fn tiff_pixels_over_the_budget_fail_before_decoding() {
    for (what, image) in [
        (
            "16-bit RGB of 540,000,000 bytes",
            uniform(15_000, 6_000, 16, 3, 2),
        ),
        ("8-bit gray of 512 MiB", uniform(32_768, 16_384, 8, 1, 1)),
    ] {
        let bytes = Tiff::classic(vec![image]).bytes();
        assert_eq!(preview(&bytes, "image/tiff"), tiff_failed(BUDGET), "{what}");
    }
}

#[test]
fn tiff_tile_scratch_over_the_budget_fails() {
    // One tile row is wider than the whole budget.
    let image = Image {
        tags: Image::chunky(16, 16, 32, 1, 1, Vec::new())
            .without(ROWS_PER_STRIP)
            .with(SAMPLE_FORMAT, SHORT, &[3])
            .coded(DEFLATE_CODE, Some(3))
            .with(TILE_WIDTH, tiff::LONG, &[1 << 27])
            .with(TILE_LENGTH, SHORT, &[16])
            .tags,
        strips: None,
        tiles: Some(vec![compress(&[0u8; 64], DEFLATE_CODE)]),
    };
    assert_eq!(
        preview(&Tiff::classic(vec![image]).bytes(), "image/tiff"),
        tiff_failed(BUDGET)
    );
}

#[test]
fn tiff_beyond_the_codecs_default_256_mib_output_gets_a_preview() {
    let bytes = Tiff::classic(vec![uniform(16_384, 6_400, 8, 3, 2)]).bytes();
    let jpeg = preview(&bytes, "image/tiff").unwrap();
    let decoded = decode(&jpeg);
    assert_eq!((decoded.width, decoded.height), (16_384, 6_400));
}

#[test]
fn tiff_a_tiff_cut_off_inside_its_last_strip_fails() {
    let mut image = Image::chunky(64, 64, 8, 3, 2, Vec::new()).with(ROWS_PER_STRIP, SHORT, &[16]);
    image.strips = Some(
        rgb8_pixels(64, 64)
            .chunks(64 * 16 * 3)
            .map(<[u8]>::to_vec)
            .collect(),
    );
    let bytes = Tiff::classic(vec![image]).bytes();
    // The writer puts the strips before the directory. Keep the directory,
    // but cut the last strip short and move the directory up to follow it.
    let directory = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let mut cut = bytes[..directory - 1000].to_vec();
    let moved = cut.len() as u32;
    cut.extend_from_slice(&bytes[directory..]);
    cut[4..8].copy_from_slice(&moved.to_le_bytes());
    // The values outside the directory moved by the same distance.
    let shift = (directory - moved as usize) as u32;
    let mut fixed = cut.clone();
    let entries = u16::from_le_bytes([fixed[moved as usize], fixed[moved as usize + 1]]) as usize;
    for entry in 0..entries {
        let at = moved as usize + 2 + entry * 12;
        let kind = u16::from_le_bytes([fixed[at + 2], fixed[at + 3]]);
        let count = u32::from_le_bytes(fixed[at + 4..at + 8].try_into().unwrap());
        let size = match kind {
            3 => 2,
            4 => 4,
            _ => 1,
        };
        if count * size > 4 {
            let offset = u32::from_le_bytes(fixed[at + 8..at + 12].try_into().unwrap());
            fixed[at + 8..at + 12].copy_from_slice(&(offset - shift).to_le_bytes());
        }
    }
    assert_eq!(
        preview(&fixed, "image/tiff"),
        tiff_failed("the TIFF image could not be decoded")
    );
}

#[test]
fn tiff_bytes_that_are_a_png_fail_as_a_tiff() {
    let bytes = png(
        16,
        16,
        png::ColorType::Rgb,
        png::BitDepth::Eight,
        &rgb_samples(16, 16, gradient),
    );
    assert_eq!(
        preview(&bytes, "image/tiff"),
        tiff_failed("the TIFF image could not be decoded")
    );
}

/// The preview of `bytes` in a helper whose budget is `budget` bytes.
fn preview_within(bytes: &[u8], budget: usize) -> Result<Vec<u8>, ExecutionError> {
    let runner = Arc::new(WorkerRunner::new(WorkerCommand {
        program: HELPER.into(),
        args: vec!["--budget".into(), budget.to_string().into()],
    }));
    let mut registry = Registry::new();
    registry
        .register_pack(&sidereal_pack_image::ImagePack::new(runner))
        .unwrap();
    let mut output = Kept::default();
    registry
        .execute_operator(
            sidereal_pack_image::MAKE_PREVIEW,
            &mut Bytes(bytes.to_vec()),
            "image/tiff",
            &mut output,
            &mut |_| {},
            &|| false,
        )
        .map(|_| output.0)
}

/// The stack the helper charges, read from the helper itself.
fn stack() -> usize {
    let (_, out) = helper(&["--self-check", "describe", "image/tiff"]);
    out.split_whitespace()
        .find_map(|part| part.strip_prefix("stack="))
        .unwrap()
        .parse()
        .unwrap()
}

const KIB: usize = 1024;

/// The smallest heap, in whole 4 KiB pages, at which the helper makes a
/// preview of `bytes`; every smaller one is refused with the budget reason.
fn smallest_heap(bytes: &[u8]) -> usize {
    let stack = stack();
    let (mut refused, mut made) = (0usize, 64 * 1024 * KIB);
    assert!(
        preview_within(bytes, stack + made).is_ok(),
        "no preview even with 64 MiB"
    );
    while made - refused > 4 * KIB {
        let middle = (refused + made) / 2 / (4 * KIB) * (4 * KIB);
        match preview_within(bytes, stack + middle) {
            Ok(_) => made = middle,
            Err(error) => {
                assert_eq!(Err::<Vec<u8>, _>(error), tiff_failed(BUDGET));
                refused = middle;
            }
        }
    }
    made
}

/// What the preflight check counts for an image: its pixels and one row of
/// a strip or tile. A smallest heap above this means that, just below it,
/// the check passed and the heap refused an allocation during the decode.
fn counted(pixels: usize, row: usize) -> usize {
    pixels + row
}

#[test]
fn tiff_constructor_allocations_count_against_a_reduced_budget() {
    // The same 16 by 4,096 pixels in one strip, and in 4,096 one-row
    // strips, whose offsets and byte counts the decoder keeps as it starts.
    let image = |rows: u64| {
        let mut image =
            Image::chunky(16, 4_096, 8, 1, 1, Vec::new()).with(ROWS_PER_STRIP, SHORT, &[rows]);
        image.strips = Some(
            vec![9u8; 16 * 4_096]
                .chunks(16 * rows as usize)
                .map(<[u8]>::to_vec)
                .collect(),
        );
        Tiff::classic(vec![image]).bytes()
    };
    let one = smallest_heap(&image(4_096));
    let many = smallest_heap(&image(1));
    assert!(
        many >= one + 64 * KIB,
        "one strip {one}, 4,096 strips {many}"
    );
    assert!(many > counted(16 * 4_096, 16));
}

#[test]
fn tiff_float_strip_decoding_counts_against_a_reduced_budget() {
    let values: Vec<u8> = (0..256 * 64)
        .flat_map(|n| (n as f32 / 16_384.0).to_le_bytes())
        .collect();
    let image =
        strip_image(256, 64, 32, 1, 1, DEFLATE_CODE, values).with(SAMPLE_FORMAT, SHORT, &[3]);
    let heap = smallest_heap(&Tiff::classic(vec![image]).bytes());
    assert!(heap > counted(256 * 64 * 4, 256 * 4), "{heap}");
}

#[test]
fn tiff_padded_tile_decoding_counts_against_a_reduced_budget() {
    // 40 by 40 pixels in one 256 by 256 tile: the decoder reads padded rows.
    let heap = smallest_heap(&float_tiles(40, 40, 256, |x, y| ((x + y) % 2) as f32));
    assert!(heap > counted(40 * 40 * 4, 256 * 4), "{heap}");
}

#[allow(dead_code)]
fn unused(_: Tag, _: u16, _: u16) {
    let _ = (PHOTOMETRIC, SAMPLES_PER_PIXEL, COMPRESSION);
}
