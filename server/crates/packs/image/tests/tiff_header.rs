//! The header Operator over TIFF versions, through the public contract only:
//! it reads the first image's width and length, whatever its type.
#[path = "common/tiff.rs"]
mod tiff;

#[path = "common/runner.rs"]
mod runner;

use sidereal_pack_image::READ_HEADER;
use sidereal_plugin_abi::{Candidate, ExecutionError, FacetProposal, FacetValue, Output, Registry};
use tiff::*;

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

struct Discard;

impl Output for Discard {
    fn write(&mut self, _: &[u8]) -> std::io::Result<()> {
        Ok(())
    }
}

/// Run the header Operator over `bytes`, a version of `image/tiff`.
fn read_header(bytes: Vec<u8>) -> Result<Vec<FacetProposal>, ExecutionError> {
    let mut registry = Registry::new();
    registry.register_pack(&runner::pack()).unwrap();
    registry
        .execute_operator(
            READ_HEADER,
            &mut Bytes(bytes),
            "image/tiff",
            &mut Discard,
            &mut |_| {},
            &|| false,
        )
        .map(|run| run.proposals)
}

fn header(width: i64, height: i64) -> Result<Vec<FacetProposal>, ExecutionError> {
    Ok(vec![
        FacetProposal {
            name: "image.width".into(),
            value: FacetValue::Integer(width),
        },
        FacetProposal {
            name: "image.height".into(),
            value: FacetValue::Integer(height),
        },
    ])
}

fn unreadable() -> Result<Vec<FacetProposal>, ExecutionError> {
    Err(ExecutionError::Failed(
        "the TIFF header could not be read".into(),
    ))
}

/// An 8-bit RGB image's tags, with no pixel data.
fn rgb(width: u64, height: u64) -> Image {
    Image::sized(width, height)
        .with(BITS_PER_SAMPLE, SHORT, &[8, 8, 8])
        .with(PHOTOMETRIC, SHORT, &[2])
        .with(SAMPLES_PER_PIXEL, SHORT, &[3])
}

#[test]
fn a_little_endian_classic_tiff_gives_its_first_images_width_and_length() {
    let bytes = Tiff::classic(vec![rgb(7000, 5000)]).bytes();
    assert_eq!(&bytes[..4], b"II*\0");
    assert_eq!(read_header(bytes), header(7000, 5000));
}

#[test]
fn a_big_endian_big_tiff_gives_its_width_and_length() {
    let bytes = Tiff::classic(vec![rgb(640, 480)])
        .big_endian()
        .big_tiff()
        .bytes();
    assert_eq!(&bytes[..4], b"MM\0+");
    assert_eq!(read_header(bytes), header(640, 480));
}

#[test]
fn each_layout_gives_width_and_length_in_short_and_long_fields() {
    for (big_endian, big_tiff) in [(false, false), (true, false), (false, true), (true, true)] {
        let image = rgb(800, 600)
            .with(IMAGE_WIDTH, SHORT, &[800])
            .with(IMAGE_LENGTH, LONG, &[600]);
        let mut tiff = Tiff::classic(vec![image]);
        tiff.big_endian = big_endian;
        tiff.big_tiff = big_tiff;
        assert_eq!(
            read_header(tiff.bytes()),
            header(800, 600),
            "{big_endian} {big_tiff}"
        );
    }
    let bytes = Tiff::classic(vec![rgb(1, 1).with(IMAGE_WIDTH, LONG8, &[70_000]).with(
        IMAGE_LENGTH,
        LONG8,
        &[3],
    )])
    .big_tiff()
    .bytes();
    assert_eq!(read_header(bytes), header(70_000, 3));
}

#[test]
fn a_tiff_with_several_images_gives_the_first_ones() {
    let bytes = Tiff::classic(vec![rgb(800, 600), rgb(200, 150)]).bytes();
    assert_eq!(read_header(bytes), header(800, 600));
}

#[test]
fn a_cmyk_tiff_the_preview_cannot_decode_gives_its_width_and_length() {
    let image = Image::sized(800, 600)
        .with(BITS_PER_SAMPLE, SHORT, &[8, 8, 8, 8])
        .with(PHOTOMETRIC, SHORT, &[5])
        .with(SAMPLES_PER_PIXEL, SHORT, &[4])
        .with(COMPRESSION, SHORT, &[7]);
    assert_eq!(
        read_header(Tiff::classic(vec![image]).bytes()),
        header(800, 600)
    );
}

#[test]
fn a_tiff_with_more_than_64_samples_per_pixel_gives_its_width_and_length() {
    let image = Image::sized(800, 600)
        .with(BITS_PER_SAMPLE, SHORT, &[8; 70])
        .with(SAMPLE_FORMAT, SHORT, &[1; 70])
        .with(EXTRA_SAMPLES, SHORT, &[0; 67])
        .with(SAMPLES_PER_PIXEL, SHORT, &[70]);
    assert_eq!(
        read_header(Tiff::classic(vec![image]).bytes()),
        header(800, 600)
    );
}

#[test]
fn a_tiff_that_ends_after_its_signature_fails() {
    assert_eq!(read_header(b"II*\0".to_vec()), unreadable());
}

/// A classic TIFF's entry has four bytes, so an eight-byte LONG8 width
/// sits elsewhere and the entry holds only its offset. That offset is not
/// the width.
#[test]
fn a_classic_tiff_whose_width_is_a_long8_entry_fails() {
    let image = rgb(800, 600).with(IMAGE_WIDTH, LONG8, &[800]);
    assert_eq!(
        read_header(Tiff::classic(vec![image]).bytes()),
        unreadable()
    );
}

#[test]
fn a_tiff_whose_first_image_has_no_width_fails() {
    let image = rgb(800, 600).without(IMAGE_WIDTH);
    assert_eq!(
        read_header(Tiff::classic(vec![image]).bytes()),
        unreadable()
    );
}

#[test]
fn a_tiff_whose_first_image_has_no_length_or_a_zero_side_fails() {
    for image in [
        rgb(800, 600).without(IMAGE_LENGTH),
        rgb(0, 600),
        rgb(800, 0),
    ] {
        assert_eq!(
            read_header(Tiff::classic(vec![image]).bytes()),
            unreadable()
        );
    }
}

#[test]
fn a_tiff_whose_first_image_directory_lies_past_its_end_fails() {
    let mut tiff = Tiff::classic(vec![rgb(800, 600)]);
    tiff.first_directory = Some(1_000_000);
    assert_eq!(read_header(tiff.bytes()), unreadable());
}

#[test]
fn a_tiff_cut_off_inside_its_first_image_directory_fails() {
    let bytes = Tiff::classic(vec![rgb(800, 600)]).bytes();
    let directory = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    assert_eq!(read_header(bytes[..directory + 10].to_vec()), unreadable());
}
