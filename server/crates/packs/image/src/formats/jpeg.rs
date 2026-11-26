//! JPEG: its signature and its frame header, read within the first 8 MiB,
//! and its preview: a walk of the first image's markers to its end, then a
//! strict decode of the bytes through that end.
use super::ImageFormat;
use crate::{
    io::{CandidateReader, ImageInput},
    preview::{check_budget, check_sides, Channels, JobFailure, Layout, Pixels, Sample},
};
use zune_jpeg::zune_core::{colorspace::ColorSpace, options::DecoderOptions};

/// The largest width or height a JPEG frame header can hold. zune-jpeg's own
/// limit is 16,384, which a large mosaic passes.
const MAX_SIDE: usize = u16::MAX as usize;

/// How far into a JPEG the header read goes: its header, through the end of
/// its first start-of-scan segment, must fit in the first 8 MiB. This bounds
/// a run's reads and what zune-jpeg keeps from the segments it passes.
pub(crate) const READ_LIMIT: u64 = 8 * 1024 * 1024;

pub(crate) struct Jpeg;

impl ImageFormat for Jpeg {
    fn name(&self) -> &'static str {
        "JPEG"
    }

    fn media_type(&self) -> &'static str {
        "image/jpeg"
    }

    fn signatures(&self) -> &'static [&'static [u8]] {
        &[b"\xff\xd8\xff"]
    }

    /// The width and height a JPEG's frame header gives. zune-jpeg reads the
    /// markers through the first start-of-scan segment, skips the body of
    /// each segment it does not keep, and reads 8-bit baseline, extended and
    /// progressive frame headers only.
    fn dimensions(&self, reader: &mut CandidateReader<'_>) -> Option<(u32, u32)> {
        header(reader).map(|header| (header.width, header.height))
    }

    fn render_preview(
        &self,
        input: &mut dyn ImageInput,
        size: u64,
        budget: usize,
    ) -> Result<Pixels, JobFailure> {
        let header = {
            let mut reader = CandidateReader::new(input, size);
            let header = header(&mut reader);
            if reader.failed() {
                return Err(JobFailure::Unreadable);
            }
            header.ok_or(JobFailure::Invalid)?
        };
        check_sides(header.width, header.height)?;
        let channels = if header.gray {
            Channels::Gray
        } else {
            Channels::Rgb
        };
        let layout = Layout {
            channels,
            sample: Sample::U8,
        };
        check_budget(layout, header.width, header.height, budget)?;
        let end = walk(input, size)?;
        let mut reader = CandidateReader::new(input, end);
        let pixels = decode(&mut reader, layout, header.width, header.height);
        if reader.failed() {
            return Err(JobFailure::Unreadable);
        }
        pixels
    }
}

/// What the frame header says, read as the header Operator reads it.
struct Header {
    width: u32,
    height: u32,
    /// One component: the preview is gray.
    gray: bool,
}

fn options() -> DecoderOptions {
    DecoderOptions::default()
        .set_max_width(MAX_SIDE)
        .set_max_height(MAX_SIDE)
}

/// Read the markers through the first start-of-scan segment, within the
/// first 8 MiB.
fn header(reader: &mut CandidateReader<'_>) -> Option<Header> {
    reader.limit(READ_LIMIT);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(reader, options());
    decoder.decode_headers().ok()?;
    let (width, height) = decoder.dimensions()?;
    Some(Header {
        width: u32::try_from(width).ok()?,
        height: u32::try_from(height).ok()?,
        gray: decoder.input_colorspace()? == ColorSpace::Luma,
    })
}

/// Decode every pixel with unmodified zune-jpeg in strict mode, from a view
/// that ends after the first image's EOI.
fn decode(
    reader: &mut CandidateReader<'_>,
    layout: Layout,
    width: u32,
    height: u32,
) -> Result<Pixels, JobFailure> {
    let colorspace = if layout.channels == Channels::Gray {
        ColorSpace::Luma
    } else {
        ColorSpace::RGB
    };
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(
        reader,
        options()
            .set_strict_mode(true)
            .jpeg_set_out_colorspace(colorspace),
    );
    decoder.decode_headers().map_err(|_| JobFailure::Invalid)?;
    let size = layout.image_size(width, height).ok_or(JobFailure::Budget)?;
    if decoder.output_buffer_size() != Some(size) {
        return Err(JobFailure::Invalid);
    }
    let mut data = vec![0; size];
    decoder
        .decode_into(&mut data)
        .map_err(|_| JobFailure::Invalid)?;
    Ok(Pixels {
        width,
        height,
        layout,
        data,
    })
}

