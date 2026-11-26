//! The header Operator, through the public contract only: it reads an image's
//! pixel dimensions from its leading bytes.
use sidereal_pack_image::{ImagePack, HEIGHT_FACET, READ_HEADER, WIDTH_FACET};
use sidereal_plugin_abi::{
    Candidate, ExecutionError, FacetProposal, FacetSchema, FacetScope, FacetType, FacetValue,
    Registry,
};

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry.register_pack(&ImagePack).unwrap();
    registry
}

/// The schema the image pack declares under `name`, which it must own.
fn declared(name: &str) -> FacetSchema {
    let registry = registry();
    let declared = registry.facet_schema(name).unwrap();
    assert_eq!(declared.owner, "image");
    declared.schema.clone()
}

#[test]
fn registry_has_no_format_schema() {
    assert!(registry().facet_schema("image.format").is_none());
}

#[test]
fn schema_of_the_width_is_a_count_of_pixels() {
    assert_eq!(WIDTH_FACET, "image.width");
    assert_eq!(
        declared(WIDTH_FACET),
        FacetSchema {
            name: "image.width".into(),
            label: "Width".into(),
            value_type: FacetType::Count,
            unit: "pixels".into(),
            version: 1,
            scope: FacetScope::AssetVersion,
        }
    );
}

#[test]
fn schema_of_the_height_is_a_count_of_pixels() {
    assert_eq!(HEIGHT_FACET, "image.height");
    assert_eq!(
        declared(HEIGHT_FACET),
        FacetSchema {
            name: "image.height".into(),
            label: "Height".into(),
            value_type: FacetType::Count,
            unit: "pixels".into(),
            version: 1,
            scope: FacetScope::AssetVersion,
        }
    );
}

#[test]
fn schema_names_are_the_two_facets_the_header_operator_lists() {
    assert_eq!(READ_HEADER, "image.read-header");
    let registry = registry();
    let entry = registry.operator(READ_HEADER).unwrap();
    assert_eq!(entry.writes(), ["image.width", "image.height"]);
    assert_eq!(entry.version(), env!("CARGO_PKG_VERSION"));
}

/// Candidate bytes held in memory. Each read returns at most `most` bytes;
/// `reads` counts the reads and `furthest` is the end of the furthest one.
struct Bytes {
    bytes: Vec<u8>,
    most: usize,
    reads: usize,
    furthest: u64,
}

impl Bytes {
    fn new(bytes: Vec<u8>) -> Self {
        Self::reading_at_most(bytes, usize::MAX)
    }

    fn reading_at_most(bytes: Vec<u8>, most: usize) -> Self {
        Self {
            bytes,
            most,
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
        let n = buf.len().min(self.most).min(self.bytes.len() - start);
        buf[..n].copy_from_slice(&self.bytes[start..start + n]);
        self.furthest = self.furthest.max((start + n) as u64);
        Ok(n)
    }
}

/// The media type the Source records for `bytes`: PNG for a PNG
/// signature, otherwise JPEG.
fn media_type(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG") {
        "image/png"
    } else {
        "image/jpeg"
    }
}

/// Run the header Operator over `candidate`.
fn read_header(candidate: &mut Bytes) -> Result<Vec<FacetProposal>, ExecutionError> {
    let media_type = media_type(&candidate.bytes);
    registry()
        .execute_operator(READ_HEADER, candidate, media_type, &mut |_| {}, &|| false)
        .map(|run| run.proposals)
}

/// What the header Operator proposes for an image of `width` by `height`
/// pixels.
fn header(width: i64, height: i64) -> Vec<FacetProposal> {
    vec![
        FacetProposal {
            name: "image.width".into(),
            value: FacetValue::Integer(width),
        },
        FacetProposal {
            name: "image.height".into(),
            value: FacetValue::Integer(height),
        },
    ]
}

