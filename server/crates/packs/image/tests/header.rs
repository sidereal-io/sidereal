//! The header Operator, through the public contract only: it reads an image's
//! format and pixel dimensions from its leading bytes.
use sidereal_pack_image::{ImagePack, FORMAT_FACET, HEIGHT_FACET, READ_HEADER, WIDTH_FACET};
use sidereal_plugin_abi::{FacetSchema, FacetScope, FacetType, Registry};

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry.register_pack(&ImagePack).unwrap();
    registry
}

/// The schema the image pack declares under `name`, which it must own.
fn declared(name: &str) -> FacetSchema {
    let registry = registry();
    let declared = registry.facet_schema(name).unwrap();
    assert_eq!(declared.owner, "image");
    declared.schema.clone()
}

#[test]
fn schema_of_the_format_is_a_choice_of_png_or_jpeg() {
    assert_eq!(FORMAT_FACET, "image.format");
    assert_eq!(
        declared(FORMAT_FACET),
        FacetSchema {
            name: "image.format".into(),
            label: "Format".into(),
            value_type: FacetType::Choice(vec!["PNG".into(), "JPEG".into()]),
            unit: String::new(),
            version: 1,
            scope: FacetScope::AssetVersion,
        }
    );
}

#[test]
fn schema_of_the_width_is_a_count_of_pixels() {
    assert_eq!(WIDTH_FACET, "image.width");
    assert_eq!(
        declared(WIDTH_FACET),
        FacetSchema {
            name: "image.width".into(),
            label: "Width".into(),
            value_type: FacetType::Count,
            unit: "pixels".into(),
            version: 1,
            scope: FacetScope::AssetVersion,
        }
    );
}

#[test]
fn schema_of_the_height_is_a_count_of_pixels() {
    assert_eq!(HEIGHT_FACET, "image.height");
    assert_eq!(
        declared(HEIGHT_FACET),
        FacetSchema {
            name: "image.height".into(),
            label: "Height".into(),
            value_type: FacetType::Count,
            unit: "pixels".into(),
            version: 1,
            scope: FacetScope::AssetVersion,
        }
    );
}

#[test]
fn schema_names_are_the_three_facets_the_header_operator_lists() {
    assert_eq!(READ_HEADER, "image.read-header");
    let registry = registry();
    let entry = registry.operator(READ_HEADER).unwrap();
    assert_eq!(
        entry.writes(),
        ["image.format", "image.width", "image.height"]
    );
    assert_eq!(entry.version(), env!("CARGO_PKG_VERSION"));
}
