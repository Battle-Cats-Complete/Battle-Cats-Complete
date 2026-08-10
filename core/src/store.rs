use crate::common::io::cache;
use crate::modules::settings::Settings;
use crate::{Vds, Vfs};

pub struct Store {
    pub vfs: Vfs,
    pub vds: Vds,
}

impl Store {
    pub fn new(settings: &Settings) -> Self {
        Self {
            vfs: Vfs::new(settings),
            vds: Vds::default(),
        }
    }

    pub fn hash(active_mod: Option<&str>) -> u64 {
        cache::get_game_hash(active_mod)
    }

    pub fn evict(&self, key: &str) {
        self.vfs.evict(key);
        self.vds.evict(key);
    }

    pub fn priority(&self, order: &[String]) {
        self.vfs.priority(order);
        self.vds.clear();
    }
}
