pub mod index;
pub mod mmap;

pub use index::{IndexMessage, IndexPhase, IndexProgress, LineIndex, LinePosition};
pub use mmap::MmapReader;
