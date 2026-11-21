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

use sidereal_plugin_abi::{AssetContext, AssetProposal, Pack, Registry, Source, SourceError};

/// The id of the manual-upload Source.
pub const MANUAL_UPLOAD: &str = "image.manual-upload";

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
