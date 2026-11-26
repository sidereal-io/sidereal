//! TIFF: classic and BigTIFF, in either byte order. The header read takes
//! the first image's width and length and nothing else, so every TIFF
//! records its dimensions, whatever its type. The preview reads the first
//! image's other tags with its own directory reader, checks them, and only
//! then hands the file to the `tiff` decoder.
use super::ImageFormat;
use crate::{
    io::{CandidateReader, ImageInput},
    preview::{check_budget, check_sides, Channels, JobFailure, Layout, Pixels, Sample},
};
use std::io::{Seek, SeekFrom};

mod ifd;

const IMAGE_WIDTH: u16 = 256;
const IMAGE_LENGTH: u16 = 257;
const COMPRESSION: u16 = 259;
const PHOTOMETRIC: u16 = 262;
const STRIP_OFFSETS: u16 = 273;
const SAMPLES_PER_PIXEL: u16 = 277;
const ROWS_PER_STRIP: u16 = 278;
const STRIP_BYTE_COUNTS: u16 = 279;
const PLANAR_CONFIGURATION: u16 = 284;
const PREDICTOR: u16 = 317;
const TILE_WIDTH: u16 = 322;
const TILE_LENGTH: u16 = 323;
const TILE_OFFSETS: u16 = 324;
const TILE_BYTE_COUNTS: u16 = 325;

/// The tags that describe a pixel's samples, which the preview reads.
pub(crate) const BITS_PER_SAMPLE: u16 = 258;
pub(crate) const EXTRA_SAMPLES: u16 = 338;
pub(crate) const SAMPLE_FORMAT: u16 = 339;

/// The most values the preview reads from an array of sample properties.
pub(crate) const MAX_SAMPLE_VALUES: u64 = 64;

/// Every tag the preview's check reads.
const PREVIEW_TAGS: &[u16] = &[
    IMAGE_WIDTH,
    IMAGE_LENGTH,
    BITS_PER_SAMPLE,
    COMPRESSION,
    PHOTOMETRIC,
    STRIP_OFFSETS,
    SAMPLES_PER_PIXEL,
    ROWS_PER_STRIP,
    STRIP_BYTE_COUNTS,
    PLANAR_CONFIGURATION,
    PREDICTOR,
    TILE_WIDTH,
    TILE_LENGTH,
    TILE_OFFSETS,
    TILE_BYTE_COUNTS,
    EXTRA_SAMPLES,
    SAMPLE_FORMAT,
];

/// The compressions the preview decodes: none, LZW, Deflate (both codes)
/// and PackBits.
const NONE: u64 = 1;
const LZW: u64 = 5;
const DEFLATE: [u64; 2] = [8, 32946];
const PACKBITS: u64 = 32773;

pub(crate) struct Tiff;

impl ImageFormat for Tiff {
    fn name(&self) -> &'static str {
        "TIFF"
    }

    fn media_type(&self) -> &'static str {
        "image/tiff"
    }

    fn signatures(&self) -> &'static [&'static [u8]] {
        &[b"II*\0", b"MM\0*", b"II+\0", b"MM\0+"]
    }

    /// The first image's width and length. Every other value in the
    /// directory is skipped unread, however long its array.
    fn dimensions(&self, reader: &mut CandidateReader<'_>) -> Option<(u32, u32)> {
        let size = reader.seek(SeekFrom::End(0)).ok()?;
        let directory = ifd::first_directory(reader, size, &[IMAGE_WIDTH, IMAGE_LENGTH])?;
        let width = u32::try_from(directory.scalar(IMAGE_WIDTH)?).ok()?;
        let length = u32::try_from(directory.scalar(IMAGE_LENGTH)?).ok()?;
        Some((width, length))
    }

    fn render_preview(
        &self,
        input: &mut dyn ImageInput,
        size: u64,
        budget: usize,
    ) -> Result<Pixels, JobFailure> {
        let plan = {
            let mut reader = CandidateReader::new(input, size);
            let plan = plan(&mut reader, size, budget);
            if reader.failed() {
                return Err(JobFailure::Unreadable);
            }
            plan?
        };
        let mut reader = CandidateReader::new(input, size);
        let pixels = decode(&mut reader, plan, budget);
        if reader.failed() {
            return Err(JobFailure::Unreadable);
        }
        pixels
    }
}

/// What the check found: the first image's size and how its pixels are
/// laid out once decoded.
#[derive(Clone, Copy, Debug)]
struct Plan {
    width: u32,
    height: u32,
    layout: Layout,
}

