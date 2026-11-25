//! The header Operator: an image's width and height, read from its header by
//! the decoders the `image` crate uses, called directly. It decodes no pixel
//! data and ignores any EXIF orientation.
use crate::{Format, HEIGHT_FACET, READ_HEADER, WIDTH_FACET};
use sidereal_plugin_abi::{AssetContext, FacetValue, Operator, OperatorError};
use std::io::{self, BufRead, Read, Seek, SeekFrom};

/// The reason a run gives for bytes that start with neither signature.
const NOT_RECOGNIZED: &str = "the image format is not recognized";

/// The largest width or height a PNG header may hold.
const PNG_MAX_SIDE: u32 = i32::MAX as u32;

/// The largest width or height a JPEG frame header can hold. zune-jpeg's own
/// limit is 16,384, which a large mosaic passes.
const JPEG_MAX_SIDE: usize = u16::MAX as usize;

/// How far into a JPEG the reader goes: its header, through the end of its
/// first start-of-scan segment, must fit in the first 8 MiB. This bounds a
/// run's reads and what zune-jpeg keeps from the segments it passes.
const JPEG_READ_LIMIT: u64 = 8 * 1024 * 1024;

pub(crate) struct ReadHeader;

impl Operator for ReadHeader {
    fn id(&self) -> &str {
        READ_HEADER
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn writes(&self) -> &[&str] {
        &[WIDTH_FACET, HEIGHT_FACET]
    }

    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError> {
        let mut reader = CandidateReader::new(context);
        let format = reader
            .signature()
            .and_then(|head| Format::of(&head))
            .ok_or_else(|| OperatorError::Failed(NOT_RECOGNIZED.into()))?;
        let unreadable =
            || OperatorError::Failed(format!("the {} header could not be read", format.name()));
        let (width, height) = match format {
            Format::Png => png_dimensions(&mut reader),
            Format::Jpeg => {
                reader.limit(JPEG_READ_LIMIT);
                jpeg_dimensions(&mut reader)
            }
        }
        .filter(|&(width, height)| width > 0 && height > 0)
        .ok_or_else(unreadable)?;
        for (name, value) in [
            (WIDTH_FACET, FacetValue::Integer(width.into())),
            (HEIGHT_FACET, FacetValue::Integer(height.into())),
        ] {
            context
                .propose_facet(name, value)
                .map_err(|_| OperatorError::Failed("the proposal was refused".into()))?;
        }
        Ok(())
    }
}

/// The width and height a PNG's `IHDR` chunk gives. The decoder reads the
/// signature and `IHDR`, checks its CRC-32 and stops: no chunk after it is
/// read.
fn png_dimensions(reader: &mut CandidateReader<'_, '_>) -> Option<(u32, u32)> {
    let mut decoder = png::Decoder::new(reader);
    let info = decoder.read_header_info().ok()?;
    let (width, height) = (info.width, info.height);
    (width <= PNG_MAX_SIDE && height <= PNG_MAX_SIDE).then_some((width, height))
}

/// The width and height a JPEG's frame header gives. zune-jpeg reads the
/// markers through the first start-of-scan segment, skips the body of each
/// segment it does not keep, and reads 8-bit baseline, extended and
/// progressive frame headers only.
fn jpeg_dimensions(reader: &mut CandidateReader<'_, '_>) -> Option<(u32, u32)> {
    let options = zune_jpeg::zune_core::options::DecoderOptions::default()
        .set_max_width(JPEG_MAX_SIDE)
        .set_max_height(JPEG_MAX_SIDE);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(reader, options);
    decoder.decode_headers().ok()?;
    let (width, height) = decoder.dimensions()?;
    Some((u32::try_from(width).ok()?, u32::try_from(height).ok()?))
}

/// One window of candidate bytes, as one `read_candidate` call filled it.
/// Its buffer only grows, so a short read does not make the next fill zero
/// the buffer again.
#[derive(Default)]
struct Window {
    start: u64,
    bytes: Vec<u8>,
    /// How many of `bytes`, from the start, the last fill gave.
    len: usize,
}

impl Window {
    /// The bytes from `position` to the window's end, when it holds
    /// `position`.
    fn from(&self, position: u64) -> Option<&[u8]> {
        let offset = position.checked_sub(self.start)?;
        let offset = usize::try_from(offset).ok()?;
        (offset < self.len).then(|| &self.bytes[offset..self.len])
    }
}

/// Reads the candidate through an [`AssetContext`] as `BufRead + Seek`,
/// which both decoders need. It keeps the last two windows, each filled by
/// one `read_candidate` call at the current position, so a seek back into
/// the window before needs no new read. A seek only moves the position.
struct CandidateReader<'c, 'a> {
    context: &'c mut AssetContext<'a>,
    position: u64,
    /// Where the candidate ends for this reader: no byte at or past it is
    /// read.
    end: u64,
    windows: [Window; 2],
    /// The window filled last.
    newest: usize,
    /// A `read_candidate` call failed. Every later read fails too, so a
    /// decoder never goes on past bytes the candidate did not give.
    failed: bool,
}

impl<'c, 'a> CandidateReader<'c, 'a> {
    fn new(context: &'c mut AssetContext<'a>) -> Self {
        let end = context.candidate_size();
        Self {
            context,
            position: 0,
            end,
            windows: Default::default(),
            newest: 0,
            failed: false,
        }
    }

