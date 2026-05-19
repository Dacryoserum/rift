use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc, RwLock,
};

use memmap2::Mmap;
use regex::Regex;

use crate::event::BackgroundEvent;
use crate::reader::index::LineIndex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub pattern: String,
    pub regex: Regex,
    pub direction: SearchDirection,
}

impl SearchQuery {
    pub fn new(pattern: &str, direction: SearchDirection) -> anyhow::Result<Self> {
        let regex = Regex::new(pattern)?;
        Ok(Self {
            pattern: pattern.to_owned(),
            regex,
            direction,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub line_num: u64,
    pub byte_offset: u64,
    pub match_start: usize,
    pub match_end: usize,
}

pub struct SearchEngine {
    cancel: Arc<AtomicBool>,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Cancel any running search and start a new one.
    pub fn start(
        &mut self,
        mmap: Arc<Mmap>,
        index: Arc<RwLock<LineIndex>>,
        query: SearchQuery,
        start_offset: u64,
        bg_tx: Sender<BackgroundEvent>,
    ) {
        // Cancel previous search
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = Arc::clone(&self.cancel);

        std::thread::spawn(move || {
            run_search(mmap, index, query, start_offset, cancel, bg_tx);
        });
    }

    pub fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn run_search(
    mmap: Arc<Mmap>,
    index: Arc<RwLock<LineIndex>>,
    query: SearchQuery,
    start_offset: u64,
    cancel: Arc<AtomicBool>,
    bg_tx: Sender<BackgroundEvent>,
) {
    let file_size = mmap.len() as u64;
    let data: &[u8] = &mmap;

    // Collect line offsets from index
    let (offsets, _phase) = {
        let idx = match index.read() {
            Ok(g) => g,
            Err(_) => return,
        };
        let count = idx.line_count() as usize;
        let mut offs = Vec::with_capacity(count);
        for i in 0..count as u64 {
            match idx.offset_for_line(i) {
                crate::reader::index::LinePosition::Exact { byte_offset, .. } => {
                    offs.push(byte_offset)
                }
                crate::reader::index::LinePosition::Estimated { byte_offset, .. } => {
                    offs.push(byte_offset)
                }
            }
        }
        let p = idx.phase();
        (offs, p)
    };

    // Find starting line
    let start_line = offsets
        .binary_search(&start_offset)
        .unwrap_or_else(|i| i.saturating_sub(1));

    let line_count = offsets.len();

    let indices: Vec<usize> = match query.direction {
        SearchDirection::Forward => {
            // start_line..end, then 0..start_line (wrap)
            let mut v: Vec<usize> = (start_line..line_count).collect();
            v.extend(0..start_line);
            v
        }
        SearchDirection::Backward => {
            let mut v: Vec<usize> = (0..=start_line).rev().collect();
            v.extend((start_line + 1..line_count).rev());
            v
        }
    };

    for (check_count, &line_idx) in indices.iter().enumerate() {
        if check_count % 1000 == 0 && cancel.load(Ordering::Relaxed) {
            return;
        }

        let line_offset = offsets[line_idx];
        let next_offset = if line_idx + 1 < line_count {
            offsets[line_idx + 1]
        } else {
            file_size
        };

        let line_len = (next_offset.saturating_sub(line_offset)) as usize;
        if line_offset as usize >= data.len() {
            continue;
        }
        let end = (line_offset as usize + line_len).min(data.len());
        let line_bytes = &data[line_offset as usize..end];

        // Strip trailing newline/CR
        let line_bytes = if line_bytes.last() == Some(&b'\n') {
            &line_bytes[..line_bytes.len() - 1]
        } else {
            line_bytes
        };
        let line_bytes = if line_bytes.last() == Some(&b'\r') {
            &line_bytes[..line_bytes.len() - 1]
        } else {
            line_bytes
        };

        let line_str = String::from_utf8_lossy(line_bytes);

        if let Some(m) = query.regex.find(&line_str) {
            let result = SearchResult {
                line_num: line_idx as u64,
                byte_offset: line_offset,
                match_start: m.start(),
                match_end: m.end(),
            };
            if bg_tx.send(BackgroundEvent::SearchResult(result)).is_err() {
                return;
            }
        }
    }

    let _ = bg_tx.send(BackgroundEvent::SearchComplete);
}
