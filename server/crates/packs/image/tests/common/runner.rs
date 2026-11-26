//! A pack for tests that run no preview: its runner fails every run.
#![allow(dead_code)]
use sidereal_pack_image::{preview::PreviewRunner, ImagePack};
use sidereal_plugin_abi::{AssetContext, OperatorError};
use std::sync::Arc;

struct NoPreviews;

impl PreviewRunner for NoPreviews {
    fn run(&self, _: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        Err(OperatorError::Failed("this test runs no preview".into()))
    }
}

/// The image pack, with a runner that makes no preview.
pub fn pack() -> ImagePack {
    ImagePack::new(Arc::new(NoPreviews))
}
