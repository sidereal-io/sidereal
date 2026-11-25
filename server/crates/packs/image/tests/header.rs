//! The header Operator, through the public contract only: it reads an image's
//! format and pixel dimensions from its leading bytes.
use sidereal_pack_image::{ImagePack, FORMAT_FACET, HEIGHT_FACET, READ_HEADER, WIDTH_FACET};
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
fn schema_of_the_format_is_a_choice_of_png_or_jpeg() {
    assert_eq!(FORMAT_FACET, "image.format");
    assert_eq!(
        declared(FORMAT_FACET),
        FacetSchema {
            name: "image.format".into(),
            label: "Format".into(),
            value_type: FacetType::Choice(vec!["PNG".into(), "JPEG".into()]),
            unit: String::new(),
            version: 1,
            scope: FacetScope::AssetVersion,
        }
    );
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
fn schema_names_are_the_three_facets_the_header_operator_lists() {
    assert_eq!(READ_HEADER, "image.read-header");
    let registry = registry();
    let entry = registry.operator(READ_HEADER).unwrap();
    assert_eq!(
        entry.writes(),
        ["image.format", "image.width", "image.height"]
    );
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

/// Run the header Operator over `candidate`.
fn read_header(candidate: &mut Bytes) -> Result<Vec<FacetProposal>, ExecutionError> {
    registry()
        .execute_operator(READ_HEADER, candidate, &mut |_| {}, &|| false)
        .map(|run| run.proposals)
}

/// What the header Operator proposes for an image of `format`, `width` by
/// `height` pixels.
fn header(format: &str, width: i64, height: i64) -> Vec<FacetProposal> {
    vec![
        FacetProposal {
            name: "image.format".into(),
            value: FacetValue::Text(format.into()),
        },
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
fn png_header_gives_its_format_width_and_height() {
    assert_eq!(
        read_header(&mut Bytes::new(png(4096, 2160))),
        Ok(header("PNG", 4096, 2160))
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
    assert_eq!(read_header(&mut candidate), Ok(header("PNG", 640, 480)));
    assert_eq!(candidate.reads, 1);
}

#[test]
fn png_read_three_bytes_at_a_time_gives_the_same_answer() {
    assert_eq!(
        read_header(&mut Bytes::reading_at_most(png(4096, 2160), 3)),
        Ok(header("PNG", 4096, 2160))
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
        Ok(header("PNG", 4096, 2160))
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
