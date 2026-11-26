//! First-party image pack.
//!
//! The pack is compiled into the server but codes against
//! [`sidereal_plugin_abi`] only, the same contract a third-party pack uses.
//! `server/scripts/check-arch.sh` enforces that direction.
//!
//! Its manual-upload Source recognizes each format in its `formats` table
//! from the leading bytes. It does not decode them: an image with a valid
//! signature but damaged content is still recognized, and the browser
//! reports when it cannot show it. Every Operator of the pack accepts the
//! media types of that same table.
//!
//! Its file-size Operator records how many bytes an Asset version holds, as
//! the facet `image.file-size`. Its header Operator records an image's pixel
//! dimensions, as `image.width` and `image.height`. The format is not a
//! facet: the Source sets each version's media type from its bytes. Its
//! preview Operator makes a browser-ready JPEG of each image, as a derived
//! Asset, through a runner the host supplies.

mod formats;
mod header;
mod io;
pub mod preview;
#[cfg(test)]
mod testing;

use header::ReadHeader;
use preview::PreviewRunner;
use sidereal_plugin_abi::{
    AssetContext, AssetProposal, FacetSchema, FacetScope, FacetType, FacetValue, Operator,
    OperatorError, Pack, Registry, Source, SourceError,
};
use std::sync::Arc;

/// The id of the manual-upload Source.
pub const MANUAL_UPLOAD: &str = "image.manual-upload";

/// The id of the file-size Operator.
pub const FILE_SIZE: &str = "image.measure-file-size";

/// The facet the file-size Operator writes: an Asset version's size in bytes.
pub const FILE_SIZE_FACET: &str = "image.file-size";

/// The id of the header Operator.
pub const READ_HEADER: &str = "image.read-header";

/// The facet the header Operator writes for an image's width in pixels.
pub const WIDTH_FACET: &str = "image.width";

/// The facet the header Operator writes for an image's height in pixels.
pub const HEIGHT_FACET: &str = "image.height";

/// The id of the preview Operator.
pub const MAKE_PREVIEW: &str = "image.make-preview";

/// The refusal a Source run gives for bytes of no format the pack reads,
/// such as `only PNG and JPEG images are supported`.
pub fn unsupported() -> String {
    formats::refusal()
}

/// The image pack. Its preview Operator makes each preview through
/// `runner`, which the host supplies.
pub struct ImagePack {
    runner: Arc<dyn PreviewRunner>,
}

impl ImagePack {
    pub fn new(runner: Arc<dyn PreviewRunner>) -> Self {
        Self { runner }
    }
}

impl Pack for ImagePack {
    fn id(&self) -> &str {
        "image"
    }

    fn register(&self, registry: &mut Registry) {
        registry.register_source(Box::new(ManualUpload));
        registry.declare_facet(FacetSchema {
            name: FILE_SIZE_FACET.into(),
            label: "File size".into(),
            value_type: FacetType::Count,
            unit: "bytes".into(),
            version: 1,
            scope: FacetScope::AssetVersion,
        });
        registry.register_operator(Box::new(MeasureFileSize));
        for (name, label) in [(WIDTH_FACET, "Width"), (HEIGHT_FACET, "Height")] {
            registry.declare_facet(FacetSchema {
                name: name.into(),
                label: label.into(),
                value_type: FacetType::Count,
                unit: "pixels".into(),
                version: 1,
                scope: FacetScope::AssetVersion,
            });
        }
        registry.register_operator(Box::new(ReadHeader));
        registry.register_operator(Box::new(preview::MakePreview(self.runner.clone())));
    }
}

/// Brings one file a person chose into the library.
struct ManualUpload;

impl Source for ManualUpload {
    fn id(&self) -> &str {
        MANUAL_UPLOAD
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), SourceError> {
        let head = io::head(context, formats::probe_len())
            .ok_or_else(|| SourceError::Failed("could not read the candidate".into()))?;
        let format =
            formats::recognize(&head).ok_or_else(|| SourceError::Refused(unsupported()))?;
        context
            .propose_asset(AssetProposal::new("image", format.media_type()))
            .map_err(|_| SourceError::Failed("the proposal was refused".into()))
    }
}

/// Records an Asset version's size in bytes. It reads no bytes: the size is
/// the fact, and the host has checked that the stored object has that size.
struct MeasureFileSize;

impl Operator for MeasureFileSize {
    fn id(&self) -> &str {
        FILE_SIZE
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn writes(&self) -> &[&str] {
        &[FILE_SIZE_FACET]
    }

    fn accepts(&self) -> &[&str] {
        formats::media_types()
    }

    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        let size = i64::try_from(context.candidate_size())
            .map_err(|_| OperatorError::Failed("the version is too large to count".into()))?;
        context
            .propose_facet(FILE_SIZE_FACET, FacetValue::Integer(size))
            .map_err(|_| OperatorError::Failed("the proposal was refused".into()))
    }
}
