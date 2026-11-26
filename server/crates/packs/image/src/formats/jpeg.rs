//! JPEG: its signature and its frame header, read within the first 8 MiB.
use super::ImageFormat;
use crate::io::CandidateReader;

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
        reader.limit(READ_LIMIT);
        let options = zune_jpeg::zune_core::options::DecoderOptions::default()
            .set_max_width(MAX_SIDE)
            .set_max_height(MAX_SIDE);
        let mut decoder = zune_jpeg::JpegDecoder::new_with_options(reader, options);
        decoder.decode_headers().ok()?;
        let (width, height) = decoder.dimensions()?;
        Some((u32::try_from(width).ok()?, u32::try_from(height).ok()?))
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

    #[test]
    fn a_jpeg_whose_scan_segment_ends_one_byte_past_the_limit_fails() {
        let mut candidate = Bytes::new(jpeg_whose_scan_ends_at(LIMIT + 1));
        assert_eq!(run(&mut candidate), unreadable());
        assert!(candidate.furthest <= READ_LIMIT, "{}", candidate.furthest);
    }
}
