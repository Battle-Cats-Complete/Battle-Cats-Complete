use std::fs;
use std::path::PathBuf;

use directories::BaseDirs;
use tracing::error;

pub const APP_DIR: &str = "battle-cats-complete";

pub fn config() -> Option<PathBuf> {
    ensure(base()?.config_dir().join(APP_DIR))
}

pub fn state() -> Option<PathBuf> {
    let base_dirs = base()?;

    let path = base_dirs
        .state_dir()
        .map_or_else(|| base_dirs.data_local_dir().join(APP_DIR).join("state"), |state_root| state_root.join(APP_DIR));

    ensure(path)
}

pub fn cache_path() -> Option<PathBuf> {
    let path = base()?.cache_dir().join(APP_DIR);

    Some(if cfg!(target_os = "windows") { path.join("cache") } else { path })
}

pub fn cache() -> Option<PathBuf> {
    ensure(cache_path()?)
}

pub fn data() -> Option<PathBuf> {
    ensure(base()?.data_local_dir().join(APP_DIR))
}

fn base() -> Option<BaseDirs> {
    let Some(base_dirs) = BaseDirs::new() else {
        error!("Failed to retrieve BaseDirs. Application directories are unavailable.");
        return None;
    };

    Some(base_dirs)
}

fn ensure(path: PathBuf) -> Option<PathBuf> {
    if let Err(err) = fs::create_dir_all(&path) {
        error!("Failed to create application directory {:?}: {}", path, err);
        return None;
    }

    Some(path)
}
