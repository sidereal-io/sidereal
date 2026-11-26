//! The manual-upload Source recognizes PNG and JPEG from their bytes alone,
//! through the public contract. It never sees a filename or a client type.
use sidereal_pack_image::{ImagePack, MANUAL_UPLOAD};
use sidereal_plugin_abi::{AssetProposal, Candidate, ExecutionError, Pack, Provenance, Registry};

struct Bytes(Vec<u8>);
impl Candidate for Bytes {
    fn size(&self) -> u64 {
        self.0.len() as u64
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        let start = (offset as usize).min(self.0.len());
        let n = buf.len().min(self.0.len() - start);
        buf[..n].copy_from_slice(&self.0[start..start + n]);
        Ok(n)
    }
}

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

fn recognize(bytes: &[u8]) -> Result<AssetProposal, ExecutionError> {
    let mut registry = Registry::new();
    registry.register_pack(&ImagePack).unwrap();
    registry
        .execute_source(
            MANUAL_UPLOAD,
            &mut Bytes(bytes.to_vec()),
            &mut |_| {},
            &|| false,
        )
        .map(|accepted| accepted.proposal)
}

fn png() -> Vec<u8> {
    // A complete 1x1 PNG.
    let mut bytes = PNG_SIGNATURE.to_vec();
    bytes.extend_from_slice(b"\x00\x00\x00\x0dIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\x0dIDATx\x9cc\xf8\x0f\x00\x00\x01\x01\x00\x05\x18\xd8N\x00\x00\x00\x00IEND\xaeB`\x82");
    bytes
}

#[test]
fn registration_gives_the_registry_the_manual_upload_source() {
    let mut registry = Registry::new();
    registry.register_pack(&ImagePack).unwrap();
    assert_eq!(ImagePack.id(), "image");
    assert_eq!(registry.source_ids(), ["image.manual-upload"]);
    assert_eq!(
        registry.operator_ids(),
        ["image.measure-file-size", "image.read-header"]
    );
}

#[test]
fn registration_gives_every_operator_the_media_types_the_source_recognizes() {
    let mut registry = Registry::new();
    registry.register_pack(&ImagePack).unwrap();
    for id in registry.operator_ids() {
        assert_eq!(
            registry.operator(id).unwrap().accepts(),
            ["image/png", "image/jpeg"],
            "{id}"
        );
    }
}

#[test]
fn png_bytes_are_an_image_png() {
    assert_eq!(
        recognize(&png()),
        Ok(AssetProposal::new("image", "image/png"))
    );
}

#[test]
fn jpeg_bytes_are_an_image_jpeg() {
    assert_eq!(
        recognize(b"\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00\xff\xd9"),
        Ok(AssetProposal::new("image", "image/jpeg"))
    );
}

#[test]
fn provenance_names_the_manual_upload_source_and_its_version() {
    let mut registry = Registry::new();
    registry.register_pack(&ImagePack).unwrap();
    let accepted = registry
        .execute_source(MANUAL_UPLOAD, &mut Bytes(png()), &mut |_| {}, &|| false)
        .unwrap();
    assert_eq!(
        accepted.provenance,
        Provenance {
            source_id: "image.manual-upload".into(),
            source_version: env!("CARGO_PKG_VERSION").into()
        }
    );
}

/// A file named `nebula.jpg` that holds plain text: the name is not an
/// input, so only the bytes decide.
#[test]
fn text_that_a_filename_calls_a_jpeg_is_refused() {
    assert!(matches!(
        recognize(b"this is not a jpeg"),
        Err(ExecutionError::Refused(_))
    ));
}

#[test]
fn unsupported_bytes_are_refused_with_the_supported_formats_named() {
    for (what, bytes) in [
        ("empty", &b""[..]),
        ("TIFF little-endian", b"II*\x00\x08\x00\x00\x00"),
        ("TIFF big-endian", b"MM\x00*\x00\x00\x00\x08"),
        ("SVG", b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
        ("a truncated PNG signature", &PNG_SIGNATURE[..7]),
        ("two JPEG bytes", b"\xff\xd8"),
    ] {
        match recognize(bytes) {
            Err(ExecutionError::Refused(reason)) => {
                assert!(
                    reason.contains("PNG") && reason.contains("JPEG"),
                    "{what}: {reason}"
                )
            }
            other => panic!("{what}: {other:?}"),
        }
    }
}

#[test]
fn a_damaged_png_with_its_signature_is_still_recognized() {
    let mut bytes = PNG_SIGNATURE.to_vec();
    bytes.extend_from_slice(b"garbage that no decoder accepts");
    assert_eq!(
        recognize(&bytes),
        Ok(AssetProposal::new("image", "image/png"))
    );
}

#[test]
fn a_damaged_jpeg_with_its_signature_is_still_recognized() {
    assert_eq!(
        recognize(b"\xff\xd8\xff\x00 truncated"),
        Ok(AssetProposal::new("image", "image/jpeg"))
    );
}
