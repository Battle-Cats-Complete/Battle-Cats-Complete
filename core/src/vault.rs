mod vds;
mod vfs;

use std::fmt;

use crate::common::io::cache;
use crate::modules::settings::{ScannerConfig, Settings};

pub use vds::{CatStore, ContentStore, EnemyStore, StageStore, Vds};
pub use vfs::{Conflict, Mount, Target, Vfs, VfsError};

pub struct Vault {
    pub vfs: Vfs,
    pub vds: Vds,
}

impl fmt::Debug for Vault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vault").finish_non_exhaustive()
    }
}

impl Vault {
    pub fn new(settings: &Settings) -> Self {
        Self::with_priority(&settings.general.language_priority)
    }

    pub fn with_priority(order: &[String]) -> Self {
        Self {
            vfs: Vfs::with_priority(order),
            vds: Vds::default(),
        }
    }

    pub fn hash(active_mod: Option<&str>) -> u64 {
        cache::get_game_hash(active_mod)
    }

    pub fn key(config: &ScannerConfig) -> u64 {
        cache::content_hash(config)
    }

    pub fn key_for(index: u64, config: &ScannerConfig) -> u64 {
        cache::content_key(index, config)
    }

    pub fn evict(&self, key: &str) {
        self.vfs.evict(key);
        self.vds.evict(key);
    }

    pub fn purge(&self, keys: &[Box<str>]) {
        self.vfs.purge(keys);
        self.vds.purge(keys);
    }

    pub fn priority(&self, order: &[String]) {
        self.vfs.priority(order);
        self.vds.clear();
    }
}
