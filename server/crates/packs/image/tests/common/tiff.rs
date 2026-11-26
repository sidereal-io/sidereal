//! A small TIFF writer for tests: classic or BigTIFF, either byte order,
//! any tags, and strips or tiles of given bytes. Tests build big-endian,
//! BigTIFF, CMYK, compressed, tiled and cut-off files with it, so no binary
//! file is committed. The image pack's unit tests, its integration tests and
//! the server's tests include this one file with `#[path]`.
#![allow(dead_code)]

/// TIFF field types.
pub const BYTE: u16 = 1;
pub const ASCII: u16 = 2;
pub const SHORT: u16 = 3;
pub const LONG: u16 = 4;
pub const FLOAT: u16 = 11;
pub const DOUBLE: u16 = 12;
pub const LONG8: u16 = 16;

/// Tags the tests use.
pub const IMAGE_WIDTH: u16 = 256;
pub const IMAGE_LENGTH: u16 = 257;
pub const BITS_PER_SAMPLE: u16 = 258;
pub const COMPRESSION: u16 = 259;
pub const PHOTOMETRIC: u16 = 262;
pub const STRIP_OFFSETS: u16 = 273;
pub const SAMPLES_PER_PIXEL: u16 = 277;
pub const ROWS_PER_STRIP: u16 = 278;
pub const STRIP_BYTE_COUNTS: u16 = 279;
pub const PLANAR_CONFIGURATION: u16 = 284;
pub const PREDICTOR: u16 = 317;
pub const TILE_WIDTH: u16 = 322;
pub const TILE_LENGTH: u16 = 323;
pub const TILE_OFFSETS: u16 = 324;
pub const TILE_BYTE_COUNTS: u16 = 325;
pub const EXTRA_SAMPLES: u16 = 338;
pub const SAMPLE_FORMAT: u16 = 339;

/// One directory entry: a tag, its field type and its values. A value is
/// the raw bits: a FLOAT is `f32::to_bits`, a DOUBLE `f64::to_bits`.
#[derive(Clone, Debug)]
pub struct Tag {
    pub tag: u16,
    pub kind: u16,
    pub values: Vec<u64>,
}

impl Tag {
    pub fn new(tag: u16, kind: u16, values: &[u64]) -> Self {
        Self {
            tag,
            kind,
            values: values.to_vec(),
        }
    }

    pub fn short(tag: u16, values: &[u64]) -> Self {
        Self::new(tag, SHORT, values)
    }

    pub fn long(tag: u16, values: &[u64]) -> Self {
        Self::new(tag, LONG, values)
    }
}

/// One image: its tags, and the bytes of its strips or tiles. The writer
/// fills in the offsets and byte counts of the strips or tiles.
#[derive(Clone, Debug, Default)]
pub struct Image {
    pub tags: Vec<Tag>,
    pub strips: Option<Vec<Vec<u8>>>,
    pub tiles: Option<Vec<Vec<u8>>>,
}

impl Image {
    /// An image of `width` by `height` with only those two tags.
    pub fn sized(width: u64, height: u64) -> Self {
        Self {
            tags: vec![
                Tag::long(IMAGE_WIDTH, &[width]),
                Tag::long(IMAGE_LENGTH, &[height]),
            ],
            ..Self::default()
        }
    }

    /// An uncompressed, chunky image of `width` by `height`, with
    /// `samples` samples of `bits` bits each per pixel, `photometric`
    /// interpretation, and one strip holding `pixels`.
    pub fn chunky(
        width: u64,
        height: u64,
        bits: u64,
        samples: u64,
        photometric: u64,
        pixels: Vec<u8>,
    ) -> Self {
        let mut image = Self::sized(width, height);
        image.tags.extend([
            Tag::short(BITS_PER_SAMPLE, &vec![bits; samples as usize]),
            Tag::short(COMPRESSION, &[1]),
            Tag::short(PHOTOMETRIC, &[photometric]),
            Tag::short(SAMPLES_PER_PIXEL, &[samples]),
            Tag::long(ROWS_PER_STRIP, &[height]),
            Tag::short(PLANAR_CONFIGURATION, &[1]),
        ]);
        image.strips = Some(vec![pixels]);
        image
    }

    /// An 8-bit RGB image whose pixel at column x, row y is `pixel(x, y)`.
    pub fn rgb8(width: u64, height: u64, pixel: impl Fn(u64, u64) -> [u8; 3]) -> Self {
        let mut pixels = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(&pixel(x, y));
            }
        }
        Self::chunky(width, height, 8, 3, 2, pixels)
    }

    /// The image with `tag` set to `values` of `kind`, replacing any it had.
    pub fn with(mut self, tag: u16, kind: u16, values: &[u64]) -> Self {
        self.tags.retain(|t| t.tag != tag);
        self.tags.push(Tag::new(tag, kind, values));
        self
    }

    /// The image without `tag`.
    pub fn without(mut self, tag: u16) -> Self {
        self.tags.retain(|t| t.tag != tag);
        self
    }
}

