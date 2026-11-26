//! The preview helper process, `sidereal-preview-worker` (ADR-020). The
//! server's [`crate::preview::WorkerRunner`] starts one per preview, with
//! the candidate's copy as standard input and the media type as the last
//! argument, and reads the JPEG from standard output.
//!
//! All work that depends on the candidate's bytes runs on one decoder thread
//! with a 2 MiB stack. Before it reads any input, that thread measures the
//! stack the operating system gave it, maps the rest of the budget once, and
//! from then on takes every allocation from that mapping. There is no
//! fallback to the system allocator: a refused allocation exits the process
//! at once with [`JobFailure::Budget`]'s status.
//!
//! A watcher thread reads the liveness pipe on file descriptor 3. Its write
//! end belongs to the server, so when the server goes away the watcher sees
//! the end of the pipe and stops the helper.
use linked_list_allocator::Heap;
use sidereal_pack_image::preview::JobFailure;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::{Cell, UnsafeCell},
    fs::File,
    io::Write,
    os::fd::FromRawFd,
    process::ExitCode,
    ptr::NonNull,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

/// The memory one preview decode may use: 512 MiB.
pub const BUDGET: usize = 512 * 1024 * 1024;

/// The stack the decoder thread asks for.
const STACK: usize = 2 * 1024 * 1024;

/// The largest stack the helper accepts from the operating system.
const MAX_STACK: usize = 4 * 1024 * 1024;

/// The file descriptor of the liveness pipe's read end.
const LIVENESS: i32 = 3;

/// The fixed-capacity heap of the decoder thread.
struct Arena {
    lock: AtomicBool,
    heap: UnsafeCell<Heap>,
    start: AtomicUsize,
    end: AtomicUsize,
}

// SAFETY: `heap` is only touched while `lock` is held.
unsafe impl Sync for Arena {}

static ARENA: Arena = Arena {
    lock: AtomicBool::new(false),
    heap: UnsafeCell::new(Heap::empty()),
    start: AtomicUsize::new(0),
    end: AtomicUsize::new(0),
};

impl Arena {
    fn with<T>(&self, f: impl FnOnce(&mut Heap) -> T) -> T {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
        // SAFETY: the lock is held, so no other thread uses the heap.
        let out = f(unsafe { &mut *self.heap.get() });
        self.lock.store(false, Ordering::Release);
        out
    }

    fn holds(&self, pointer: *mut u8) -> bool {
        let address = pointer as usize;
        address >= self.start.load(Ordering::Acquire) && address < self.end.load(Ordering::Acquire)
    }
}

thread_local! {
    /// Set on the decoder thread once its heap is mapped.
    static DECODING: Cell<bool> = const { Cell::new(false) };
}

fn decoding() -> bool {
    DECODING.try_with(Cell::get).unwrap_or(false)
}

/// Exit with the budget status, allocating nothing.
fn refuse() -> ! {
    // SAFETY: `_exit` ends the process without running any more Rust code.
    unsafe { libc::_exit(JobFailure::Budget.status().into()) }
}

/// The helper's global allocator: the decoder thread's allocations come
/// from its fixed heap, every other thread's from the system allocator. A
/// block is freed to whichever of the two holds it.
pub struct BoundedAllocator;

// SAFETY: every block comes from the heap or the system allocator and goes
// back to the one that holds it.
unsafe impl GlobalAlloc for BoundedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if !decoding() {
            return unsafe { System.alloc(layout) };
        }
        match ARENA.with(|heap| heap.allocate_first_fit(layout)) {
            Ok(pointer) => pointer.as_ptr(),
            Err(()) => refuse(),
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if !decoding() {
            return unsafe { System.alloc_zeroed(layout) };
        }
        let pointer = unsafe { self.alloc(layout) };
        // SAFETY: `alloc` gave `layout.size()` writable bytes.
        unsafe { pointer.write_bytes(0, layout.size()) };
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if ARENA.holds(pointer) {
            // SAFETY: the heap gave this block with this layout.
            ARENA.with(|heap| unsafe { heap.deallocate(NonNull::new_unchecked(pointer), layout) });
        } else {
            unsafe { System.dealloc(pointer, layout) }
        }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if !decoding() && !ARENA.holds(pointer) {
            return unsafe { System.realloc(pointer, layout, size) };
        }
        // The new block is taken while the old one is still held, so growth
        // needs room for both, as it would in place of the copy.
        // SAFETY: the caller's layout and size are valid for `realloc`.
        let new_layout = unsafe { Layout::from_size_align_unchecked(size, layout.align()) };
        let moved = unsafe { self.alloc(new_layout) };
        unsafe {
            std::ptr::copy_nonoverlapping(pointer, moved, layout.size().min(size));
            self.dealloc(pointer, layout);
        }
        moved
    }
}

/// What one helper run does, from its arguments.
struct Job {
    budget: usize,
    check: Option<String>,
    media_type: String,
}

/// `[--budget BYTES] [--self-check NAME] MEDIA-TYPE`. A smaller budget and
/// the self-checks are for the helper's own tests.
fn parse(mut args: impl Iterator<Item = String>) -> Option<Job> {
    let mut job = Job {
        budget: BUDGET,
        check: None,
        media_type: String::new(),
    };
    loop {
        match args.next()?.as_str() {
            "--budget" => job.budget = args.next()?.parse().ok().filter(|&b| b <= BUDGET)?,
            "--self-check" => job.check = Some(args.next()?),
            media_type => {
                job.media_type = media_type.to_owned();
                return args.next().is_none().then_some(job);
            }
        }
    }
}