    /// The candidate's first bytes, as many as the longest signature, or
    /// fewer when the candidate is shorter. Afterwards the reader is back at
    /// the start. `None` when a read fails.
    fn signature(&mut self) -> Option<Vec<u8>> {
        let mut head = Vec::new();
        self.take(crate::PNG_SIGNATURE.len() as u64)
            .read_to_end(&mut head)
            .ok()?;
        self.rewind().ok()?;
        Some(head)
    }

    /// Treat the candidate as ending after `limit` bytes: no byte at or past
    /// it is read, and a read there returns 0 bytes.
    fn limit(&mut self, limit: u64) {
        self.end = self.end.min(limit);
    }

    /// The window that holds the current position, if one does.
    fn held(&self) -> Option<usize> {
        [self.newest, 1 - self.newest]
            .into_iter()
            .find(|&index| self.windows[index].from(self.position).is_some())
    }
}

/// A read fills `buf` across windows, and returns fewer bytes only at the
/// end of the candidate or after an error. zune-jpeg reads a segment's
/// length with one read call and takes any byte it did not get as zero, so a
/// read that stopped at the end of a window would misread the segment.
impl Read for CandidateReader<'_, '_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut done = 0;
        while done < buf.len() {
            let available = match self.fill_buf() {
                Ok(available) => available,
                Err(error) if done == 0 => return Err(error),
                Err(_) => break,
            };
            if available.is_empty() {
                break;
            }
            let n = available.len().min(buf.len() - done);
            buf[done..done + n].copy_from_slice(&available[..n]);
            self.consume(n);
            done += n;
        }
        Ok(done)
    }
}

impl BufRead for CandidateReader<'_, '_> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.failed {
            return Err(unreadable());
        }
        if self.position >= self.end {
            return Ok(&[]);
        }
        let index = match self.held() {
            Some(index) => index,
            None => {
                if self.context.is_cancelled() {
                    return Err(io::Error::other("the run was cancelled"));
                }
                let index = 1 - self.newest;
                let len = (self.end - self.position).min(AssetContext::MAX_READ as u64) as usize;
                let window = &mut self.windows[index];
                window.start = self.position;
                if window.bytes.len() < len {
                    window.bytes.resize(len, 0);
                }
                let read = self
                    .context
                    .read_candidate(self.position, &mut window.bytes[..len]);
                let Ok(n) = read else {
                    window.len = 0;
                    self.failed = true;
                    return Err(unreadable());
                };
                window.len = n.min(len);
                self.newest = index;
                if n == 0 {
                    return Ok(&[]);
                }
                index
            }
        };
        let bytes = self.windows[index]
            .from(self.position)
            .expect("the window holds the position");
        let left = usize::try_from(self.end - self.position).unwrap_or(usize::MAX);
        Ok(&bytes[..bytes.len().min(left)])
    }

    fn consume(&mut self, amount: usize) {
        self.position = self.position.saturating_add(amount as u64);
    }
}

fn unreadable() -> io::Error {
    io::Error::other("the candidate could not be read")
}

impl Seek for CandidateReader<'_, '_> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let target = match to {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta),
            SeekFrom::End(delta) => self.end.checked_add_signed(delta),
        };
        self.position = target.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "a seek before the start")
        })?;
        Ok(self.position)
    }
}
