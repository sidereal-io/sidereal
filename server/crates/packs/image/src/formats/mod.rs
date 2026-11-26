//! The image formats the pack reads, one module each, and the one table that
//! drives recognition, the Operators' accepted media types and the refusal
//! text. Adding a format adds one module and one entry in [`FORMATS`].
use crate::{
    io::{CandidateReader, ImageInput},
    preview::{JobFailure, Pixels},
};
use std::sync::LazyLock;

mod jpeg;
mod png;
mod tiff;

/// One image format. Internal to the pack: it is not part of the plugin
/// contract.
pub(crate) trait ImageFormat: Sync {
    /// The name in failure reasons and the refusal, such as `PNG`.
    fn name(&self) -> &'static str;
    /// The media type the Source records, such as `image/png`.
    fn media_type(&self) -> &'static str;
    /// The signatures a file of this format starts with.
    fn signatures(&self) -> &'static [&'static [u8]];
    /// How many leading bytes `matches` needs: the longest signature.
    fn probe_len(&self) -> usize {
        self.signatures().iter().map(|s| s.len()).max().unwrap_or(0)
    }
    /// Whether `head`, the candidate's first bytes, starts with one of this
    /// format's signatures. `head` is shorter than `probe_len` only when
    /// the candidate is.
    fn matches(&self, head: &[u8]) -> bool {
        self.signatures().iter().any(|s| head.starts_with(s))
    }
    /// The width and height the header stores, read under this format's
    /// own read limits, or `None` when they cannot be read.
    fn dimensions(&self, reader: &mut CandidateReader<'_>) -> Option<(u32, u32)>;
    /// Render the first image for its preview from `input`, which holds
    /// `size` bytes: read the header, check it against the shared limits
    /// and `budget`, then decode its pixels.
    fn render_preview(
        &self,
        input: &mut dyn ImageInput,
        size: u64,
        budget: usize,
    ) -> Result<Pixels, JobFailure>;
}

/// Every format the pack reads, in the order the Source tries them.
pub(crate) static FORMATS: &[&dyn ImageFormat] = &[&png::Png, &jpeg::Jpeg, &tiff::Tiff];

/// How many leading bytes the Source reads: the most any format needs.
pub(crate) fn probe_len() -> usize {
    FORMATS.iter().map(|f| f.probe_len()).max().unwrap_or(0)
}

/// The first format whose signature `head` starts with.
pub(crate) fn recognize(head: &[u8]) -> Option<&'static dyn ImageFormat> {
    FORMATS.iter().copied().find(|f| f.matches(head))
}

/// The format whose media type is `media_type`.
pub(crate) fn by_media_type(media_type: &str) -> Option<&'static dyn ImageFormat> {
    FORMATS
        .iter()
        .copied()
        .find(|f| f.media_type() == media_type)
}

static MEDIA_TYPES: LazyLock<Vec<&'static str>> =
    LazyLock::new(|| FORMATS.iter().map(|f| f.media_type()).collect());

/// Every format's media type, in table order: each Operator's `accepts`.
pub(crate) fn media_types() -> &'static [&'static str] {
    &MEDIA_TYPES
}

/// The Source's refusal: `only PNG and JPEG images are supported`, with
/// every format's name.
pub(crate) fn refusal() -> String {
    let names: Vec<_> = FORMATS.iter().map(|f| f.name()).collect();
    let list = match names.split_last() {
        Some((last, [])) => (*last).to_owned(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    };
    format!("only {list} images are supported")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_two_formats_share_a_media_type_or_match_each_others_signature() {
        for (i, a) in FORMATS.iter().enumerate() {
            for (j, b) in FORMATS.iter().enumerate() {
                if i == j {
                    continue;
                }
                assert_ne!(a.media_type(), b.media_type());
                for signature in a.signatures() {
                    assert!(!b.matches(signature), "{} matches {}", b.name(), a.name());
                }
            }
        }
    }

    #[test]
    fn each_format_recognizes_its_own_signatures_and_media_type() {
        for format in FORMATS {
            for signature in format.signatures() {
                assert_eq!(recognize(signature).unwrap().name(), format.name());
                assert!(signature.len() <= probe_len());
            }
            assert_eq!(
                by_media_type(format.media_type()).unwrap().name(),
                format.name()
            );
        }
        assert!(by_media_type("image/gif").is_none());
    }

    #[test]
    fn the_refusal_names_every_format() {
        assert_eq!(refusal(), "only PNG, JPEG and TIFF images are supported");
        assert_eq!(media_types(), ["image/png", "image/jpeg", "image/tiff"]);
        assert_eq!(probe_len(), 8);
    }
}
