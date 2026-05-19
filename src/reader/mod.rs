pub mod mmap;
pub mod index;

pub use mmap::MmapReader;
pub use index::{LineIndex, LinePosition, IndexProgress, IndexMessage, IndexPhase};
