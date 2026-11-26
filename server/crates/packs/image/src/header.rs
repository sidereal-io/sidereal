//! The header Operator: an image's width and height, read from its header by
//! the format its version's media type names. It decodes no pixel data and
//! ignores any EXIF orientation.
use crate::{
    formats,
    io::{CandidateReader, ContextInput},
    HEIGHT_FACET, READ_HEADER, WIDTH_FACET,
};
use sidereal_plugin_abi::{AssetContext, FacetValue, Operator, OperatorError};

/// The reason a run gives for a media type the pack has no format for. The
/// Registry never runs the Operator over one, so this is only a guard.
const NOT_RECOGNIZED: &str = "the image format is not recognized";

/// The reason a run gives when any read of the stored file fails.
pub(crate) const NOT_READ: &str = "the stored file could not be read";

pub(crate) struct ReadHeader;

impl Operator for ReadHeader {
    fn id(&self) -> &str {
        READ_HEADER
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn writes(&self) -> &[&str] {
        &[WIDTH_FACET, HEIGHT_FACET]
    }

    fn accepts(&self) -> &[&str] {
        formats::media_types()
    }

    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        let format = context
            .media_type()
            .and_then(formats::by_media_type)
            .ok_or_else(|| OperatorError::Failed(NOT_RECOGNIZED.into()))?;
        let size = context.candidate_size();
        let mut input = ContextInput(context);
        let mut reader = CandidateReader::new(&mut input, size);
        let dimensions = format.dimensions(&mut reader);
        if reader.failed() {
            return Err(OperatorError::Failed(NOT_READ.into()));
        }
        let (width, height) = dimensions
            .filter(|&(width, height)| width > 0 && height > 0)
            .ok_or_else(|| {
                OperatorError::Failed(format!("the {} header could not be read", format.name()))
            })?;
        for (name, value) in [
            (WIDTH_FACET, FacetValue::Integer(width.into())),
            (HEIGHT_FACET, FacetValue::Integer(height.into())),
        ] {
            context
                .propose_facet(name, value)
                .map_err(|_| OperatorError::Failed("the proposal was refused".into()))?;
        }
        Ok(())
    }
}
