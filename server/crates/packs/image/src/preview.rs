//! The preview job's contract with the host: how a host runs the job outside
//! its own process ([`PreviewRunner`]), and how the job's helper reports a
//! failure ([`JobFailure`], as its exit status).
use sidereal_plugin_abi::{AssetContext, OperatorError};

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

#[cfg(test)]
mod tests {
    use super::*;

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
