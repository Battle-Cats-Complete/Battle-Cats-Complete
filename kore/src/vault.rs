mod vds;
mod vfs;

use std::fmt;
use std::iter;

use crate::common::io::cache;
use crate::domains::settings::{ScannerConfig, Settings};

pub use vds::{CatStore, ContentStore, EnemyStore, StageStore, Vds};
pub use vfs::{Conflict, Listing, Mount, Target, Vfs, VfsError};

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

    pub fn hash(&self, active_mod: Option<&str>) -> u64 {
        cache::index_key(self.vfs.fingerprint(), active_mod)
    }

    pub fn key(&self, config: &ScannerConfig) -> u64 {
        cache::content_hash(self.vfs.fingerprint(), config)
    }

    pub fn key_for(index: u64, config: &ScannerConfig) -> u64 {
        cache::content_key(index, config)
    }

    pub fn evict(&self, key: &str) {
        self.vfs.evict(key);
        self.vds.evict(key);

        if let Some(base) = self.vfs.stripped(key) {
            self.vfs.evict(&base);
            self.vds.evict(&base);
        }
    }

    pub fn purge(&self, keys: &[Box<str>]) {
        let expanded: Vec<Box<str>> = keys
            .iter()
            .flat_map(|key| {
                let base = self.vfs.stripped(key).map(Box::<str>::from);
                iter::once(key.clone()).chain(base)
            })
            .collect();

        self.vfs.purge(&expanded);
        self.vds.purge(&expanded);
    }

    pub fn priority(&self, order: &[String]) {
        self.vfs.priority(order);
        self.vds.clear();
    }
}