/// The marker walk's one fixed buffer.
const WALK_BUFFER: usize = 64 * 1024;

/// Reads the input forward, one byte at a time, through one fixed buffer.
struct Bytes<'i> {
    input: &'i mut dyn ImageInput,
    size: u64,
    buffer: Vec<u8>,
    /// The input offset of `buffer[0]`.
    start: u64,
    len: usize,
    at: usize,
}

impl<'i> Bytes<'i> {
    fn new(input: &'i mut dyn ImageInput, size: u64) -> Self {
        Self {
            input,
            size,
            buffer: vec![0; WALK_BUFFER],
            start: 0,
            len: 0,
            at: 0,
        }
    }

    fn offset(&self) -> u64 {
        self.start + self.at as u64
    }

    /// The next byte; the end of the input before EOI is damage.
    fn next(&mut self) -> Result<u8, JobFailure> {
        if self.at == self.len {
            self.start += self.len as u64;
            self.at = 0;
            let left = self.size.saturating_sub(self.start).min(WALK_BUFFER as u64) as usize;
            if left == 0 {
                return Err(JobFailure::Invalid);
            }
            self.len = self
                .input
                .read_at(self.start, &mut self.buffer[..left])
                .map_err(|_| JobFailure::Unreadable)?;
            if self.len == 0 {
                return Err(JobFailure::Invalid);
            }
        }
        let byte = self.buffer[self.at];
        self.at += 1;
        Ok(byte)
    }

    fn u16(&mut self) -> Result<u16, JobFailure> {
        Ok(u16::from(self.next()?) << 8 | u16::from(self.next()?))
    }

    /// Skip `count` bytes, which must lie within the input.
    fn skip(&mut self, count: u64) -> Result<(), JobFailure> {
        let to = self
            .offset()
            .checked_add(count)
            .filter(|&to| to <= self.size)
            .ok_or(JobFailure::Invalid)?;
        let buffered = (self.len - self.at) as u64;
        if count <= buffered {
            self.at += count as usize;
        } else {
            self.start = to;
            self.len = 0;
            self.at = 0;
        }
        Ok(())
    }

    /// A segment's length field, which counts itself.
    fn length(&mut self) -> Result<u16, JobFailure> {
        let length = self.u16()?;
        if length < 2 {
            return Err(JobFailure::Invalid);
        }
        Ok(length)
    }
}

/// What the walk keeps of the first frame header.
struct Frame {
    /// SOF0 or SOF1: baseline or extended sequential.
    sequential: bool,
    components: u8,
    /// A component has a sampling factor above 1.
    subsampled: bool,
}

