use crossterm::event::{KeyEvent, MouseEvent};

pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Background(BackgroundEvent),
    Tick,
}

pub enum BackgroundEvent {
    IndexProgress(crate::reader::index::IndexProgress),
    IndexComplete,
    SearchResult(crate::search::engine::SearchResult),
    SearchComplete,
    FileSizeChanged(u64),
}
