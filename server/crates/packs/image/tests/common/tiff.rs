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

// ---- compression and prediction, as a TIFF writer applies them ----

/// Compression codes.
pub const UNCOMPRESSED: u64 = 1;
pub const LZW_CODE: u64 = 5;
pub const DEFLATE_CODE: u64 = 8;
pub const PACKBITS_CODE: u64 = 32773;

/// `data` compressed with the TIFF compression `code`.
pub fn compress(data: &[u8], code: u64) -> Vec<u8> {
    match code {
        UNCOMPRESSED => data.to_vec(),
        LZW_CODE => weezl::encode::Encoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8)
            .encode(data)
            .unwrap(),
        DEFLATE_CODE => {
            use std::io::Write;
            let mut encoder =
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
            encoder.write_all(data).unwrap();
            encoder.finish().unwrap()
        }
        PACKBITS_CODE => packbits(data),
        other => panic!("no compressor for {other}"),
    }
}

/// PackBits: runs of 2 to 128 equal bytes, and literals of up to 128.
pub fn packbits(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let run = data[at..]
            .iter()
            .take(128)
            .take_while(|&&b| b == data[at])
            .count();
        if run >= 2 {
            out.push((257 - run) as u8);
            out.push(data[at]);
            at += run;
        } else {
            let start = at;
            while at < data.len()
                && at - start < 128
                && !(at + 1 < data.len() && data[at] == data[at + 1])
            {
                at += 1;
            }
            if at == start {
                at += 1;
            }
            out.push((at - start - 1) as u8);
            out.extend_from_slice(&data[start..at]);
        }
    }
    out
}

/// Apply the horizontal predictor to `rows`, each `row_len` bytes of
/// samples `size` bytes wide in the file's byte order, `samples` per pixel.
pub fn horizontal(rows: &mut [u8], row_len: usize, samples: usize, size: usize, big_endian: bool) {
    let read = |bytes: &[u8]| -> u64 {
        let mut value = 0u64;
        let ordered: Vec<u8> = if big_endian {
            bytes.to_vec()
        } else {
            bytes.iter().rev().copied().collect()
        };
        for b in ordered {
            value = value << 8 | u64::from(b);
        }
        value
    };
    let write = |bytes: &mut [u8], value: u64| {
        for (index, byte) in bytes.iter_mut().enumerate() {
            let shift = if big_endian {
                (size - 1 - index) * 8
            } else {
                index * 8
            };
            *byte = (value >> shift) as u8;
        }
    };
    let mask = if size == 8 {
        u64::MAX
    } else {
        (1u64 << (size * 8)) - 1
    };
    for row in rows.chunks_mut(row_len) {
        let stride = samples * size;
        for at in (stride..row.len()).step_by(size).rev() {
            let previous = read(&row[at - stride..at - stride + size]);
            let current = read(&row[at..at + size]);
            write(
                &mut row[at..at + size],
                current.wrapping_sub(previous) & mask,
            );
        }
    }
}

/// One row of 32-bit floats with the floating-point predictor applied: the
/// bytes of each value, most significant first, split into byte planes,
/// then differenced byte by byte.
pub fn floating_point_row(values: &[f32]) -> Vec<u8> {
    let count = values.len();
    let mut planes = vec![0u8; count * 4];
    for (index, value) in values.iter().enumerate() {
        for (plane, byte) in value.to_be_bytes().into_iter().enumerate() {
            planes[plane * count + index] = byte;
        }
    }
    for at in (1..planes.len()).rev() {
        planes[at] = planes[at].wrapping_sub(planes[at - 1]);
    }
    planes
}

/// Cut `pixels`, rows of `width` pixels of `pixel` bytes each, into tiles
/// of `tile_width` by `tile_length`, padded with zero bytes at the edges.
pub fn tiles_of(
    pixels: &[u8],
    width: usize,
    height: usize,
    pixel: usize,
    tile_width: usize,
    tile_length: usize,
) -> Vec<Vec<u8>> {
    let mut tiles = Vec::new();
    for top in (0..height).step_by(tile_length) {
        for left in (0..width).step_by(tile_width) {
            let mut tile = vec![0u8; tile_width * tile_length * pixel];
            for y in 0..tile_length.min(height - top) {
                let across = tile_width.min(width - left);
                let from = ((top + y) * width + left) * pixel;
                tile[y * tile_width * pixel..][..across * pixel]
                    .copy_from_slice(&pixels[from..from + across * pixel]);
            }
            tiles.push(tile);
        }
    }
    tiles
}

impl Image {
    /// The image with these compression and predictor codes.
    pub fn coded(self, compression: u64, predictor: Option<u64>) -> Self {
        let image = self.with(COMPRESSION, SHORT, &[compression]);
        match predictor {
            Some(predictor) => image.with(PREDICTOR, SHORT, &[predictor]),
            None => image.without(PREDICTOR),
        }
    }
}
