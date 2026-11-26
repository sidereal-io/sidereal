//! How the pack's format code reads its input. [`ImageInput`] is one
//! positioned read, so the same format code runs over the Asset context in
//! the host and over a plain file. [`CandidateReader`] gives a decoder
//! `BufRead + Seek` over it.
use sidereal_plugin_abi::AssetContext;
use std::io::{self, BufRead, Read, Seek, SeekFrom};

/// The bytes a format reads: one positioned read.
pub(crate) trait ImageInput {
    /// Read bytes starting at `offset` into `buf`, returning how many were
    /// read. Zero means the end of the input.
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize>;
}

/// The helper's input: a seekable file, read with positioned reads.
pub(crate) struct FileInput<'f>(pub &'f std::fs::File);

impl ImageInput for FileInput<'_> {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        std::os::unix::fs::FileExt::read_at(self.0, buf, offset)
    }
}

/// An Operator's candidate, read through its Asset context. A read after
/// the host cancelled the run fails.
pub(crate) struct ContextInput<'c, 'a>(pub &'c mut AssetContext<'a>);

impl ImageInput for ContextInput<'_, '_> {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        if self.0.is_cancelled() {
            return Err(io::Error::other("the run was cancelled"));
        }
        self.0
            .read_candidate(offset, buf)
            .map_err(|_| io::Error::other("the candidate could not be read"))
    }
}

/// The candidate's first `len` bytes, or fewer when the candidate is
/// shorter; `None` when a read fails.
pub(crate) fn head(context: &mut AssetContext<'_>, len: usize) -> Option<Vec<u8>> {
    let mut head = vec![0; len];
    let mut read = 0;
    while read < head.len() {
        match context.read_candidate(read as u64, &mut head[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(_) => return None,
        }
    }
    head.truncate(read);
    Some(head)
}

/// The most bytes one fill of a [`CandidateReader`] window asks for.
const WINDOW: usize = AssetContext::MAX_READ;

/// One window of input bytes, as one read filled it. Its buffer only grows,
/// so a short read does not make the next fill zero the buffer again.
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

/// Reads an [`ImageInput`] as `BufRead + Seek`, which the decoders need. It
/// keeps the last two windows, each filled by one read at the current
/// position, so a seek back into the window before needs no new read. A
/// seek only moves the position.
pub(crate) struct CandidateReader<'r> {
    input: &'r mut dyn ImageInput,
    position: u64,
    /// Where the input ends for this reader: no byte at or past it is read.
    end: u64,
    windows: [Window; 2],
    /// The window filled last.
    newest: usize,
    /// A read failed. Every later read fails too, so a decoder never goes on
    /// past bytes the input did not give.
    failed: bool,
}

impl<'r> CandidateReader<'r> {
    /// A reader over the first `size` bytes of `input`.
    pub(crate) fn new(input: &'r mut dyn ImageInput, size: u64) -> Self {
        Self {
            input,
            position: 0,
            end: size,
            windows: Default::default(),
            newest: 0,
            failed: false,
        }
    }

    /// Treat the input as ending after `limit` bytes: no byte at or past it
    /// is read, and a read there returns 0 bytes.
    pub(crate) fn limit(&mut self, limit: u64) {
        self.end = self.end.min(limit);
    }

    /// Whether a read of the input failed. A failed read is a storage
    /// problem, whatever the decoder then said.
    pub(crate) fn failed(&self) -> bool {
        self.failed
    }

    /// The window that holds the current position, if one does.
    fn held(&self) -> Option<usize> {
        [self.newest, 1 - self.newest]
            .into_iter()
            .find(|&index| self.windows[index].from(self.position).is_some())
    }
}

/// A read fills `buf` across windows, and returns fewer bytes only at the
/// end of the input or after an error. zune-jpeg reads a segment's length
/// with one read call and takes any byte it did not get as zero, so a read
/// that stopped at the end of a window would misread the segment.
impl Read for CandidateReader<'_> {
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

impl BufRead for CandidateReader<'_> {
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
                let index = 1 - self.newest;
                let len = (self.end - self.position).min(WINDOW as u64) as usize;
                let window = &mut self.windows[index];
                window.start = self.position;
                if window.bytes.len() < len {
                    window.bytes.resize(len, 0);
                }
                let read = self.input.read_at(self.position, &mut window.bytes[..len]);
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

impl Seek for CandidateReader<'_> {
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