fn failed(reason: &str) -> Result<Vec<FacetProposal>, ExecutionError> {
    Err(ExecutionError::Failed(reason.into()))
}

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const PNG_UNREADABLE: &str = "the PNG header could not be read";

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

/// The 13 bytes of an 8-bit RGB `IHDR` of `width` by `height`.
fn ihdr_data(width: u32, height: u32) -> Vec<u8> {
    let mut data = width.to_be_bytes().to_vec();
    data.extend_from_slice(&height.to_be_bytes());
    data.extend_from_slice(&[8, 2, 0, 0, 0]);
    data
}

/// A PNG's signature and a valid `IHDR` of `width` by `height`, and no more.
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = PNG_SIGNATURE.to_vec();
    bytes.extend(chunk(b"IHDR", &ihdr_data(width, height)));
    bytes
}

#[test]
fn png_header_gives_its_width_and_height() {
    assert_eq!(
        read_header(&mut Bytes::new(png(4096, 2160))),
        Ok(header(4096, 2160))
    );
}

#[test]
fn png_with_200000_text_chunks_after_its_header_costs_one_read() {
    let mut bytes = png(640, 480);
    let text = chunk(b"tEXt", b"k\0");
    assert_eq!(text.len(), 14);
    for _ in 0..200_000 {
        bytes.extend_from_slice(&text);
    }
    let mut candidate = Bytes::new(bytes);
    assert_eq!(read_header(&mut candidate), Ok(header(640, 480)));
    assert_eq!(candidate.reads, 1);
}

#[test]
fn png_read_three_bytes_at_a_time_gives_the_same_answer() {
    assert_eq!(
        read_header(&mut Bytes::reading_at_most(png(4096, 2160), 3)),
        Ok(header(4096, 2160))
    );
}

#[test]
fn png_of_only_its_signature_fails() {
    assert_eq!(
        read_header(&mut Bytes::new(PNG_SIGNATURE.to_vec())),
        failed(PNG_UNREADABLE)
    );
}

#[test]
fn png_whose_first_chunk_is_not_a_13_byte_header_fails() {
    let mut text_first = PNG_SIGNATURE.to_vec();
    text_first.extend(chunk(b"tEXt", b"k\0v"));
    text_first.extend(chunk(b"IHDR", &ihdr_data(640, 480)));
    let mut short = PNG_SIGNATURE.to_vec();
    short.extend(chunk(b"IHDR", &ihdr_data(640, 480)[..12]));
    let mut long = PNG_SIGNATURE.to_vec();
    long.extend(chunk(b"IHDR", &[ihdr_data(640, 480), vec![0]].concat()));
    for (what, bytes) in [
        ("tEXt first", text_first),
        ("12 bytes", short),
        ("14 bytes", long),
    ] {
        assert_eq!(
            read_header(&mut Bytes::new(bytes)),
            failed(PNG_UNREADABLE),
            "{what}"
        );
    }
}

#[test]
fn png_whose_header_checksum_does_not_match_fails() {
    let bytes = png(4096, 2160);
    assert_eq!(
        read_header(&mut Bytes::new(bytes.clone())),
        Ok(header(4096, 2160))
    );
    let mut damaged = bytes;
    let last = damaged.len() - 1;
    damaged[last] ^= 0x01;
    assert_eq!(
        read_header(&mut Bytes::new(damaged)),
        failed(PNG_UNREADABLE)
    );
}

#[test]
fn png_cut_off_inside_its_header_fails() {
    let bytes = png(4096, 2160);
    assert_eq!(
        read_header(&mut Bytes::new(bytes[..bytes.len() - 6].to_vec())),
        failed(PNG_UNREADABLE)
    );
}

#[test]
fn png_with_a_zero_width_or_height_fails() {
    for (width, height) in [(0, 480), (640, 0)] {
        assert_eq!(
            read_header(&mut Bytes::new(png(width, height))),
            failed(PNG_UNREADABLE),
            "{width} by {height}"
        );
    }
}

