use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

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
    pub fn start(&mut self, path: impl Into<PathBuf>) {
        let path = path.into();
        let state_clone = Arc::clone(&self.state);

        self.state.store(DELETING, Ordering::SeqCst);
        self.success_time = None;

        thread::spawn(move || {
            let _ = fs::remove_dir_all(&path);
            state_clone.store(DONE, Ordering::SeqCst);
        });
    }

    pub fn update(&mut self) {
        let current = self.state.load(Ordering::SeqCst);

        if current == DONE && self.success_time.is_none() {
            self.success_time = Some(Instant::now());
        }

        if let Some(time) = self.success_time
            && time.elapsed() > Duration::from_secs(2) {
                self.state.store(IDLE, Ordering::SeqCst);
                self.success_time = None;
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