/// Walk the first image's markers from its SOI to its first EOI, and give
/// the offset just past that EOI.
///
/// Outside a scan it skips `0xFF` fill bytes and steps over each segment
/// by its length, so a thumbnail inside an `APP1` segment, with its own
/// EOI, is never read as markers. In a scan, `0xFF 0x00` is a stuffed byte,
/// `0xFF 0xFF` fill and RST0 to RST7 continue it; any other marker ends it.
/// A second SOI, a marker it does not know, or the end of the input before
/// EOI is damage. A sequential frame with subsampling whose first scan
/// holds fewer components than the frame is a layout the decoder reads
/// wrongly, and is refused as unsupported.
fn walk(input: &mut dyn ImageInput, size: u64) -> Result<u64, JobFailure> {
    let mut bytes = Bytes::new(input, size);
    if bytes.next()? != 0xff || bytes.next()? != 0xd8 {
        return Err(JobFailure::Invalid);
    }
    let mut frame: Option<Frame> = None;
    let mut scanned = false;
    let mut pending: Option<u8> = None;
    loop {
        let marker = match pending.take() {
            Some(marker) => marker,
            None => {
                if bytes.next()? != 0xff {
                    return Err(JobFailure::Invalid);
                }
                let mut marker = bytes.next()?;
                while marker == 0xff {
                    marker = bytes.next()?;
                }
                marker
            }
        };
        match marker {
            0xd9 => return Ok(bytes.offset()),
            // APPn, COM, DQT, DHT, DRI and DNL.
            0xe0..=0xef | 0xfe | 0xdb | 0xc4 | 0xdd | 0xdc => {
                let length = bytes.length()?;
                bytes.skip(u64::from(length - 2))?;
            }
            // SOF0 to SOF15, but not DHT (C4), JPG (C8) or DAC (CC).
            0xc0..=0xcf if ![0xc4, 0xc8, 0xcc].contains(&marker) => {
                let length = bytes.length()?;
                if length < 8 {
                    return Err(JobFailure::Invalid);
                }
                let _precision = bytes.next()?;
                let _lines = bytes.u16()?;
                let _samples = bytes.u16()?;
                let components = bytes.next()?;
                if u16::from(components) * 3 + 8 != length {
                    return Err(JobFailure::Invalid);
                }
                let mut subsampled = false;
                for _ in 0..components {
                    let _id = bytes.next()?;
                    let sampling = bytes.next()?;
                    let _table = bytes.next()?;
                    subsampled |= sampling >> 4 > 1 || sampling & 0x0f > 1;
                }
                frame.get_or_insert(Frame {
                    sequential: matches!(marker, 0xc0 | 0xc1),
                    components,
                    subsampled,
                });
            }
            0xda => {
                let length = bytes.length()?;
                if length < 3 {
                    return Err(JobFailure::Invalid);
                }
                let in_scan = bytes.next()?;
                if !scanned {
                    scanned = true;
                    if let Some(frame) = &frame {
                        if frame.sequential && frame.subsampled && in_scan < frame.components {
                            return Err(JobFailure::Unsupported);
                        }
                    }
                }
                bytes.skip(u64::from(length - 3))?;
                pending = Some(scan(&mut bytes)?);
            }
            _ => return Err(JobFailure::Invalid),
        }
    }
}

