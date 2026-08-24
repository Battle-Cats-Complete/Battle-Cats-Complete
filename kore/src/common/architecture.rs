use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use tracing::{debug, warn};

pub const GAME: &str = "game";
pub const MODS: &str = "mods";
pub const WORK: &str = ".work";

static ACTIVE_JOBS: AtomicUsize = AtomicUsize::new(0);

pub struct Scratch;

impl Scratch {
    #[must_use]
    pub fn claim() -> Self {
        ACTIVE_JOBS.fetch_add(1, Ordering::AcqRel);
        Self
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        ACTIVE_JOBS.fetch_sub(1, Ordering::AcqRel);
        work_cleanup();
    }
}

pub fn work_cleanup() {
    let outstanding = ACTIVE_JOBS.load(Ordering::Acquire);

    if outstanding > 0 {
        debug!(outstanding, "{} is still in use, leaving it to the last job out", WORK);
        return;
    }

    match fs::remove_dir_all(WORK) {
        Ok(()) => debug!("Cleared {}", WORK),
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => warn!("Failed to clear {}: {}", WORK, err),
    }
}

pub fn has_content(path: &Path) -> bool {
    fs::read_dir(path).is_ok_and(|mut entries| entries.next().is_some())
}

pub fn game_present() -> bool {
    has_content(Path::new(GAME))
}
