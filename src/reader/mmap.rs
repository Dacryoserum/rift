use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use memmap2::{Mmap, MmapOptions};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Encoding {
    Utf8,
    Latin1,
    Unknown,
}

pub struct MmapReader {
    pub mmap: Arc<Mmap>,
    pub file_size: u64,
    pub encoding: Encoding,
}

impl Clone for MmapReader {
    fn clone(&self) -> Self {
        Self {
            mmap: Arc::clone(&self.mmap),
            file_size: self.file_size,
            encoding: self.encoding,
        }
    }
}

impl MmapReader {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let file = std::fs::File::open(path)?;
        let meta = file.metadata()?;
        let file_size = meta.len();

        let mmap = if file_size == 0 {
            // mmap(2) rejects length-0 mappings on Linux; use a 1-byte anonymous mapping
            // that is never accessed (all read paths guard on file_size / mmap.len()).
            let anon = MmapOptions::new().len(1).map_anon()?;
            return Ok(Self {
                mmap: Arc::new(anon.make_read_only()?),
                file_size: 0,
                encoding: Encoding::Utf8,
            });
        } else {
            unsafe { Mmap::map(&file)? }
        };

        let encoding = detect_encoding(&mmap);

        Ok(Self {
            mmap: Arc::new(mmap),
            file_size,
            encoding,
        })
    }

    /// Zero-copy slice of bytes starting at `offset`, up to `max_len` bytes.
    pub fn bytes_at(&self, offset: u64, max_len: usize) -> &[u8] {
        let start = offset as usize;
        let len = self.mmap.len();
        if start >= len {
            return &[];
        }
        let end = (start + max_len).min(len);
        &self.mmap[start..end]
    }

    /// Returns (line_bytes, next_line_byte_offset).
    /// Reads from `offset` until newline or max_bytes.
    /// line_bytes does NOT include the newline or carriage return.
    pub fn line_bytes_at(&self, offset: u64, max_bytes: usize) -> (&[u8], u64) {
        let start = offset as usize;
        let mmap_len = self.mmap.len();

        if start >= mmap_len {
            return (&[], offset);
        }

        let end_search = (start + max_bytes).min(mmap_len);
        let slice = &self.mmap[start..end_search];

        if let Some(nl_pos) = memchr_newline(slice) {
            let line_end = nl_pos;
            // Strip trailing \r if present
            let content_end = if line_end > 0 && slice[line_end - 1] == b'\r' {
                line_end - 1
            } else {
                line_end
            };
            let next_offset = offset + nl_pos as u64 + 1;
            (&self.mmap[start..start + content_end], next_offset)
        } else {
            // No newline found in window
            let content = &self.mmap[start..start + slice.len()];
            let next_offset = offset + slice.len() as u64;
            (content, next_offset)
        }
    }

    /// Decode bytes to string (UTF-8 lossy or Latin-1 based on detected encoding).
    pub fn decode<'a>(&self, bytes: &'a [u8]) -> Cow<'a, str> {
        match self.encoding {
            Encoding::Utf8 => String::from_utf8_lossy(bytes),
            Encoding::Latin1 | Encoding::Unknown => {
                // Try UTF-8 first, fall back to Latin-1 byte-by-byte
                match std::str::from_utf8(bytes) {
                    Ok(s) => Cow::Borrowed(s),
                    Err(_) => {
                        let s: String = bytes.iter().map(|&b| b as char).collect();
                        Cow::Owned(s)
                    }
                }
            }
        }
    }
}

fn memchr_newline(slice: &[u8]) -> Option<usize> {
    slice.iter().position(|&b| b == b'\n')
}

fn detect_encoding(mmap: &Mmap) -> Encoding {
    let sample_len = mmap.len().min(8192);
    let sample = &mmap[..sample_len];

    if sample.is_empty() {
        return Encoding::Utf8;
    }

    // Check if valid UTF-8
    if std::str::from_utf8(sample).is_ok() {
        return Encoding::Utf8;
    }

    // Check Latin-1: all bytes representable
    let non_printable_count = sample
        .iter()
        .filter(|&&b| b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r')
        .count();

    if non_printable_count as f64 / (sample.len() as f64) < 0.05 {
        Encoding::Latin1
    } else {
        Encoding::Unknown
    }
}
