pub mod engine;
pub mod fuzzy;

pub use engine::{SearchEngine, SearchQuery, SearchResult, SearchDirection};
pub use fuzzy::FuzzySearch;
