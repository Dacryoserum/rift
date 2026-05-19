use std::sync::mpsc::Sender;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
};

use memmap2::Mmap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndexPhase {
    Sampling,
    Scanning,
    Complete,
}

#[derive(Debug, Clone)]
pub struct IndexProgress {
    pub phase: IndexPhase,
    pub bytes_scanned: u64,
    pub lines_found: u64,
    pub estimated_total: Option<u64>,
    pub file_size: u64,
}

pub enum IndexMessage {
    Progress(IndexProgress),
    Complete,
    Error(String),
}

pub struct LineIndex {
    offsets: Vec<u64>,
    phase: IndexPhase,
    file_size: u64,
}

#[derive(Debug, Clone)]
pub enum LinePosition {
    Exact { byte_offset: u64, line_num: u64 },
    Estimated { byte_offset: u64, line_num: u64 },
}

impl LineIndex {
    pub fn new(file_size: u64) -> Self {
        Self {
            offsets: vec![0],
            phase: IndexPhase::Sampling,
            file_size,
        }
    }

    pub fn line_count(&self) -> u64 {
        self.offsets.len() as u64
    }

    pub fn phase(&self) -> IndexPhase {
        self.phase
    }

    pub fn offset_for_line(&self, line_num: u64) -> LinePosition {
        let count = self.offsets.len() as u64;
        if line_num >= count {
            // Estimate beyond known range
            if count <= 1 {
                return LinePosition::Estimated {
                    byte_offset: 0,
                    line_num: 0,
                };
            }
            let last_offset = self.offsets[count as usize - 1];
            let avg_line_size = last_offset / count.max(1);
            let estimated_offset =
                (last_offset + avg_line_size * (line_num - count + 1)).min(self.file_size);
            LinePosition::Estimated {
                byte_offset: estimated_offset,
                line_num,
            }
        } else {
            let byte_offset = self.offsets[line_num as usize];
            if self.phase == IndexPhase::Complete {
                LinePosition::Exact {
                    byte_offset,
                    line_num,
                }
            } else {
                LinePosition::Estimated {
                    byte_offset,
                    line_num,
                }
            }
        }
    }

    pub fn line_at_offset(&self, offset: u64) -> u64 {
        match self.offsets.binary_search(&offset) {
            Ok(idx) => idx as u64,
            Err(idx) => {
                if idx == 0 {
                    0
                } else {
                    (idx - 1) as u64
                }
            }
        }
    }

    fn set_phase(&mut self, phase: IndexPhase) {
        self.phase = phase;
    }

    fn replace_offsets(&mut self, offsets: Vec<u64>) {
        self.offsets = offsets;
    }
}

/// Spawn the background indexer thread. Returns shared index.
pub fn spawn_indexer(
    mmap: Arc<Mmap>,
    file_size: u64,
    sample_interval: u64,
    cancel: Arc<AtomicBool>,
    tx: Sender<IndexMessage>,
) -> Arc<RwLock<LineIndex>> {
    let index = Arc::new(RwLock::new(LineIndex::new(file_size)));
    let index_clone = Arc::clone(&index);

    std::thread::spawn(move || {
        run_indexer(mmap, file_size, sample_interval, cancel, tx, index_clone);
    });

    index
}

fn run_indexer(
    mmap: Arc<Mmap>,
    file_size: u64,
    sample_interval: u64,
    cancel: Arc<AtomicBool>,
    tx: Sender<IndexMessage>,
    index: Arc<RwLock<LineIndex>>,
) {
    if file_size == 0 {
        let _ = tx.send(IndexMessage::Complete);
        if let Ok(mut idx) = index.write() {
            idx.set_phase(IndexPhase::Complete);
        }
        return;
    }

    let data: &[u8] = &mmap;

    // ── Phase 1: Sampling ────────────────────────────────────────────────────
    {
        let mut sparse_offsets: Vec<u64> = vec![0];
        let mut pos: u64 = 0;
        let mut entry_count: u64 = 0;

        while pos < file_size {
            if cancel.load(Ordering::Relaxed) {
                return;
            }

            // Step forward by sample_interval, then scan to next newline
            let step_pos = (pos + sample_interval).min(file_size);
            let search_start = step_pos as usize;

            // Find next newline at or after step_pos
            let next_nl = if search_start >= data.len() {
                None
            } else {
                data[search_start..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map(|p| search_start + p)
            };

            if let Some(nl_idx) = next_nl {
                let line_start = nl_idx as u64 + 1;
                if line_start < file_size {
                    sparse_offsets.push(line_start);
                }
                pos = line_start;
            } else {
                break;
            }

            entry_count += 1;
            if entry_count % 10_000 == 0 {
                let progress = IndexProgress {
                    phase: IndexPhase::Sampling,
                    bytes_scanned: pos,
                    lines_found: sparse_offsets.len() as u64,
                    estimated_total: estimate_total_lines(&sparse_offsets, pos, file_size),
                    file_size,
                };
                let _ = tx.send(IndexMessage::Progress(progress));

                // Update shared index with sparse data
                if let Ok(mut idx) = index.write() {
                    idx.replace_offsets(sparse_offsets.clone());
                }
            }
        }

        // Final sparse update
        if let Ok(mut idx) = index.write() {
            idx.replace_offsets(sparse_offsets.clone());
        }

        let progress = IndexProgress {
            phase: IndexPhase::Scanning,
            bytes_scanned: file_size,
            lines_found: sparse_offsets.len() as u64,
            estimated_total: None,
            file_size,
        };
        let _ = tx.send(IndexMessage::Progress(progress));
    }

    if cancel.load(Ordering::Relaxed) {
        return;
    }

    // ── Phase 2: Full sequential scan ────────────────────────────────────────
    {
        if let Ok(mut idx) = index.write() {
            idx.set_phase(IndexPhase::Scanning);
        }

        let mut exact_offsets: Vec<u64> = vec![0];
        let mut lines_found: u64 = 1;

        for (i, &byte) in data.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return;
            }

            if byte == b'\n' {
                let next = i as u64 + 1;
                if next < file_size {
                    exact_offsets.push(next);
                    lines_found += 1;

                    // Send progress without touching the shared index — the sparse
                    // index from phase 1 is good enough for navigation during scan.
                    if lines_found % 2_000_000 == 0 {
                        let estimated_total = estimate_total_lines(&exact_offsets, next, file_size);
                        let _ = tx.send(IndexMessage::Progress(IndexProgress {
                            phase: IndexPhase::Scanning,
                            bytes_scanned: next,
                            lines_found,
                            estimated_total,
                            file_size,
                        }));
                    }
                }
            }
        }

        // Single write at the end — no repeated giant clones under the lock.
        if let Ok(mut idx) = index.write() {
            idx.replace_offsets(exact_offsets);
            idx.set_phase(IndexPhase::Complete);
        }

        let _ = tx.send(IndexMessage::Complete);
    }
}

fn estimate_total_lines(offsets: &[u64], bytes_scanned: u64, file_size: u64) -> Option<u64> {
    if bytes_scanned == 0 || offsets.len() < 2 {
        return None;
    }
    let density = offsets.len() as f64 / bytes_scanned as f64;
    Some((density * file_size as f64) as u64)
}
