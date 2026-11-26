//! The TIFF header and the first image file directory (IFD), read through a
//! [`CandidateReader`]. Entries stream through one fixed buffer and only the
//! tags a caller asks for are kept, so a directory of any number of entries
//! costs time but no memory. There is no entry cap beyond the directory's
//! own count and the file's length.
use crate::io::CandidateReader;
use std::io::{Read, Seek, SeekFrom};

/// Whether numbers are big-endian (`MM`) or little-endian (`II`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Order {
    Little,
    Big,
}

impl Order {
    /// The unsigned number in `bytes`, at most 8 of them.
    fn number(self, bytes: &[u8]) -> u64 {
        let mut value = 0u64;
        match self {
            Self::Big => bytes
                .iter()
                .for_each(|&b| value = value << 8 | u64::from(b)),
            Self::Little => bytes
                .iter()
                .rev()
                .for_each(|&b| value = value << 8 | u64::from(b)),
        }
        value
    }
}

/// The size in bytes of one value of a TIFF field type, or `None` for a
/// type this reader does not know.
pub(crate) fn type_size(kind: u16) -> Option<u64> {
    Some(match kind {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 | 13 => 4,
        5 | 10 | 12 | 16 | 17 | 18 => 8,
        _ => return None,
    })
}

/// One kept directory entry. `field` holds the value bytes when they fit in
/// the entry, or else the offset of the values.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub tag: u16,
    pub kind: u16,
    pub count: u64,
    field: [u8; 8],
}

/// The first image's directory: the entries kept, and how to read them.
pub(crate) struct Directory {
    pub order: Order,
    /// BigTIFF: 8-byte offsets and counts.
    pub big: bool,
    entries: Vec<Entry>,
}

/// Why an array of values was not read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArrayError {
    /// It holds more values than the caller allows.
    TooLong,
    /// Its type is not a whole-number type, or its bytes could not be read.
    Unreadable,
}

/// The entries one buffer fill holds: 4 KiB of classic or BigTIFF entries.
const BUFFER: usize = 4096;

/// Read `buf.len()` bytes at `offset`, or `None` when the input ends first
/// or a read fails.
fn read_exact_at(reader: &mut CandidateReader<'_>, offset: u64, buf: &mut [u8]) -> Option<()> {
    reader.seek(SeekFrom::Start(offset)).ok()?;
    reader.read_exact(buf).ok()
}

/// Read the header, then the first directory, keeping the entries whose tag
/// is in `keep`. The first entry for a tag wins. `size` is the input's
/// length. `None` when the header or directory cannot be read.
pub(crate) fn first_directory(
    reader: &mut CandidateReader<'_>,
    size: u64,
    keep: &[u16],
) -> Option<Directory> {
    let mut header = [0; 16];
    read_exact_at(reader, 0, &mut header[..8])?;
    let order = match &header[..2] {
        b"II" => Order::Little,
        b"MM" => Order::Big,
        _ => return None,
    };
    let (big, offset) = match order.number(&header[2..4]) {
        42 => (false, order.number(&header[4..8])),
        43 => {
            read_exact_at(reader, 8, &mut header[8..16])?;
            if order.number(&header[4..6]) != 8 || order.number(&header[6..8]) != 0 {
                return None;
            }
            (true, order.number(&header[8..16]))
        }
        _ => return None,
    };
    let (count_size, entry_size) = if big { (8, 20) } else { (2, 12) };
    let mut count = [0; 8];
    read_exact_at(reader, offset, &mut count[..count_size])?;
    let count = order.number(&count[..count_size]);
    let entries_start = offset.checked_add(count_size as u64)?;
    let entries_end = count
        .checked_mul(entry_size as u64)
        .and_then(|len| entries_start.checked_add(len))?;
    if entries_end > size {
        return None;
    }
    let mut kept: Vec<Entry> = Vec::new();
    let mut buffer = [0; BUFFER];
    let per_fill = (BUFFER / entry_size) as u64;
    let mut done = 0;
    while done < count {
        let now = per_fill.min(count - done);
        let bytes = &mut buffer[..now as usize * entry_size];
        read_exact_at(reader, entries_start + done * entry_size as u64, bytes)?;
        for raw in bytes.chunks_exact(entry_size) {
            let tag = order.number(&raw[..2]) as u16;
            if !keep.contains(&tag) || kept.iter().any(|e| e.tag == tag) {
                continue;
            }
            let (count, field) = if big {
                (order.number(&raw[4..12]), &raw[12..20])
            } else {
                (order.number(&raw[4..8]), &raw[8..12])
            };
            let mut entry = Entry {
                tag,
                kind: order.number(&raw[2..4]) as u16,
                count,
                field: [0; 8],
            };
            entry.field[..field.len()].copy_from_slice(field);
            kept.push(entry);
        }
        done += now;
    }
    Some(Directory {
        order,
        big,
        entries: kept,
    })
}

impl Directory {
    fn entry(&self, tag: u16) -> Option<&Entry> {
        self.entries.iter().find(|e| e.tag == tag)
    }

    /// How many bytes an entry holds in place of an offset.
    fn inline(&self) -> u64 {
        if self.big {
            8
        } else {
            4
        }
    }

    /// Whether the directory has an entry for `tag`.
    pub(crate) fn has(&self, tag: u16) -> bool {
        self.entry(tag).is_some()
    }

    /// The one whole-number value of `tag`, when it holds exactly one value
    /// of a whole-number type, stored in the entry.
    pub(crate) fn scalar(&self, tag: u16) -> Option<u64> {
        let entry = self.entry(tag)?;
        if entry.count != 1 {
            return None;
        }
        let size = whole_number_size(entry.kind)?;
        Some(self.order.number(&entry.field[..size as usize]))
    }

    /// Every whole-number value of `tag`, at most `cap` of them, read from
    /// the entry or from where its offset points. `Ok(None)` when the
    /// directory has no entry for `tag`.
    pub(crate) fn array(
        &self,
        reader: &mut CandidateReader<'_>,
        tag: u16,
        cap: u64,
    ) -> Result<Option<Vec<u64>>, ArrayError> {
        let Some(entry) = self.entry(tag) else {
            return Ok(None);
        };
        if entry.count > cap {
            return Err(ArrayError::TooLong);
        }
        let size = whole_number_size(entry.kind).ok_or(ArrayError::Unreadable)?;
        let len = (entry.count * size) as usize;
        let mut bytes = vec![0; len];
        if len as u64 <= self.inline() {
            bytes.copy_from_slice(&entry.field[..len]);
        } else {
            let offset = self.order.number(&entry.field[..self.inline() as usize]);
            read_exact_at(reader, offset, &mut bytes).ok_or(ArrayError::Unreadable)?;
        }
        Ok(Some(
            bytes
                .chunks_exact(size as usize)
                .map(|value| self.order.number(value))
                .collect(),
        ))
    }
}

/// The size of one value of a whole-number type: BYTE, SHORT, LONG or
/// LONG8.
fn whole_number_size(kind: u16) -> Option<u64> {
    matches!(kind, 1 | 3 | 4 | 16).then(|| type_size(kind).expect("a known type"))
}
