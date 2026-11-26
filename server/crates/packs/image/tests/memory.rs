//! What one header run holds in memory, for crafted headers made to make the
//! decoders keep as much as they can. This file is its own test binary, so
//! its counting allocator sees only these tests. It counts per thread, so
//! tests that run at the same time do not mix their counts.
use sidereal_pack_image::{ImagePack, READ_HEADER};
use sidereal_plugin_abi::{Candidate, ExecutionError, Registry};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// Counts the bytes each thread holds, and the most it held since the count
/// was last reset.
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

const MIB: usize = 1024 * 1024;

/// Candidate bytes held in memory, read in full.
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

/// Run the header Operator over `bytes`. Gives the outcome and the most
/// the run held beyond what the thread held before it.
fn measured_run(bytes: Vec<u8>) -> (Result<usize, ExecutionError>, usize) {
    let mut registry = Registry::new();
    registry.register_pack(&ImagePack).unwrap();
    let media_type = if bytes.starts_with(b"\x89PNG") {
        "image/png"
    } else {
        "image/jpeg"
    };
    let mut candidate = Bytes(bytes);
    let before = HELD.with(Cell::get);
    PEAK.with(|peak| peak.set(before));
    let outcome = registry
        .execute_operator(
            READ_HEADER,
            &mut candidate,
            media_type,
            &mut |_| {},
            &|| false,
        )
        .map(|run| run.proposals.len());
    let held = PEAK.with(Cell::get) - before;
    (outcome, held)
}

/// The CRC-32 of a PNG chunk's type and data (ISO 3309), worked bit by bit.
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

/// One PNG chunk: its length, type, data and CRC-32.
fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = (data.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut covered = kind.to_vec();
    covered.extend_from_slice(data);
    out.extend_from_slice(&crc32(&covered).to_be_bytes());
    out
}

/// One JPEG segment: `0xFF`, its marker, its length and `payload`.
fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![0xff, marker];
    out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// A JPEG: start-of-image, `before`, a 640 by 480 baseline frame header, a
/// scan, some image data and end-of-image.
fn jpeg_with(before: &[u8]) -> Vec<u8> {
    let mut out = vec![0xff, 0xd8];
    out.extend_from_slice(before);
    out.extend(segment(0xc0, &[8, 0x01, 0xe0, 0x02, 0x80, 1, 1, 0x11, 0]));
    out.extend(segment(0xda, &[1, 1, 0x00, 0, 63, 0]));
    out.extend_from_slice(&[0x12, 0x34, 0x56, 0xff, 0xd9]);
    out
}

/// The worst case design.md decision 5 works out: 19-byte `APP2` ICC
/// segments, the smallest zune-jpeg keeps, fill the first 8 MiB. zune-jpeg
/// keeps an entry and an allocation for each, about 441,500 in all. The
/// design puts the run at about 30 to 40 MiB; this holds it to 40 MiB.
#[test]
fn a_jpeg_of_19_byte_icc_segments_filling_8_mib_holds_at_most_40_mib() {
    let mut icc = b"ICC_PROFILE\0\x01\x01".to_vec();
    icc.push(0x55);
    let one = segment(0xe2, &icc);
    assert_eq!(one.len(), 19);
    let bytes = jpeg_with(&one.repeat(8 * MIB / 19 - 1));
    assert!(bytes.len() <= 8 * MIB, "{} bytes", bytes.len());
    let (outcome, held) = measured_run(bytes);
    assert_eq!(outcome, Ok(2));
    eprintln!("JPEG run held {held} bytes");
    assert!(held <= 40 * MIB, "the run held {held} bytes");
}

/// The PNG decoder stops after `IHDR`, so 200,000 tiny text chunks after it
/// cost nothing: the run holds its two 64 KiB windows and the decoder's own
/// state, under 1 MiB.
#[test]
fn a_png_of_200000_text_chunks_holds_under_1_mib() {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend(chunk(
        b"IHDR",
        &[0, 0, 2, 0x80, 0, 0, 1, 0xe0, 8, 2, 0, 0, 0],
    ));
    let text = chunk(b"tEXt", b"k\0");
    for _ in 0..200_000 {
        bytes.extend_from_slice(&text);
    }
    let (outcome, held) = measured_run(bytes);
    assert_eq!(outcome, Ok(2));
    eprintln!("PNG run held {held} bytes");
    assert!(held < MIB, "the run held {held} bytes");
}