/// Every value of `tag`, which must all be the same, and how many there
/// are; `default` and 0 when the directory has none.
fn same_values(
    directory: &ifd::Directory,
    reader: &mut CandidateReader<'_>,
    tag: u16,
    default: u64,
) -> Result<(u64, usize), JobFailure> {
    let values = directory
        .array(reader, tag, MAX_SAMPLE_VALUES)
        .map_err(|error| match error {
            ifd::ArrayError::TooLong => JobFailure::Unsupported,
            ifd::ArrayError::Unreadable => JobFailure::Invalid,
        })?;
    match values {
        None => Ok((default, 0)),
        Some(values) => match values.split_first() {
            Some((&first, rest)) if rest.iter().all(|&v| v == first) => Ok((first, values.len())),
            Some(_) => Err(JobFailure::Unsupported),
            None => Ok((default, 0)),
        },
    }
}

/// Check the first image before any pixel is decoded: its sides first,
/// then that its type is one the preview decodes and its strips or tiles
/// match its size, then that its pixels and one decoded row of a strip or
/// tile fit `budget`.
fn plan(reader: &mut CandidateReader<'_>, size: u64, budget: usize) -> Result<Plan, JobFailure> {
    let directory = ifd::first_directory(reader, size, PREVIEW_TAGS).ok_or(JobFailure::Invalid)?;
    let side = |tag| {
        directory
            .scalar(tag)
            .and_then(|v| u32::try_from(v).ok())
            .ok_or(JobFailure::Invalid)
    };
    let (width, height) = (side(IMAGE_WIDTH)?, side(IMAGE_LENGTH)?);
    check_sides(width, height)?;
    let unsupported = |ok: bool| {
        if ok {
            Ok(())
        } else {
            Err(JobFailure::Unsupported)
        }
    };
    let scalar = |tag, default| match directory.count(tag) {
        None => Ok(default),
        Some(_) => directory.scalar(tag).ok_or(JobFailure::Invalid),
    };
    let samples = scalar(SAMPLES_PER_PIXEL, 1)?;
    let (bits, bits_listed) = same_values(&directory, reader, BITS_PER_SAMPLE, 1)?;
    let (format, formats_listed) = same_values(&directory, reader, SAMPLE_FORMAT, 1)?;
    let (_, extra) = same_values(&directory, reader, EXTRA_SAMPLES, 0)?;
    unsupported(extra == 0)?;
    unsupported(bits_listed == 0 || bits_listed as u64 == samples)?;
    unsupported(formats_listed == 0 || formats_listed as u64 == samples)?;
    let channels = match (directory.scalar(PHOTOMETRIC), samples) {
        (Some(0 | 1), 1) => Channels::Gray,
        (Some(2), 3) => Channels::Rgb,
        (None, _) => return Err(JobFailure::Invalid),
        _ => return Err(JobFailure::Unsupported),
    };
    let sample = match (bits, format) {
        (8, 1) => Sample::U8,
        (16, 1) => Sample::U16,
        (32, 3) => Sample::F32,
        _ => return Err(JobFailure::Unsupported),
    };
    let compression = scalar(COMPRESSION, NONE)?;
    let compressed = compression == LZW || DEFLATE.contains(&compression);
    unsupported(compressed || compression == NONE || compression == PACKBITS)?;
    match scalar(PREDICTOR, 1)? {
        1 => {}
        2 => unsupported(compressed && sample != Sample::F32)?,
        3 => unsupported(compressed && sample == Sample::F32)?,
        _ => return Err(JobFailure::Unsupported),
    }
    unsupported(scalar(PLANAR_CONFIGURATION, 1)? == 1)?;
    let layout = Layout { channels, sample };
    let chunk_width = chunks(&directory, width, height)?;
    let row = u64::from(chunk_width)
        .checked_mul(layout.pixel_size() as u64)
        .and_then(|row| usize::try_from(row).ok())
        .ok_or(JobFailure::Budget)?;
    let left = budget.checked_sub(row).ok_or(JobFailure::Budget)?;
    check_budget(layout, width, height, left)?;
    Ok(Plan {
        width,
        height,
        layout,
    })
}

/// Check that the offset and byte-count arrays of the strips or tiles hold
/// one entry for each strip or tile the image's size needs, without reading
/// their values. Gives the width of one strip or tile.
fn chunks(directory: &ifd::Directory, width: u32, height: u32) -> Result<u32, JobFailure> {
    let invalid = JobFailure::Invalid;
    let positive = |tag| {
        directory
            .scalar(tag)
            .and_then(|v| u32::try_from(v).ok())
            .filter(|&v| v > 0)
    };
    let (chunk_width, count, offsets, byte_counts) = if directory.has(TILE_OFFSETS) {
        let tile_width = positive(TILE_WIDTH).ok_or(invalid)?;
        let tile_length = positive(TILE_LENGTH).ok_or(invalid)?;
        let across = u64::from(width.div_ceil(tile_width));
        let down = u64::from(height.div_ceil(tile_length));
        let count = across.checked_mul(down).ok_or(invalid)?;
        (tile_width, count, TILE_OFFSETS, TILE_BYTE_COUNTS)
    } else {
        let rows = match directory.count(ROWS_PER_STRIP) {
            None => height,
            Some(_) => positive(ROWS_PER_STRIP).ok_or(invalid)?.min(height),
        };
        let count = u64::from(height.div_ceil(rows));
        (width, count, STRIP_OFFSETS, STRIP_BYTE_COUNTS)
    };
    if directory.count(offsets) != Some(count) || directory.count(byte_counts) != Some(count) {
        return Err(invalid);
    }
    Ok(chunk_width)
}

