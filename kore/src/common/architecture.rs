use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use tracing::{debug, warn};

pub const GAME: &str = "game";
pub const MODS: &str = "mods";
pub const STUDIO: &str = "studio";
pub const WORK: &str = ".work";

struct Anchor {
    home: PathBuf,
    note: String,
    volatile: bool,
}

static ANCHOR: OnceLock<Anchor> = OnceLock::new();

pub fn anchor() {
    if cfg!(debug_assertions) {
        return;
    }

    let Some(home) = env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf)) else {
        return;
    };

    let settled = env::current_dir().is_ok_and(|cwd| cwd == home);

    let note = match env::set_current_dir(&home) {
        Ok(()) if settled => format!("Running from {}", home.display()),
        Ok(()) => format!("Anchored the working directory to {}", home.display()),
        Err(err) => format!("Could not anchor the working directory to {}: {}", home.display(), err),
    };

    let volatile = under_temp(&home);

    let _ = ANCHOR.set(Anchor { home, note, volatile });
}

pub fn anchored() -> Option<&'static str> {
    ANCHOR.get().map(|anchor| anchor.note.as_str())
}

pub fn volatile() -> Option<&'static Path> {
    ANCHOR.get().filter(|anchor| anchor.volatile).map(|anchor| anchor.home.as_path())
}

fn under_temp(home: &Path) -> bool {
    let settled = |path: &Path| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    settled(home).starts_with(settled(&env::temp_dir()))
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_temp_sees_through_an_unresolved_temp_path() {
        let nested = env::temp_dir().join("bcc-volatile-probe").join("Temp1_bcc.zip");

        fs::create_dir_all(&nested).expect("probe dir");

        // Windows hands back a short-form %TEMP% while current_exe() is long-form,
        // so both sides have to be resolved before they can be compared.
        assert!(under_temp(&nested));

        let _ = fs::remove_dir_all(env::temp_dir().join("bcc-volatile-probe"));
    }

    #[test]
    fn a_real_install_directory_is_not_volatile() {
        let Ok(cwd) = env::current_dir() else {
            return;
        };

        assert!(!under_temp(&cwd));
    }
}
