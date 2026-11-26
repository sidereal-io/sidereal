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
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
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