#[cfg(test)]
thread_local! {
    /// How many times this thread began decoding a TIFF's pixels.
    pub(crate) static DECODES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Decode the first image with the `tiff` crate, whose buffer limits are
/// the whole budget: the helper's heap enforces the real one. The crate
/// inverts white-is-zero samples itself.
fn decode(
    reader: &mut CandidateReader<'_>,
    plan: Plan,
    budget: usize,
) -> Result<Pixels, JobFailure> {
    #[cfg(test)]
    DECODES.with(|count| count.set(count.get() + 1));
    let mut limits = tiff::decoder::Limits::default();
    limits.decoding_buffer_size = budget;
    limits.intermediate_buffer_size = budget;
    let mut decoder = tiff::decoder::Decoder::new(reader)
        .map_err(failure)?
        .with_limits(limits);
    let size = plan
        .layout
        .image_size(plan.width, plan.height)
        .ok_or(JobFailure::Budget)?;
    let mut data = vec![0; size];
    decoder.read_image_bytes(&mut data).map_err(failure)?;
    Ok(Pixels {
        width: plan.width,
        height: plan.height,
        layout: plan.layout,
        data,
    })
}

fn failure(error: tiff::TiffError) -> JobFailure {
    match error {
        tiff::TiffError::LimitsExceeded => JobFailure::Budget,
        tiff::TiffError::UnsupportedError(_) => JobFailure::Unsupported,
        _ => JobFailure::Invalid,
    }
}

/// The sample-property arrays of the first image, each at most
/// [`MAX_SAMPLE_VALUES`] long.
#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Samples {
    pub bits_per_sample: Option<Vec<u64>>,
    pub extra_samples: Option<Vec<u64>>,
    pub sample_format: Option<Vec<u64>>,
}

/// Read the first image's sample-property arrays. `Err` when one is longer
/// than [`MAX_SAMPLE_VALUES`] or cannot be read.
#[cfg(test)]
pub(crate) fn samples(reader: &mut CandidateReader<'_>) -> Result<Samples, ifd::ArrayError> {
    let size = reader
        .seek(SeekFrom::End(0))
        .map_err(|_| ifd::ArrayError::Unreadable)?;
    let tags = [BITS_PER_SAMPLE, EXTRA_SAMPLES, SAMPLE_FORMAT];
    let directory = ifd::first_directory(reader, size, &tags).ok_or(ifd::ArrayError::Unreadable)?;
    let mut read = |tag| directory.array(reader, tag, MAX_SAMPLE_VALUES);
    Ok(Samples {
        bits_per_sample: read(BITS_PER_SAMPLE)?,
        extra_samples: read(EXTRA_SAMPLES)?,
        sample_format: read(SAMPLE_FORMAT)?,
    })
}

