//! Test-only: the counting allocator of the pack's unit-test build, an
//! in-memory candidate, and small builders for crafted files.
use crate::{preview::PreviewRunner, ImagePack, READ_HEADER};
use sidereal_plugin_abi::{Candidate, ExecutionError, FacetProposal, Output, Registry};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// Counts the bytes each thread holds, and the most it held since the count
/// was last reset. It counts per thread, so unit tests that run at the same
/// time do not mix their counts.
struct Counting;

thread_local! {
    static HELD: Cell<usize> = const { Cell::new(0) };
    static PEAK: Cell<usize> = const { Cell::new(0) };
}

fn grew(by: usize) {
    let _ = HELD.try_with(|held| {
        let now = held.get() + by;
        held.set(now);
        let _ = PEAK.try_with(|peak| peak.set(peak.get().max(now)));
    });
}

fn shrank(by: usize) {
    let _ = HELD.try_with(|held| held.set(held.get().saturating_sub(by)));
}

// SAFETY: every call goes straight to the system allocator; the counters
// only add and subtract sizes.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            grew(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            grew(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        shrank(layout.size());
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let moved = unsafe { System.realloc(pointer, layout, size) };
        if !moved.is_null() {
            // A reallocation that moves holds both blocks while it copies,
            // so count the new block before the old one goes.
            grew(size);
            shrank(layout.size());
        }
        moved
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Run `f`, and give its result with the most it held beyond what the
/// thread held before it.
pub(crate) fn measured<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = HELD.with(Cell::get);
    PEAK.with(|peak| peak.set(before));
    let outcome = f();
    (outcome, PEAK.with(Cell::get) - before)
}

pub(crate) const MIB: usize = 1024 * 1024;

/// Candidate bytes held in memory. `reads` counts the reads, and `furthest`
/// is the end of the furthest one.
pub(crate) struct Bytes {
    pub bytes: Vec<u8>,
    pub reads: usize,
    pub furthest: u64,
}

impl Bytes {
    pub(crate) fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            reads: 0,
            furthest: 0,
        }
    }
}

impl Candidate for Bytes {
    fn size(&self) -> u64 {
        self.bytes.len() as u64
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reads += 1;
        let start = (offset as usize).min(self.bytes.len());
        let n = buf.len().min(self.bytes.len() - start);
        buf[..n].copy_from_slice(&self.bytes[start..start + n]);
        self.furthest = self.furthest.max((start + n) as u64);
        Ok(n)
    }
}

/// An input held in memory, for the format modules' own tests.
pub(crate) struct Memory(pub Vec<u8>);

impl crate::io::ImageInput for Memory {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        let start = (offset as usize).min(self.0.len());
        let n = buf.len().min(self.0.len() - start);
        buf[..n].copy_from_slice(&self.0[start..start + n]);
        Ok(n)
    }
}

/// Run the preview job over `bytes`, a version of `media_type`, with a
/// 512 MiB budget. Gives the JPEG's length and the most the job held
/// beyond what the thread held before, input and output not counted.
pub(crate) fn measured_preview(
    bytes: Vec<u8>,
    media_type: &str,
) -> (Result<usize, crate::preview::JobFailure>, usize) {
    let size = bytes.len() as u64;
    let mut input = Memory(bytes);
    let mut output = Vec::with_capacity(64 * MIB);
    let (outcome, held) =
        measured(|| crate::preview::render(&mut input, size, media_type, 512 * MIB, &mut output));
    (outcome.map(|()| output.len()), held)
}

/// A host destination that drops every preview byte.
pub(crate) struct Discard;

impl Output for Discard {
    fn write(&mut self, _: &[u8]) -> std::io::Result<()> {
        Ok(())
    }
}

struct NoPreviews;

impl PreviewRunner for NoPreviews {
    fn run(
        &self,
        _: &mut sidereal_plugin_abi::AssetContext<'_>,
    ) -> Result<(), sidereal_plugin_abi::OperatorError> {
        Err(sidereal_plugin_abi::OperatorError::Failed(
            "this test runs no preview".into(),
        ))
    }
}

