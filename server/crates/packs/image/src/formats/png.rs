//! PNG: its signature and its `IHDR` header.
use super::ImageFormat;
use crate::{
    io::{CandidateReader, ImageInput},
    preview::{check_budget, check_sides, Channels, JobFailure, Layout, Pixels, Sample},
};

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

    fn render_preview(
        &self,
        input: &mut dyn ImageInput,
        size: u64,
        budget: usize,
    ) -> Result<Pixels, JobFailure> {
        let mut reader = CandidateReader::new(input, size);
        let pixels = decode(&mut reader, budget);
        if reader.failed() {
            return Err(JobFailure::Unreadable);
        }
        pixels
    }
}

/// A decoder refusal: its own limit is the budget, anything else is
/// damage.
fn failure(error: png::DecodingError) -> JobFailure {
    match error {
        png::DecodingError::LimitsExceeded => JobFailure::Budget,
        _ => JobFailure::Invalid,
    }
}

/// Decode a PNG of any color type and bit depth, expanded to 8 or 16 bits,
/// and require it to reach `IEND`. The decoder's resource limit is the
/// whole budget: the helper's heap enforces the real one.
fn decode(reader: &mut CandidateReader<'_>, budget: usize) -> Result<Pixels, JobFailure> {
    let mut decoder = png::Decoder::new_with_limits(reader, png::Limits { bytes: budget });
    decoder.set_transformations(png::Transformations::EXPAND);
    let (width, height) = {
        let info = decoder.read_header_info().map_err(failure)?;
        (info.width, info.height)
    };
    check_sides(width, height)?;
    let mut png = decoder.read_info().map_err(failure)?;
    let (color, depth) = png.output_color_type();
    let channels = match color {
        png::ColorType::Grayscale => Channels::Gray,
        png::ColorType::GrayscaleAlpha => Channels::GrayAlpha,
        png::ColorType::Rgb => Channels::Rgb,
        png::ColorType::Rgba => Channels::Rgba,
        png::ColorType::Indexed => return Err(JobFailure::Invalid),
    };
    let sample = match depth {
        png::BitDepth::Eight => Sample::U8,
        png::BitDepth::Sixteen => Sample::U16,
        _ => return Err(JobFailure::Invalid),
    };
    let layout = Layout { channels, sample };
    check_budget(layout, width, height, budget)?;
    let size = png.output_buffer_size().ok_or(JobFailure::Budget)?;
    let mut data = vec![0; size];
    png.next_frame(&mut data).map_err(failure)?;
    png.finish().map_err(failure)?;
    if sample == Sample::U16 {
        for pair in data.as_chunks_mut::<2>().0 {
            *pair = u16::from_be_bytes(*pair).to_ne_bytes();
        }
    }
    Ok(Pixels {
        width,
        height,
        layout,
        data,
    })
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

    /// An RGBA PNG's preview holds its decoded pixels once, plus the
    /// decoder's and the encoder's small working state.
    #[test]
    fn an_rgba_png_preview_holds_its_pixels_and_little_more() {
        let (width, height) = (1024u32, 1024u32);
        let pixels: Vec<u8> = (0..width * height * 4).map(|n| (n % 251) as u8).collect();
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&pixels).unwrap();
        writer.finish().unwrap();
        let (outcome, held) = measured_preview(bytes, "image/png");
        assert!(outcome.is_ok(), "{outcome:?}");
        let decoded = (width * height * 4) as usize;
        eprintln!("RGBA PNG preview held {held} bytes for {decoded} bytes of pixels");
        assert!(held >= decoded, "the count missed the pixels: {held}");
        assert!(held <= decoded + 2 * MIB, "the preview held {held} bytes");
    }
}
