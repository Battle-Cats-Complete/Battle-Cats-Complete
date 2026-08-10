use std::fs;
use std::path::Path;

pub const GAME: &str = "game";
pub const MODS: &str = "mods";
pub const APP: &str = "game/app";
pub const RAW: &str = "game/raw";

pub const TRANSIENT: [&str; 2] = ["app", "raw"];

pub const PACKAGES: &str = "packages";

pub const MOD_TRANSIENT: [&str; 1] = ["app"];

pub fn has_content(path: &Path) -> bool {
    fs::read_dir(path).is_ok_and(|mut entries| entries.next().is_some())
}

pub fn game_present() -> bool {
    has_content(Path::new(GAME))
}