#[test]
fn png_wider_or_taller_than_the_png_limit_fails() {
    for (width, height) in [(1 << 31, 480), (640, 1 << 31)] {
        assert_eq!(
            read_header(&mut Bytes::new(png(width, height))),
            failed(PNG_UNREADABLE),
            "{width} by {height}"
        );
    }
}

#[test]
fn neither_png_nor_jpeg_bytes_fail_as_unrecognized() {
    for bytes in [
        &b""[..],
        b"II*\x00\x08\x00\x00\x00",
        &PNG_SIGNATURE[..7],
        b"\xff\xd8",
    ] {
        assert_eq!(
            read_header(&mut Bytes::new(bytes.to_vec())),
            failed("the image format is not recognized"),
            "{bytes:?}"
        );
    }
}

/// A candidate whose every read fails.
struct FailsEveryRead(Bytes);

impl Candidate for FailsEveryRead {
    fn size(&self) -> u64 {
        self.0.size()
    }
    fn read_at(&mut self, _offset: u64, _buf: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("the disk could not be read"))
    }
}

/// A first read that fails is a storage problem, not an unknown format, so
/// the run says the file could not be read.
#[test]
fn candidate_whose_first_read_fails_fails_as_unreadable() {
    for bytes in [png(640, 480), jpeg(640, 480)] {
        let media_type = media_type(&bytes);
        let mut candidate = FailsEveryRead(Bytes::new(bytes));
        let outcome = registry()
            .execute_operator(
                READ_HEADER,
                &mut candidate,
                media_type,
                &mut |_| {},
                &|| false,
            )
            .map(|run| run.proposals);
        assert_eq!(outcome, failed("the stored file could not be read"));
    }
}

const JPEG_UNREADABLE: &str = "the JPEG header could not be read";

/// The JPEG read limit: 8 MiB.
const LIMIT: usize = 8 * 1024 * 1024;

/// One JPEG segment: `0xFF`, its marker, its length and `payload`.
fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    assert!(
        payload.len() <= 65_533,
        "a segment holds at most 65,533 bytes"
    );
    let mut out = vec![0xff, marker];
    out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// A one-component frame header of `kind` (`0xC0` to `0xC3`), at
/// `precision` bits, `width` by `height`.
fn frame(kind: u8, precision: u8, width: u16, height: u16) -> Vec<u8> {
    let mut payload = vec![precision];
    payload.extend_from_slice(&height.to_be_bytes());
    payload.extend_from_slice(&width.to_be_bytes());
    payload.extend_from_slice(&[1, 1, 0x11, 0]);
    segment(kind, &payload)
}

/// The start-of-scan segment for the one component.
fn scan() -> Vec<u8> {
    segment(0xda, &[1, 1, 0x00, 0, 63, 0])
}

/// Some entropy-coded bytes and the end-of-image marker, after the scan.
fn image_data() -> Vec<u8> {
    let mut out = vec![0x12, 0x34, 0x56];
    out.extend_from_slice(&[0xff, 0xd9]);
    out
}

/// A JPEG: the start-of-image marker, `before`, then `header`, a scan and
/// image data.
fn jpeg_with(before: &[u8], header: &[u8]) -> Vec<u8> {
    let mut out = vec![0xff, 0xd8];
    out.extend_from_slice(before);
    out.extend_from_slice(header);
    out.extend(scan());
    out.extend(image_data());
    out
}

/// A baseline JPEG of `width` by `height`.
fn jpeg(width: u16, height: u16) -> Vec<u8> {
    jpeg_with(&[], &frame(0xc0, 8, width, height))
}

/// An `APP1` EXIF segment of `length` payload bytes: a big-endian TIFF
/// header and one IFD whose orientation is `orientation`, then padding.
fn exif(length: usize, orientation: u16) -> Vec<u8> {
    let mut payload = b"Exif\0\0MM\0*\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01".to_vec();
    payload.extend_from_slice(&orientation.to_be_bytes());
    payload.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    payload.resize(length, 0);
    segment(0xe1, &payload)
}