/// The image pack, with a runner that makes no preview.
pub(crate) fn pack() -> ImagePack {
    ImagePack::new(std::sync::Arc::new(NoPreviews))
}

/// Run the header Operator over `candidate` as a version of `media_type`.
pub(crate) fn read_header(
    candidate: &mut Bytes,
    media_type: &str,
) -> Result<Vec<FacetProposal>, ExecutionError> {
    let mut registry = Registry::new();
    registry.register_pack(&pack()).unwrap();
    registry
        .execute_operator(
            READ_HEADER,
            candidate,
            media_type,
            &mut Discard,
            &mut |_| {},
            &|| false,
        )
        .map(|run| run.proposals)
}

/// The CRC-32 of a PNG chunk's type and data (ISO 3309), worked bit by bit.
pub(crate) fn crc32(bytes: &[u8]) -> u32 {
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

/// One PNG chunk: its length, type, data and CRC-32.
pub(crate) fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = (data.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut covered = kind.to_vec();
    covered.extend_from_slice(data);
    out.extend_from_slice(&crc32(&covered).to_be_bytes());
    out
}

/// One JPEG segment: `0xFF`, its marker, its length and `payload`.
pub(crate) fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    assert!(
        payload.len() <= 65_533,
        "a segment holds at most 65,533 bytes"
    );
    let mut out = vec![0xff, marker];
    out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// A one-component baseline frame header of `width` by `height`.
pub(crate) fn frame(width: u16, height: u16) -> Vec<u8> {
    let mut payload = vec![8];
    payload.extend_from_slice(&height.to_be_bytes());
    payload.extend_from_slice(&width.to_be_bytes());
    payload.extend_from_slice(&[1, 1, 0x11, 0]);
    segment(0xc0, &payload)
}

/// The start-of-scan segment for the one component.
pub(crate) fn scan() -> Vec<u8> {
    segment(0xda, &[1, 1, 0x00, 0, 63, 0])
}

/// A JPEG: start-of-image, `before`, a 640 by 480 baseline frame header, a
/// scan, some image data and end-of-image.
pub(crate) fn jpeg_with(before: &[u8]) -> Vec<u8> {
    let mut out = vec![0xff, 0xd8];
    out.extend_from_slice(before);
    out.extend(frame(640, 480));
    out.extend(scan());
    out.extend_from_slice(&[0x12, 0x34, 0x56, 0xff, 0xd9]);
    out
}

/// An `APP2` ICC profile segment carrying `data`.
pub(crate) fn icc(data: &[u8]) -> Vec<u8> {
    let mut payload = b"ICC_PROFILE\0\x01\x01".to_vec();
    payload.extend_from_slice(data);
    segment(0xe2, &payload)
}

/// `APP2` ICC segments of at most 65,533 payload bytes, `total` bytes in all.
pub(crate) fn icc_segments(total: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(total + 65_537);
    while out.len() < total {
        out.extend(icc(&[0x55; 65_519]));
    }
    out
}

/// Comment segments of exactly `total` bytes, which must be 0 or at least 4.
pub(crate) fn padding(mut total: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(total);
    while total > 0 {
        let take = if total > 65_537 + 4 {
            65_537
        } else {
            total.min(65_537)
        };
        let take = if total - take > 0 && total - take < 4 {
            take - 4
        } else {
            take
        };
        out.extend(segment(0xfe, &vec![b'x'; take - 4]));
        total -= take;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The memory tests' bounds mean something only if the allocator counts.
    #[test]
    fn the_counting_allocator_sees_what_a_run_holds() {
        let (len, held) = measured(|| vec![1u8; 2 * MIB].len());
        assert_eq!(len, 2 * MIB);
        assert!(held >= 2 * MIB, "it counted {held} bytes");
    }
}
