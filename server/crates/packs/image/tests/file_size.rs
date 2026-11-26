//! The file-size Operator, through the public contract only.
#[path = "common/runner.rs"]
mod runner;

use sidereal_pack_image::{FILE_SIZE, FILE_SIZE_FACET};
use sidereal_plugin_abi::{
    Candidate, FacetProposal, FacetSchema, FacetScope, FacetType, FacetValue, Output, Registry,
};

/// A candidate of `size` bytes that refuses every read: the size is the fact.
struct Unread(u64);
impl Candidate for Unread {
    fn size(&self) -> u64 {
        self.0
    }
    fn read_at(&mut self, _: u64, _: &mut [u8]) -> std::io::Result<usize> {
        panic!("the file-size operator read the bytes")
    }
}

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry.register_pack(&runner::pack()).unwrap();
    registry
}

#[test]
fn file_size_schema_is_declared_by_the_image_pack() {
    assert_eq!(FILE_SIZE_FACET, "image.file-size");
    let registry = registry();
    let declared = registry.facet_schema(FILE_SIZE_FACET).unwrap();
    assert_eq!(declared.owner, "image");
    assert_eq!(
        declared.schema,
        FacetSchema {
            name: "image.file-size".into(),
            label: "File size".into(),
            value_type: FacetType::Count,
            unit: "bytes".into(),
            version: 1,
            scope: FacetScope::AssetVersion,
        }
    );
}

#[test]
fn file_size_operator_lists_only_the_file_size_facet() {
    assert_eq!(FILE_SIZE, "image.measure-file-size");
    let registry = registry();
    let entry = registry.operator(FILE_SIZE).unwrap();
    assert_eq!(entry.writes(), ["image.file-size"]);
    assert_eq!(entry.version(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn file_size_of_a_png_of_known_size_is_its_byte_count() {
    let run = registry()
        .execute_operator(
            FILE_SIZE,
            &mut Unread(1_234_567),
            "image/png",
            &mut Discard,
            &mut |_| {},
            &|| false,
        )
        .unwrap();
    assert_eq!(
        run.proposals,
        [FacetProposal {
            name: "image.file-size".into(),
            value: FacetValue::Integer(1_234_567)
        }]
    );
    assert_eq!(run.producer_id, "image.measure-file-size");
    assert_eq!(run.producer_version, env!("CARGO_PKG_VERSION"));
}

/// A host destination that drops every preview byte.
struct Discard;

impl Output for Discard {
    fn write(&mut self, _: &[u8]) -> std::io::Result<()> {
        Ok(())
    }
}
