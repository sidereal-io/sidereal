//! TIFF: classic and BigTIFF, in either byte order. The header read takes
//! the first image's width and length and nothing else, so every TIFF
//! records its dimensions, whatever its type.
use super::ImageFormat;
use crate::io::CandidateReader;
use std::io::{Seek, SeekFrom};

// The preview reads the sample-property arrays; until it does, only the
// tests below call them.
#[allow(dead_code)]
mod ifd;

const IMAGE_WIDTH: u16 = 256;
const IMAGE_LENGTH: u16 = 257;

/// The tags that describe a pixel's samples, which the preview reads.
#[allow(dead_code)]
pub(crate) const BITS_PER_SAMPLE: u16 = 258;
#[allow(dead_code)]
pub(crate) const EXTRA_SAMPLES: u16 = 338;
#[allow(dead_code)]
pub(crate) const SAMPLE_FORMAT: u16 = 339;

/// The most values the preview reads from an array of sample properties.
#[allow(dead_code)]
pub(crate) const MAX_SAMPLE_VALUES: u64 = 64;

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
}

/// The sample-property arrays of the first image, each at most
/// [`MAX_SAMPLE_VALUES`] long, for the preview.
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Samples {
    pub bits_per_sample: Option<Vec<u64>>,
    pub extra_samples: Option<Vec<u64>>,
    pub sample_format: Option<Vec<u64>>,
}

/// Read the first image's sample-property arrays. `Err` when one is longer
/// than [`MAX_SAMPLE_VALUES`] or cannot be read.
#[allow(dead_code)]
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

    #[test]
    fn a_directory_whose_count_runs_past_the_file_is_unreadable() {
        let mut bytes = File::classic(vec![image(3)]).bytes();
        let directory = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        bytes[directory..directory + 2].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(with_reader(bytes, |r| Tiff.dimensions(r)), None);
    }
}