/// The size of the calling thread's stack, as the operating system reports
/// it.
#[cfg(target_os = "linux")]
fn stack_size() -> Option<usize> {
    // SAFETY: `attr` is initialized by `pthread_getattr_np` before use and
    // destroyed after.
    unsafe {
        let mut attr = std::mem::MaybeUninit::<libc::pthread_attr_t>::uninit();
        if libc::pthread_getattr_np(libc::pthread_self(), attr.as_mut_ptr()) != 0 {
            return None;
        }
        let mut address = std::ptr::null_mut();
        let mut size = 0;
        let found = libc::pthread_attr_getstack(attr.as_ptr(), &mut address, &mut size);
        libc::pthread_attr_destroy(attr.as_mut_ptr());
        (found == 0).then_some(size)
    }
}

#[cfg(target_os = "macos")]
fn stack_size() -> Option<usize> {
    // SAFETY: it reads the calling thread's own attributes.
    Some(unsafe { libc::pthread_get_stacksize_np(libc::pthread_self()) })
}

fn page_size() -> usize {
    // SAFETY: `sysconf` has no preconditions.
    usize::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).unwrap_or(4096)
}

/// The decoder's heap: the budget less the stack, in whole pages.
struct Mapped {
    stack: usize,
    heap: usize,
    page: usize,
}

/// Measure the stack, then map and install the decoder's heap.
fn map_heap(budget: usize) -> Option<Mapped> {
    let stack = stack_size().filter(|&s| s > 0 && s <= MAX_STACK)?;
    let page = page_size();
    let heap = budget.checked_sub(stack)? / page * page;
    if heap == 0 {
        return None;
    }
    // SAFETY: an anonymous private mapping with no address hint.
    let mapping = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            heap,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANON,
            -1,
            0,
        )
    };
    if mapping == libc::MAP_FAILED {
        return None;
    }
    let start = mapping as usize;
    // SAFETY: the mapping is `heap` writable bytes that nothing else uses.
    ARENA.with(|arena| unsafe { arena.init(mapping.cast(), heap) });
    ARENA.start.store(start, Ordering::Release);
    ARENA.end.store(start + heap, Ordering::Release);
    Some(Mapped { stack, heap, page })
}

/// The helper's standard output, written without a buffer of its own.
fn stdout() -> std::mem::ManuallyDrop<File> {
    // SAFETY: descriptor 1 stays open for the life of the process.
    std::mem::ManuallyDrop::new(unsafe { File::from_raw_fd(1) })
}

/// On the decoder thread: map the heap, then run the job. The exit status.
fn decode(job: Job) -> u8 {
    let Some(mapped) = map_heap(job.budget) else {
        return JobFailure::Startup.status();
    };
    DECODING.with(|d| d.set(true));
    let heap = mapped.heap;
    let outcome = match job.check.as_deref() {
        None => {
            // SAFETY: descriptor 0 is the candidate's copy, open for the
            // life of the process.
            let input = std::mem::ManuallyDrop::new(unsafe { File::from_raw_fd(0) });
            sidereal_pack_image::preview::make_preview(
                &input,
                &job.media_type,
                heap,
                &mut *stdout(),
            )
        }
        Some("describe") => {
            let text = format!(
                "stack={} heap={} page={} budget={}\n",
                mapped.stack, mapped.heap, mapped.page, job.budget
            );
            stdout()
                .write_all(text.as_bytes())
                .map_err(|_| JobFailure::Startup)
        }
        // A zeroed allocation one byte larger than the heap.
        Some("zeroed") => {
            std::hint::black_box(vec![0u8; heap + 1]);
            Ok(())
        }
        // Growth from 6/10 to 7/10 of the heap needs both blocks at once.
        Some("grow") => {
            let mut grown: Vec<u8> = Vec::with_capacity(heap / 10 * 6);
            grown.push(1);
            grown.reserve_exact(heap / 10 * 7);
            std::hint::black_box(grown);
            Ok(())
        }
        // Growth from 4/10 to 5/10 of the heap fits both blocks.
        Some("fits") => {
            let mut grown: Vec<u8> = Vec::with_capacity(heap / 10 * 4);
            grown.push(1);
            grown.reserve_exact(heap / 10 * 5);
            std::hint::black_box(grown);
            Ok(())
        }
        Some("stall") => loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        },
        Some(_) => Err(JobFailure::Startup),
    };
    match outcome {
        Ok(()) => 0,
        Err(failure) => failure.status(),
    }
}

/// Stop the helper when the liveness pipe ends: the server has gone away.
fn watch_liveness() {
    // SAFETY: `fcntl` only asks whether the descriptor is open.
    if unsafe { libc::fcntl(LIVENESS, libc::F_GETFD) } == -1 {
        return;
    }
    let _ = std::thread::Builder::new().spawn(|| {
        let mut byte = 0u8;
        loop {
            // SAFETY: a one-byte read into a local.
            let read = unsafe { libc::read(LIVENESS, (&mut byte as *mut u8).cast(), 1) };
            let interrupted = read < 0
                && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted;
            if read <= 0 && !interrupted {
                // SAFETY: `_exit` ends the process at once.
                unsafe { libc::_exit(JobFailure::Startup.status().into()) }
            }
        }
    });
}

/// The helper's entry point.
pub fn main() -> ExitCode {
    let Some(job) = parse(std::env::args().skip(1)) else {
        return ExitCode::from(JobFailure::Startup.status());
    };
    watch_liveness();
    let decoder = std::thread::Builder::new()
        .name("decoder".into())
        .stack_size(STACK)
        .spawn(move || decode(job));
    match decoder.map(|thread| thread.join()) {
        Ok(Ok(status)) => ExitCode::from(status),
        _ => ExitCode::from(JobFailure::Startup.status()),
    }
}
