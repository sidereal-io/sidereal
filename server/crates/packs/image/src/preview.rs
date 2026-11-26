//! The preview Operator and its job (design decision 8, ADR-020).
//!
//! The Operator, `image.make-preview`, proposes a JPEG preview and hands its
//! Asset context to the host's [`PreviewRunner`], which runs the job in a
//! helper process. The job, [`make_preview`], reads the version by its media
//! type, decodes its first image into one [`Pixels`] buffer, turns each row
//! into 8-bit gray or RGB, and streams a quality-90 baseline JPEG with 4:4:4
//! sampling and standard Huffman tables. A failure is a [`JobFailure`], which
//! the helper reports as its exit status.
use crate::{
    formats,
    io::{FileInput, ImageInput},
    MAKE_PREVIEW,
};
use sidereal_plugin_abi::{AssetContext, AssetProposal, Operator, OperatorError};
use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::Arc,
};

/// The longest side a browser-ready JPEG preview may have.
pub(crate) const MAX_SIDE: u32 = 65_500;

/// The JPEG quality of every preview.
const QUALITY: u8 = 90;

/// Runs one preview job for the version an Operator run reads, outside the
/// host process. The host supplies it when it registers the pack. It reads
/// the candidate through `context`, runs the job's helper over a copy, and
/// writes the JPEG the helper returns through
/// [`AssetContext::write_preview`].
pub trait PreviewRunner: Send + Sync {
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError>;
}

/// Why a preview job failed, as the helper's exit status reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobFailure {
    /// The decode needs more than the memory budget.
    Budget,
    /// The image is of a type the job does not decode.
    Unsupported,
    /// A side is longer than a browser-ready JPEG can hold.
    TooLarge,
    /// The bytes are cut off, damaged or not of the format.
    Invalid,
    /// Reading the input failed.
    Unreadable,
    /// The helper could not start its decoder.
    Startup,
}

impl JobFailure {
    const ALL: [Self; 6] = [
        Self::Budget,
        Self::Unsupported,
        Self::TooLarge,
        Self::Invalid,
        Self::Unreadable,
        Self::Startup,
    ];

    /// The helper's exit status for this failure. None is 0, 101 (a Rust
    /// panic) or one a shell gives a signal.
    pub fn status(self) -> u8 {
        match self {
            Self::Budget => 70,
            Self::Unsupported => 71,
            Self::TooLarge => 72,
            Self::Invalid => 73,
            Self::Unreadable => 74,
            Self::Startup => 75,
        }
    }

    /// The failure an exit status reports, if it reports one.
    pub fn from_status(status: i32) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|failure| i32::from(failure.status()) == status)
    }

    /// The reason core records for a run over a version of `media_type`.
    pub fn reason(self, media_type: &str) -> String {
        let name = crate::formats::by_media_type(media_type).map_or("image", |f| f.name());
        match self {
            Self::Budget => "the image needs more than 512 MiB to decode".into(),
            Self::Unsupported => format!("the {name} image type is not supported"),
            Self::TooLarge => {
                "the image is wider or taller than a browser-ready JPEG can hold".into()
            }
            Self::Invalid => format!("the {name} image could not be decoded"),
            Self::Unreadable => "the stored file could not be read".into(),
            Self::Startup => "the preview decoder stopped".into(),
        }
    }
}

/// The preview Operator. It reads nothing itself: the runner reads the
/// candidate and writes the preview through the Asset context.
pub(crate) struct MakePreview(pub Arc<dyn PreviewRunner>);

impl Operator for MakePreview {
    fn id(&self) -> &str {
        MAKE_PREVIEW
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn writes(&self) -> &[&str] {
        &[]
    }

    fn accepts(&self) -> &[&str] {
        formats::media_types()
    }

    fn makes_previews(&self) -> bool {
        true
    }

    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        context.propose_preview(AssetProposal::new("image", "image/jpeg"))?;
        self.0.run(context)
    }
}

/// How one pixel's samples are laid out in [`Pixels`]: their channels and
/// their type. Every sample is in native byte order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Layout {
    pub channels: Channels,
    pub sample: Sample,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Channels {
    Gray,
    GrayAlpha,
    Rgb,
    Rgba,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sample {
    /// 8-bit unsigned.
    U8,
    /// 16-bit unsigned.
    U16,
    /// 32-bit floating point, with 0 to 1 the range shown. Only a TIFF has
    /// it.
    F32,
}

impl Channels {
    fn count(self) -> usize {
        match self {
            Self::Gray => 1,
            Self::GrayAlpha => 2,
            Self::Rgb => 3,
            Self::Rgba => 4,
        }
    }

    fn gray(self) -> bool {
        matches!(self, Self::Gray | Self::GrayAlpha)
    }

    fn alpha(self) -> bool {
        matches!(self, Self::GrayAlpha | Self::Rgba)
    }
}

impl Sample {
    fn size(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::U16 => 2,
            Self::F32 => 4,
        }
    }
}

