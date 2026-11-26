//! PNG: its signature and its `IHDR` header.
use super::ImageFormat;
use crate::io::CandidateReader;

/// The largest width or height a PNG header may hold.
const MAX_SIDE: u32 = i32::MAX as u32;

pub(crate) struct Png;

impl ImageFormat for Png {
    fn name(&self) -> &'static str {
        "PNG"
    }

    fn media_type(&self) -> &'static str {
        "image/png"
    }

    fn signatures(&self) -> &'static [&'static [u8]] {
        &[b"\x89PNG\r\n\x1a\n"]
    }

    /// The width and height a PNG's `IHDR` chunk gives. The decoder reads
    /// the signature and `IHDR`, checks its CRC-32 and stops: no chunk after
    /// it is read.
    fn dimensions(&self, reader: &mut CandidateReader<'_>) -> Option<(u32, u32)> {
        let mut decoder = png::Decoder::new(reader);
        let info = decoder.read_header_info().ok()?;
        let (width, height) = (info.width, info.height);
        (width <= MAX_SIDE && height <= MAX_SIDE).then_some((width, height))
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::*;

    /// The PNG decoder stops after `IHDR`, so 200,000 tiny text chunks after
    /// it cost nothing: the run holds its two 64 KiB windows and the
    /// decoder's own state, under 1 MiB.
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
        let mut candidate = Bytes::new(bytes);
        let (outcome, held) =
            measured(|| read_header(&mut candidate, "image/png").map(|p| p.len()));
        assert_eq!(outcome, Ok(2));
        assert!(held < MIB, "the run held {held} bytes");
    }
}
