use crate::modules::settings::Settings;
use crate::{vds::Vds, vfs::Vfs};

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

    pub fn evict_everything(&self, key: &str) {
        self.vfs.evict(key);
        self.vds.evict(key);
    }

    pub fn priority(&self, order: &[String]) {
        self.vfs.priority(order);
        self.vds.clear();
    }
}