impl Layout {
    /// The bytes one pixel takes.
    pub(crate) fn pixel_size(self) -> usize {
        self.channels.count() * self.sample.size()
    }

    /// The bytes an image of `width` by `height` takes, when that fits a
    /// `usize`.
    pub(crate) fn image_size(self, width: u32, height: u32) -> Option<usize> {
        usize::try_from(width)
            .ok()?
            .checked_mul(usize::try_from(height).ok()?)?
            .checked_mul(self.pixel_size())
    }
}

/// One decoded image: every sample of every row, in one buffer.
pub(crate) struct Pixels {
    pub width: u32,
    pub height: u32,
    pub layout: Layout,
    pub data: Vec<u8>,
}

/// Refuse a side longer than a browser-ready JPEG holds. It comes before
/// the budget check.
pub(crate) fn check_sides(width: u32, height: u32) -> Result<(), JobFailure> {
    if width > MAX_SIDE || height > MAX_SIDE {
        Err(JobFailure::TooLarge)
    } else if width == 0 || height == 0 {
        Err(JobFailure::Invalid)
    } else {
        Ok(())
    }
}

/// Refuse an image whose decoded pixels alone do not fit `budget`.
pub(crate) fn check_budget(
    layout: Layout,
    width: u32,
    height: u32,
    budget: usize,
) -> Result<(), JobFailure> {
    match layout.image_size(width, height) {
        Some(size) if size <= budget => Ok(()),
        _ => Err(JobFailure::Budget),
    }
}

/// Run the preview job over `input`, a file that holds a version of
/// `media_type`, and write the JPEG to `output`. `budget` is the most the
/// job's allocations may hold, which the helper's heap enforces.
pub fn make_preview(
    input: &File,
    media_type: &str,
    budget: usize,
    output: &mut dyn Write,
) -> Result<(), JobFailure> {
    let size = input.metadata().map_err(|_| JobFailure::Unreadable)?.len();
    render(&mut FileInput(input), size, media_type, budget, output)
}

/// [`make_preview`] over any input of `size` bytes.
pub(crate) fn render(
    input: &mut dyn ImageInput,
    size: u64,
    media_type: &str,
    budget: usize,
    output: &mut dyn Write,
) -> Result<(), JobFailure> {
    let format = formats::by_media_type(media_type).ok_or(JobFailure::Invalid)?;
    let pixels = format.render_preview(input, size, budget)?;
    encode(&pixels, output)
}

/// Stream `pixels` as a quality-90 baseline JPEG with 4:4:4 sampling.
fn encode(pixels: &Pixels, output: &mut dyn Write) -> Result<(), JobFailure> {
    let width = u16::try_from(pixels.width).map_err(|_| JobFailure::TooLarge)?;
    let height = u16::try_from(pixels.height).map_err(|_| JobFailure::TooLarge)?;
    let mut writer = BufWriter::with_capacity(64 * 1024, output);
    let mut encoder = jpeg_encoder::Encoder::new(&mut writer, QUALITY);
    encoder.set_sampling_factor(jpeg_encoder::SamplingFactor::F_1_1);
    encoder
        .encode_image(Rows {
            pixels,
            width,
            height,
        })
        .map_err(|_| JobFailure::Startup)?;
    writer.flush().map_err(|_| JobFailure::Startup)
}

/// The rows of a [`Pixels`] as the encoder asks for them, in 8 bits.
struct Rows<'a> {
    pixels: &'a Pixels,
    width: u16,
    height: u16,
}

impl jpeg_encoder::ImageBuffer for Rows<'_> {
    fn get_jpeg_color_type(&self) -> jpeg_encoder::JpegColorType {
        if self.pixels.layout.channels.gray() {
            jpeg_encoder::JpegColorType::Luma
        } else {
            jpeg_encoder::JpegColorType::Ycbcr
        }
    }

    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn fill_buffers(&self, y: u16, buffers: &mut [Vec<u8>; 4]) {
        let layout = self.pixels.layout;
        let row_size = layout.pixel_size() * usize::from(self.width);
        let start = usize::from(y) * row_size;
        let row = &self.pixels.data[start..start + row_size];
        for pixel in row.chunks_exact(layout.pixel_size()) {
            let [r, g, b] = to_8_bits(pixel, layout);
            if layout.channels.gray() {
                buffers[0].push(r);
            } else {
                let (y, cb, cr) = jpeg_encoder::rgb_to_ycbcr(r, g, b);
                buffers[0].push(y);
                buffers[1].push(cb);
                buffers[2].push(cr);
            }
        }
    }
}

