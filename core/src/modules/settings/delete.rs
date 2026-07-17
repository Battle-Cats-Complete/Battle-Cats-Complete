use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tracing::{debug, error};

const IDLE: u8 = 0;
const DELETING: u8 = 1;
const DONE: u8 = 2;

#[derive(Clone)]
pub struct FolderDeleter {
    state: Arc<AtomicU8>,
    success_time: Option<Instant>,
}

impl Default for FolderDeleter {
    fn default() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(IDLE)),
            success_time: None,
        }
    }
}

impl FolderDeleter {
    pub fn start(&mut self, target_path: impl Into<PathBuf>) {
        let path = target_path.into();
        let atomic_state = Arc::clone(&self.state);

        debug!("Initializing folder deletion for path: {:?}", path);
        self.state.store(DELETING, Ordering::SeqCst);
        self.success_time = None;

        thread::spawn(move || {
            if let Err(delete_error) = fs::remove_dir_all(&path) {
                error!("Failed to delete folder {:?}: {}", path, delete_error);
            } else {
                debug!("Folder deletion completed successfully.");
            }
            atomic_state.store(DONE, Ordering::SeqCst);
        });
    }

    pub fn update(&mut self) {
        let current_state = self.state.load(Ordering::SeqCst);

        if current_state == DONE && self.success_time.is_none() {
            self.success_time = Some(Instant::now());
        }

        if let Some(time) = self.success_time {
            if time.elapsed() > Duration::from_secs(2) {
                self.state.store(IDLE, Ordering::SeqCst);
                self.success_time = None;
            }
        }
    }

    pub fn is_deleting(&self) -> bool {
        self.state.load(Ordering::SeqCst) == DELETING
    }

    pub fn is_done(&self) -> bool {
        self.state.load(Ordering::SeqCst) == DONE
    }

    pub fn is_active(&self) -> bool {
        self.is_deleting() || self.is_done()
    }
}