/// A whole TIFF file.
#[derive(Clone, Debug)]
pub struct Tiff {
    pub big_endian: bool,
    pub big_tiff: bool,
    pub images: Vec<Image>,
    /// Write this as the first directory's offset in place of the real one.
    pub first_directory: Option<u64>,
}

impl Tiff {
    /// A little-endian classic TIFF of `images`.
    pub fn classic(images: Vec<Image>) -> Self {
        Self {
            big_endian: false,
            big_tiff: false,
            images,
            first_directory: None,
        }
    }

    pub fn big_endian(mut self) -> Self {
        self.big_endian = true;
        self
    }

    pub fn big_tiff(mut self) -> Self {
        self.big_tiff = true;
        self
    }

    fn put(&self, out: &mut Vec<u8>, value: u64, size: usize) {
        let bytes = value.to_be_bytes();
        let mut field = bytes[8 - size..].to_vec();
        if !self.big_endian {
            field.reverse();
        }
        out.extend_from_slice(&field);
    }

    fn patch(&self, out: &mut [u8], at: usize, value: u64, size: usize) {
        let mut field = Vec::new();
        self.put(&mut field, value, size);
        out[at..at + size].copy_from_slice(&field);
    }

    /// The file's bytes.
    pub fn bytes(&self) -> Vec<u8> {
        let (offset_size, count_size) = if self.big_tiff { (8, 8) } else { (4, 2) };
        let mut out = Vec::new();
        out.extend_from_slice(if self.big_endian { b"MM" } else { b"II" });
        if self.big_tiff {
            self.put(&mut out, 43, 2);
            self.put(&mut out, 8, 2);
            self.put(&mut out, 0, 2);
        } else {
            self.put(&mut out, 42, 2);
        }
        let mut next_at = out.len();
        self.put(&mut out, 0, offset_size);
        for image in &self.images {
            let mut tags = image.tags.clone();
            for (blocks, offsets, counts) in [
                (&image.strips, STRIP_OFFSETS, STRIP_BYTE_COUNTS),
                (&image.tiles, TILE_OFFSETS, TILE_BYTE_COUNTS),
            ] {
                let Some(blocks) = blocks else { continue };
                let mut places = Vec::new();
                for block in blocks {
                    places.push(out.len() as u64);
                    out.extend_from_slice(block);
                }
                let kind = if self.big_tiff { LONG8 } else { LONG };
                tags.retain(|t| t.tag != offsets && t.tag != counts);
                tags.push(Tag::new(offsets, kind, &places));
                let sizes: Vec<u64> = blocks.iter().map(|b| b.len() as u64).collect();
                tags.push(Tag::new(counts, kind, &sizes));
            }
            tags.sort_by_key(|t| t.tag);
            if out.len() % 2 == 1 {
                out.push(0);
            }
            let directory = out.len() as u64;
            self.patch(&mut out, next_at, directory, offset_size);
            self.put(&mut out, tags.len() as u64, count_size);
            let mut outside = Vec::new();
            for tag in &tags {
                self.put(&mut out, tag.tag.into(), 2);
                self.put(&mut out, tag.kind.into(), 2);
                self.put(&mut out, tag.values.len() as u64, offset_size);
                let size = match tag.kind {
                    BYTE | ASCII | 6 | 7 => 1,
                    SHORT | 8 => 2,
                    LONG | 9 | FLOAT | 13 => 4,
                    _ => 8,
                };
                let mut value = Vec::new();
                for &v in &tag.values {
                    self.put(&mut value, v, size);
                }
                if value.len() <= offset_size {
                    value.resize(offset_size, 0);
                    out.extend_from_slice(&value);
                } else {
                    outside.push((out.len(), value));
                    self.put(&mut out, 0, offset_size);
                }
            }
            next_at = out.len();
            self.put(&mut out, 0, offset_size);
            for (at, value) in outside {
                if out.len() % 2 == 1 {
                    out.push(0);
                }
                let place = out.len() as u64;
                self.patch(&mut out, at, place, offset_size);
                out.extend_from_slice(&value);
            }
        }
        if let Some(first) = self.first_directory {
            let at = if self.big_tiff { 8 } else { 4 };
            self.patch(&mut out, at, first, offset_size);
        }
        out
    }
}