/// Sample `index` of `pixel`, from 0 to 1.
fn unit(pixel: &[u8], sample: Sample, index: usize) -> f32 {
    let size = sample.size();
    let bytes = &pixel[index * size..(index + 1) * size];
    match sample {
        Sample::U8 => f32::from(bytes[0]) / 255.0,
        Sample::U16 => f32::from(u16::from_ne_bytes([bytes[0], bytes[1]])) / 65_535.0,
        Sample::F32 => {
            let value = f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            if value.is_nan() {
                0.0
            } else {
                value.clamp(0.0, 1.0)
            }
        }
    }
}

/// One pixel as 8-bit RGB (gray in all three), over black when it has
/// alpha. An 8-bit sample stays as it is; a 16-bit one becomes
/// `v × 255 / 65535` and a float `v × 255`, rounded.
fn to_8_bits(pixel: &[u8], layout: Layout) -> [u8; 3] {
    let colors = if layout.channels.gray() { 1 } else { 3 };
    let alpha = if layout.channels.alpha() {
        unit(pixel, layout.sample, colors)
    } else {
        1.0
    };
    let mut out = [0; 3];
    for (index, value) in out.iter_mut().enumerate() {
        let channel = if colors == 1 { 0 } else { index };
        let shown = if layout.sample == Sample::U8 && alpha == 1.0 {
            pixel[channel]
        } else {
            (unit(pixel, layout.sample, channel) * alpha * 255.0).round() as u8
        };
        *value = shown;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_bit_samples_stay_and_wider_ones_scale_with_rounding() {
        let rgb8 = Layout {
            channels: Channels::Rgb,
            sample: Sample::U8,
        };
        assert_eq!(to_8_bits(&[0, 128, 255], rgb8), [0, 128, 255]);
        let gray16 = Layout {
            channels: Channels::Gray,
            sample: Sample::U16,
        };
        for (value, shown) in [(0u16, 0u8), (32_768, 128), (65_535, 255), (1_000, 4)] {
            assert_eq!(
                to_8_bits(&value.to_ne_bytes(), gray16),
                [shown; 3],
                "{value}"
            );
        }
        let gray32 = Layout {
            channels: Channels::Gray,
            sample: Sample::F32,
        };
        for (value, shown) in [(-0.5f32, 0u8), (0.5, 128), (2.0, 255), (f32::NAN, 0)] {
            assert_eq!(
                to_8_bits(&value.to_ne_bytes(), gray32),
                [shown; 3],
                "{value}"
            );
        }
    }

    #[test]
    fn a_pixel_with_alpha_shows_over_black() {
        let rgba8 = Layout {
            channels: Channels::Rgba,
            sample: Sample::U8,
        };
        assert_eq!(to_8_bits(&[255, 255, 255, 0], rgba8), [0, 0, 0]);
        assert_eq!(to_8_bits(&[255, 128, 0, 255], rgba8), [255, 128, 0]);
        assert_eq!(to_8_bits(&[200, 100, 50, 128], rgba8), [100, 50, 25]);
    }

    #[test]
    fn a_side_over_65500_is_refused_before_the_budget() {
        assert_eq!(check_sides(65_500, 100), Ok(()));
        assert_eq!(check_sides(65_501, 100), Err(JobFailure::TooLarge));
        assert_eq!(check_sides(100, 65_501), Err(JobFailure::TooLarge));
        let rgb16 = Layout {
            channels: Channels::Rgb,
            sample: Sample::U16,
        };
        assert_eq!(
            check_budget(rgb16, 15_000, 6_000, 512 * 1024 * 1024),
            Err(JobFailure::Budget)
        );
        assert_eq!(
            check_budget(rgb16, 14_000, 6_000, 512 * 1024 * 1024),
            Ok(())
        );
    }

    #[test]
    fn each_failure_has_its_own_status_and_reads_back_from_it() {
        for failure in JobFailure::ALL {
            assert_eq!(
                JobFailure::from_status(failure.status().into()),
                Some(failure)
            );
            assert!(![0, 101].contains(&failure.status()));
        }
        assert_eq!(JobFailure::from_status(0), None);
        assert_eq!(JobFailure::from_status(101), None);
    }

    #[test]
    fn each_failure_gives_the_specs_reason_for_the_media_type() {
        assert_eq!(
            JobFailure::Budget.reason("image/png"),
            "the image needs more than 512 MiB to decode"
        );
        assert_eq!(
            JobFailure::Unsupported.reason("image/tiff"),
            "the TIFF image type is not supported"
        );
        assert_eq!(
            JobFailure::Unsupported.reason("image/jpeg"),
            "the JPEG image type is not supported"
        );
        assert_eq!(
            JobFailure::TooLarge.reason("image/tiff"),
            "the image is wider or taller than a browser-ready JPEG can hold"
        );
        assert_eq!(
            JobFailure::Invalid.reason("image/png"),
            "the PNG image could not be decoded"
        );
        assert_eq!(
            JobFailure::Unreadable.reason("image/png"),
            "the stored file could not be read"
        );
    }
}
