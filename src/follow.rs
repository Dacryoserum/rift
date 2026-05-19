use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
    Arc,
};
use std::time::Duration;

use crate::event::BackgroundEvent;

pub struct FollowWatcher {
    cancel: Arc<AtomicBool>,
}

impl FollowWatcher {
    /// Spawns a background thread that polls file size every `interval`.
    /// Sends FileSizeChanged(new_size) when size increases.
    pub fn start(
        path: PathBuf,
        current_size: u64,
        interval: Duration,
        tx: Sender<BackgroundEvent>,
    ) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_clone = Arc::clone(&cancel);

        std::thread::spawn(move || {
            let mut last_size = current_size;
            loop {
                if cancel_clone.load(Ordering::Relaxed) {
                    break;
                }

                std::thread::sleep(interval);

                if cancel_clone.load(Ordering::Relaxed) {
                    break;
                }

                let new_size = std::fs::metadata(&path)
                    .map(|m| m.len())
                    .unwrap_or(last_size);

                if new_size > last_size {
                    last_size = new_size;
                    if tx.send(BackgroundEvent::FileSizeChanged(new_size)).is_err() {
                        break;
                    }
                }
            }
        });

        Self { cancel }
    }

    pub fn stop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for FollowWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}