/// Read a scan's entropy-coded data up to the marker that ends it.
fn scan(bytes: &mut Bytes<'_>) -> Result<u8, JobFailure> {
    loop {
        if bytes.next()? != 0xff {
            continue;
        }
        let mut next = bytes.next()?;
        while next == 0xff {
            next = bytes.next()?;
        }
        match next {
            0x00 | 0xd0..=0xd7 => continue,
            marker => return Ok(marker),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::READ_LIMIT;
    use crate::testing::*;
    use sidereal_plugin_abi::ExecutionError;

    const LIMIT: usize = READ_LIMIT as usize;

    fn unreadable() -> Result<usize, ExecutionError> {
        Err(ExecutionError::Failed(
            "the JPEG header could not be read".into(),
        ))
    }

    fn run(candidate: &mut Bytes) -> Result<usize, ExecutionError> {
        read_header(candidate, "image/jpeg").map(|proposals| proposals.len())
    }

    /// The worst case: 19-byte `APP2` ICC segments, the smallest zune-jpeg
    /// keeps, fill the first 8 MiB. zune-jpeg keeps an entry and an
    /// allocation for each, about 441,500 in all. This holds the run to
    /// 40 MiB.
    #[test]
    fn a_jpeg_of_19_byte_icc_segments_filling_8_mib_holds_at_most_40_mib() {
        let one = icc(&[0x55]);
        assert_eq!(one.len(), 19);
        let bytes = jpeg_with(&one.repeat(8 * MIB / 19 - 1));
        assert!(bytes.len() <= 8 * MIB, "{} bytes", bytes.len());
        let mut candidate = Bytes::new(bytes);
        let (outcome, held) = measured(|| run(&mut candidate));
        assert_eq!(outcome, Ok(2));
        assert!(held <= 40 * MIB, "the run held {held} bytes");
    }

    #[test]
    fn a_jpeg_with_9_mib_of_icc_segments_before_its_frame_header_fails_within_8_mib() {
        let mut candidate = Bytes::new(jpeg_with(&icc_segments(9 * MIB)));
        assert_eq!(run(&mut candidate), unreadable());
        assert!(candidate.furthest <= READ_LIMIT, "{}", candidate.furthest);
    }

    #[test]
    fn a_jpeg_of_19_byte_icc_segments_past_8_mib_fails_within_8_mib() {
        let mut candidate = Bytes::new(jpeg_with(&icc(&[0x55]).repeat(LIMIT / 19 + 1)));
        assert_eq!(run(&mut candidate), unreadable());
        assert!(candidate.furthest <= READ_LIMIT, "{}", candidate.furthest);
    }

    /// A 640 by 480 JPEG whose first start-of-scan segment ends at byte
    /// `end`.
    fn jpeg_whose_scan_ends_at(end: usize) -> Vec<u8> {
        let fixed = 2 + frame(640, 480).len() + scan().len();
        let bytes = jpeg_with(&padding(end - fixed));
        assert_eq!(&bytes[end - scan().len()..end], &scan()[..]);
        bytes
    }

    #[test]
    fn a_jpeg_whose_scan_segment_ends_at_the_limit_gives_its_width_and_height() {
        assert_eq!(run(&mut Bytes::new(jpeg_whose_scan_ends_at(LIMIT))), Ok(2));
    }

    /// A JPEG of `width` by `height` from `jpeg-encoder`.
    fn encoded(width: u16, height: u16, progressive: bool) -> Vec<u8> {
        let pixels: Vec<u8> = (0..usize::from(width) * usize::from(height) * 3)
            .map(|n| (n % 253) as u8)
            .collect();
        let mut out = Vec::new();
        let mut encoder = jpeg_encoder::Encoder::new(&mut out, 90);
        encoder.set_progressive(progressive);
        encoder
            .encode(&pixels, width, height, jpeg_encoder::ColorType::Rgb)
            .unwrap();
        out
    }

    /// A progressive JPEG's preview holds the decoder's coefficients for
    /// every block as well as the decoded pixels.
    #[test]
    fn a_progressive_jpeg_preview_holds_its_coefficients_and_pixels() {
        let (width, height) = (1024u16, 768u16);
        let (outcome, held) = measured_preview(encoded(width, height, true), "image/jpeg");
        assert!(outcome.is_ok(), "{outcome:?}");
        let decoded = usize::from(width) * usize::from(height) * 3;
        eprintln!("progressive JPEG preview held {held} bytes for {decoded} bytes of pixels");
        assert!(held >= decoded, "the count missed the pixels: {held}");
        assert!(
            held <= 4 * decoded + 2 * MIB,
            "the preview held {held} bytes"
        );
    }

    /// The 4:4:4 file with one scan per component and a DHT and a comment
    /// segment between its scans.
    #[test]
    fn a_444_jpeg_with_separate_component_scans_holds_its_coefficients_and_pixels() {
        let bytes = include_bytes!("../../tests/fixtures/jpeg-444-separate-scans.jpg").to_vec();
        let (outcome, held) = measured_preview(bytes, "image/jpeg");
        assert!(outcome.is_ok(), "{outcome:?}");
        let decoded = 640 * 480 * 3;
        eprintln!("separate-scan JPEG preview held {held} bytes for {decoded} bytes of pixels");
        assert!(held >= decoded, "the count missed the pixels: {held}");
        assert!(
            held <= 4 * decoded + 2 * MIB,
            "the preview held {held} bytes"
        );
    }

    #[test]
    fn a_jpeg_whose_scan_segment_ends_one_byte_past_the_limit_fails() {
        let mut candidate = Bytes::new(jpeg_whose_scan_ends_at(LIMIT + 1));
        assert_eq!(run(&mut candidate), unreadable());
        assert!(candidate.furthest <= READ_LIMIT, "{}", candidate.furthest);
    }
}
