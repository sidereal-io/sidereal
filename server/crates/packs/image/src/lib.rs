//! First-party image pack.
//!
//! The pack is compiled into the server but codes against
//! [`sidereal_plugin_abi`] only, the same contract a third-party pack uses.
//! `server/scripts/check-arch.sh` enforces that direction.
//!
//! Its manual-upload Source recognizes PNG and JPEG from their leading bytes.
//! It does not decode them: an image with a valid signature but damaged
//! content is still recognized, and the browser reports when it cannot show
//! it.
//!
//! Its file-size Operator records how many bytes an Asset version holds, as
//! the facet `image.file-size`.

use sidereal_plugin_abi::{
    AssetContext, AssetProposal, FacetSchema, FacetScope, FacetType, FacetValue, Operator,
    OperatorError, Pack, Registry, Source, SourceError,
};

/// The id of the manual-upload Source.
pub const MANUAL_UPLOAD: &str = "image.manual-upload";

/// The id of the file-size Operator.
pub const FILE_SIZE: &str = "image.measure-file-size";

/// The facet the file-size Operator writes: an Asset version's size in bytes.
pub const FILE_SIZE_FACET: &str = "image.file-size";

/// The refusal a Source run gives for bytes that are neither PNG nor JPEG.
pub const UNSUPPORTED: &str = "only PNG and JPEG images are supported";

/// The image pack.
pub struct ImagePack;

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
    }
}

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const JPEG_SIGNATURE: &[u8] = b"\xff\xd8\xff";

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
        let mut head = [0; PNG_SIGNATURE.len()];
        let mut read = 0;
        while read < head.len() {
            match context.read_candidate(read as u64, &mut head[read..]) {
                Ok(0) => break,
                Ok(n) => read += n,
                Err(_) => return Err(SourceError::Failed("could not read the candidate".into())),
            }
        }
        let head = &head[..read];
        let media_type = if head.starts_with(PNG_SIGNATURE) {
            "image/png"
        } else if head.starts_with(JPEG_SIGNATURE) {
            "image/jpeg"
        } else {
            return Err(SourceError::Refused(UNSUPPORTED.into()));
        };
        context
            .propose_asset(AssetProposal::new("image", media_type))
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

    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        let size = i64::try_from(context.candidate_size())
            .map_err(|_| OperatorError::Failed("the version is too large to count".into()))?;
        context
            .propose_facet(FILE_SIZE_FACET, FacetValue::Integer(size))
            .map_err(|_| OperatorError::Failed("the proposal was refused".into()))
    }
}