/// The TIFF writer the tests share with the pack's integration tests.
#[cfg(test)]
#[path = "../../tests/common/tiff.rs"]
pub(crate) mod builder;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{io::ImageInput, testing};
    use builder::{Image, Tiff as File, SHORT};

    struct Memory(Vec<u8>);

    impl ImageInput for Memory {
        fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
            let start = (offset as usize).min(self.0.len());
            let n = buf.len().min(self.0.len() - start);
            buf[..n].copy_from_slice(&self.0[start..start + n]);
            Ok(n)
        }
    }

    fn with_reader<T>(bytes: Vec<u8>, f: impl FnOnce(&mut CandidateReader<'_>) -> T) -> T {
        let size = bytes.len() as u64;
        let mut input = Memory(bytes);
        f(&mut CandidateReader::new(&mut input, size))
    }

    fn image(samples: usize) -> Image {
        let samples = samples as u64;
        Image::sized(800, 600)
            .with(BITS_PER_SAMPLE, SHORT, &vec![8; samples as usize])
            .with(
                EXTRA_SAMPLES,
                SHORT,
                &vec![0; samples.saturating_sub(3) as usize],
            )
            .with(SAMPLE_FORMAT, SHORT, &vec![1; samples as usize])
    }

    #[test]
    fn sample_arrays_of_64_values_are_read() {
        let bytes = File::classic(vec![image(64)]).bytes();
        let samples = with_reader(bytes, samples).unwrap();
        assert_eq!(samples.bits_per_sample, Some(vec![8; 64]));
        assert_eq!(samples.extra_samples, Some(vec![0; 61]));
        assert_eq!(samples.sample_format, Some(vec![1; 64]));
    }

    #[test]
    fn a_sample_array_of_65_values_is_refused_and_the_dimensions_are_still_read() {
        for tag in [BITS_PER_SAMPLE, EXTRA_SAMPLES, SAMPLE_FORMAT] {
            let bytes = File::classic(vec![image(3).with(tag, SHORT, &[1; 65])]).bytes();
            assert_eq!(
                with_reader(bytes.clone(), samples),
                Err(ifd::ArrayError::TooLong),
                "{tag}"
            );
            assert_eq!(with_reader(bytes, |r| Tiff.dimensions(r)), Some((800, 600)));
        }
    }

    #[test]
    fn a_classic_directory_of_5000_entries_gives_its_width_and_height() {
        let mut image = Image::sized(800, 600);
        for tag in 0..5_000u16 {
            let tag = 40_000 + tag;
            image = image.with(tag, SHORT, &[1, 2, 3]);
        }
        let bytes = File::classic(vec![image]).bytes();
        let (dimensions, held) = testing::measured(|| with_reader(bytes, |r| Tiff.dimensions(r)));
        assert_eq!(dimensions, Some((800, 600)));
        assert!(held < testing::MIB, "the read held {held} bytes");
    }

    /// A PackBits-compressed image whose every sample is the same, in
    /// strips of 16 rows: real pixel data in a small file.
    fn uniform(width: u64, height: u64, bits: u64, samples: u64, photometric: u64) -> Vec<u8> {
        let row = vec![7u8; (width * samples * bits / 8) as usize];
        let packed = builder::compress(&row.repeat(16), builder::PACKBITS_CODE);
        let mut image = Image::chunky(width, height, bits, samples, photometric, Vec::new())
            .with(builder::ROWS_PER_STRIP, SHORT, &[16])
            .coded(builder::PACKBITS_CODE, None);
        image.strips = Some(vec![packed; height.div_ceil(16) as usize]);
        File::classic(vec![image]).bytes()
    }

    /// The preview job's outcome over `bytes` with the helper's heap as the
    /// budget, and how many times it began decoding pixels.
    fn preview(bytes: Vec<u8>) -> (Result<(), JobFailure>, usize) {
        let size = bytes.len() as u64;
        let mut input = testing::Memory(bytes);
        DECODES.with(|count| count.set(0));
        let budget = 512 * testing::MIB - 2 * testing::MIB;
        let outcome =
            crate::preview::render(&mut input, size, "image/tiff", budget, &mut std::io::sink());
        (outcome, DECODES.with(std::cell::Cell::get))
    }

    #[test]
    fn a_tiff_that_passes_the_check_begins_decoding_once() {
        assert_eq!(preview(uniform(64, 64, 8, 3, 2)), (Ok(()), 1));
    }

    #[test]
    fn pixels_over_the_budget_are_refused_before_decoding_begins() {
        for (what, bytes) in [
            (
                "16-bit RGB of 540,000,000 bytes",
                uniform(15_000, 6_000, 16, 3, 2),
            ),
            ("8-bit gray of 512 MiB", uniform(32_768, 16_384, 8, 1, 1)),
        ] {
            assert_eq!(preview(bytes), (Err(JobFailure::Budget), 0), "{what}");
        }
    }

    #[test]
    fn a_type_the_preview_does_not_decode_is_refused_before_the_budget() {
        let cmyk = uniform(16_000, 12_000, 8, 4, 5);
        assert_eq!(preview(cmyk), (Err(JobFailure::Unsupported), 0));
    }

    #[test]
    fn a_side_over_65500_is_refused_before_anything_else() {
        let wide_cmyk = uniform(65_501, 100, 8, 4, 5);
        assert_eq!(preview(wide_cmyk), (Err(JobFailure::TooLarge), 0));
    }

    #[test]
    fn strips_that_do_not_match_the_image_are_damage() {
        let mut image = Image::chunky(64, 64, 8, 1, 1, vec![0; 64 * 64]).with(
            builder::ROWS_PER_STRIP,
            SHORT,
            &[16],
        );
        image.strips = Some(vec![vec![0; 64 * 16]; 3]);
        assert_eq!(
            preview(File::classic(vec![image]).bytes()),
            (Err(JobFailure::Invalid), 0)
        );
    }

    #[test]
    fn a_directory_whose_count_runs_past_the_file_is_unreadable() {
        let mut bytes = File::classic(vec![image(3)]).bytes();
        let directory = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        bytes[directory..directory + 2].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(with_reader(bytes, |r| Tiff.dimensions(r)), None);
    }
}