/// An `APP2` ICC profile segment carrying `data`.
fn icc(data: &[u8]) -> Vec<u8> {
    let mut payload = b"ICC_PROFILE\0\x01\x01".to_vec();
    payload.extend_from_slice(data);
    segment(0xe2, &payload)
}

/// `APP2` ICC segments of at most 65,533 payload bytes, `total` bytes in all.
fn icc_segments(total: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(total + 65_537);
    while out.len() < total {
        out.extend(icc(&[0x55; 65_519]));
    }
    out
}

/// Comment segments of exactly `total` bytes, which must be 0 or at least 4.
fn padding(mut total: usize) -> Vec<u8> {
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

#[test]
fn jpeg_baseline_gives_its_width_and_height() {
    assert_eq!(
        read_header(&mut Bytes::new(jpeg(6000, 4000))),
        Ok(header(6000, 4000))
    );
}

#[test]
fn jpeg_progressive_gives_its_width_and_height() {
    assert_eq!(
        read_header(&mut Bytes::new(jpeg_with(&[], &frame(0xc2, 8, 1920, 1080)))),
        Ok(header(1920, 1080))
    );
}

#[test]
fn jpeg_wider_than_16384_pixels_gives_its_width_and_height() {
    assert_eq!(
        read_header(&mut Bytes::new(jpeg(20_000, 12_000))),
        Ok(header(20_000, 12_000))
    );
}

#[test]
fn jpeg_taller_than_16384_pixels_gives_its_width_and_height() {
    assert_eq!(
        read_header(&mut Bytes::new(jpeg(12_000, 20_000))),
        Ok(header(12_000, 20_000))
    );
}

#[test]
fn jpeg_with_exif_and_icc_before_its_frame_header_gives_its_width_and_height() {
    let before = [exif(60_000, 1), icc(&[0x55; 3_000])].concat();
    assert_eq!(
        read_header(&mut Bytes::new(jpeg_with(
            &before,
            &frame(0xc0, 8, 6000, 4000)
        ))),
        Ok(header(6000, 4000))
    );
}

#[test]
fn jpeg_rotated_by_its_exif_orientation_keeps_its_stored_width_and_height() {
    // Orientation 6: rotate 90 degrees clockwise to display.
    let before = exif(200, 6);
    assert_eq!(
        read_header(&mut Bytes::new(jpeg_with(
            &before,
            &frame(0xc0, 8, 6000, 4000)
        ))),
        Ok(header(6000, 4000))
    );
}

#[test]
fn jpeg_that_reaches_its_scan_or_end_before_a_frame_header_fails() {
    let mut scan_first = vec![0xff, 0xd8];
    scan_first.extend(scan());
    scan_first.extend(image_data());
    let end_first = b"\xff\xd8\xff\xd9".to_vec();
    for (what, bytes) in [("start-of-scan", scan_first), ("end-of-image", end_first)] {
        assert_eq!(
            read_header(&mut Bytes::new(bytes)),
            failed(JPEG_UNREADABLE),
            "{what}"
        );
    }
}

#[test]
fn jpeg_cut_off_inside_a_segment_fails() {
    let bytes = jpeg_with(&exif(1_000, 1), &frame(0xc0, 8, 640, 480));
    assert_eq!(
        read_header(&mut Bytes::new(bytes[..500].to_vec())),
        failed(JPEG_UNREADABLE)
    );
}

#[test]
fn jpeg_of_an_unsupported_kind_fails() {
    for (what, header) in [
        ("lossless", frame(0xc3, 8, 640, 480)),
        ("12-bit extended", frame(0xc1, 12, 640, 480)),
    ] {
        assert_eq!(
            read_header(&mut Bytes::new(jpeg_with(&[], &header))),
            failed(JPEG_UNREADABLE),
            "{what}"
        );
    }
}

#[test]
fn jpeg_with_a_zero_width_or_height_fails() {
    for (width, height) in [(0, 480), (640, 0)] {
        assert_eq!(
            read_header(&mut Bytes::new(jpeg(width, height))),
            failed(JPEG_UNREADABLE),
            "{width} by {height}"
        );
    }
}

#[test]
fn jpeg_with_9_mib_of_icc_segments_before_its_frame_header_fails_within_8_mib() {
    let before = icc_segments(9 * 1024 * 1024);
    let mut candidate = Bytes::new(jpeg_with(&before, &frame(0xc0, 8, 640, 480)));
    assert_eq!(read_header(&mut candidate), failed(JPEG_UNREADABLE));
    assert!(candidate.furthest <= LIMIT as u64, "{}", candidate.furthest);
}

#[test]
fn jpeg_of_19_byte_icc_segments_filling_8_mib_fails_within_8_mib() {
    let one = icc(&[0x55]);
    assert_eq!(one.len(), 19);
    let before = one.repeat(LIMIT / 19 + 1);
    let mut candidate = Bytes::new(jpeg_with(&before, &frame(0xc0, 8, 640, 480)));
    assert_eq!(read_header(&mut candidate), failed(JPEG_UNREADABLE));
    assert!(candidate.furthest <= LIMIT as u64, "{}", candidate.furthest);
}

/// A 640 by 480 JPEG whose first start-of-scan segment ends at byte `end`.
fn jpeg_whose_scan_ends_at(end: usize) -> Vec<u8> {
    let header = frame(0xc0, 8, 640, 480);
    let fixed = 2 + header.len() + scan().len();
    let bytes = jpeg_with(&padding(end - fixed), &header);
    assert_eq!(&bytes[end - scan().len()..end], &scan()[..]);
    bytes
}

#[test]
fn jpeg_whose_scan_segment_ends_at_the_limit_gives_its_width_and_height() {
    assert_eq!(
        read_header(&mut Bytes::new(jpeg_whose_scan_ends_at(LIMIT))),
        Ok(header(640, 480))
    );
}

#[test]
fn jpeg_whose_scan_segment_ends_one_byte_past_the_limit_fails() {
    let mut candidate = Bytes::new(jpeg_whose_scan_ends_at(LIMIT + 1));
    assert_eq!(read_header(&mut candidate), failed(JPEG_UNREADABLE));
    assert!(candidate.furthest <= LIMIT as u64, "{}", candidate.furthest);
}

/// The most reads a JPEG run over `size` bytes may cost with full reads.
fn read_bound(size: usize) -> usize {
    size.min(LIMIT) / 65_536 + 4
}

#[test]
fn jpeg_of_tiny_segments_and_fill_bytes_costs_a_bounded_number_of_reads() {
    let mut before = Vec::with_capacity(4 * 1024 * 1024 + 8);
    while before.len() < 4 * 1024 * 1024 {
        // Fill bytes, then a 4-byte segment with no payload.
        before.extend_from_slice(&[0xff, 0xff, 0xff, 0xe3, 0x00, 0x02]);
        before.extend_from_slice(&[0xff, 0xe4, 0x00, 0x02]);
    }
    let bytes = jpeg_with(&before, &frame(0xc0, 8, 640, 480));
    let size = bytes.len();
    let mut candidate = Bytes::new(bytes);
    assert_eq!(read_header(&mut candidate), Ok(header(640, 480)));
    assert!(
        candidate.reads <= read_bound(size),
        "{} reads",
        candidate.reads
    );
}

#[test]
fn jpeg_of_64_full_exif_segments_costs_a_bounded_number_of_reads() {
    let before = exif(65_533, 1).repeat(64);
    let bytes = jpeg_with(&before, &frame(0xc0, 8, 6000, 4000));
    let size = bytes.len();
    let mut candidate = Bytes::new(bytes);
    assert_eq!(read_header(&mut candidate), Ok(header(6000, 4000)));
    assert!(
        candidate.reads <= read_bound(size),
        "{} reads",
        candidate.reads
    );
}

#[test]
fn jpeg_whose_scan_lies_in_its_first_window_costs_at_most_2_reads() {
    let mut bytes = jpeg(6000, 4000);
    bytes.resize(4 * 1024 * 1024, 0x12);
    let mut candidate = Bytes::new(bytes);
    assert_eq!(read_header(&mut candidate), Ok(header(6000, 4000)));
    assert!(candidate.reads <= 2, "{} reads", candidate.reads);
}

/// A candidate that reports the run cancelled once it has been read.
struct CancelsAfterOneRead {
    bytes: Bytes,
    cancelled: std::rc::Rc<std::cell::Cell<bool>>,
}

impl Candidate for CancelsAfterOneRead {
    fn size(&self) -> u64 {
        self.bytes.size()
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cancelled.set(true);
        self.bytes.read_at(offset, buf)
    }
}

#[test]
fn jpeg_run_cancelled_by_its_host_stops_reading() {
    let before = icc_segments(4 * 1024 * 1024);
    let cancelled = std::rc::Rc::new(std::cell::Cell::new(false));
    let mut candidate = CancelsAfterOneRead {
        bytes: Bytes::new(jpeg_with(&before, &frame(0xc0, 8, 640, 480))),
        cancelled: cancelled.clone(),
    };
    let outcome = registry().execute_operator(
        READ_HEADER,
        &mut candidate,
        "image/jpeg",
        &mut |_| {},
        &|| cancelled.get(),
    );
    assert_eq!(outcome, Err(ExecutionError::Cancelled));
    assert_eq!(candidate.bytes.reads, 1);
}

/// zune-jpeg reads a segment's length with one read call and takes missing
/// bytes as zero, so a read must not stop at the end of a window.
#[test]
fn jpeg_whose_segment_lengths_cross_a_window_boundary_gives_its_width_and_height() {
    let before = icc(&[0x55]).repeat(10_000);
    assert_eq!(
        read_header(&mut Bytes::new(jpeg_with(
            &before,
            &frame(0xc0, 8, 640, 480)
        ))),
        Ok(header(640, 480))
    );
}

#[test]
fn jpeg_read_three_bytes_at_a_time_gives_the_same_answer() {
    let before = [exif(60_000, 1), icc(&[0x55]).repeat(5_000)].concat();
    assert_eq!(
        read_header(&mut Bytes::reading_at_most(
            jpeg_with(&before, &frame(0xc0, 8, 6000, 4000)),
            3
        )),
        Ok(header(6000, 4000))
    );
}

/// A candidate whose second read fails and whose other reads succeed.
struct FailsOnSecondRead(Bytes);

impl Candidate for FailsOnSecondRead {
    fn size(&self) -> u64 {
        self.0.size()
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.0.reads == 1 {
            self.0.reads += 1;
            return Err(std::io::Error::other("the disk could not be read"));
        }
        self.0.read_at(offset, buf)
    }
}

/// A read that fails part way through a segment must not let the decoder go
/// on with bytes the candidate never gave, so the run fails.
#[test]
fn jpeg_whose_candidate_read_fails_fails() {
    for before in [
        icc(&[0x55]).repeat(10_000),
        icc_segments(256 * 1024),
        padding(64 * 1024 - 3),
    ] {
        let mut candidate =
            FailsOnSecondRead(Bytes::new(jpeg_with(&before, &frame(0xc0, 8, 640, 480))));
        let outcome = registry()
            .execute_operator(
                READ_HEADER,
                &mut candidate,
                "image/jpeg",
                &mut |_| {},
                &|| false,
            )
            .map(|run| run.proposals);
        assert_eq!(
            outcome,
            failed(JPEG_UNREADABLE),
            "{} bytes before",
            before.len()
        );
    }
}
