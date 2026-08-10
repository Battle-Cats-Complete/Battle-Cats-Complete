use serde::{Deserialize, Serialize};

use crate::common::io::cache;

use super::{CatStore, EnemyStore, StageStore, Vds};

struct ContentCache;

impl cache::CacheSpec for ContentCache {
    type Data = ContentStore;
    const FILE: &'static str = "virtual_data_store.bin";
    const VERSION: u32 = 1;
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ContentStore {
    cats: CatStore,
    enemies: EnemyStore,
    stages: StageStore,
}

impl ContentStore {
    pub fn capture(vds: &Vds) -> Self {
        Self {
            cats: vds.cats.clone(),
            enemies: vds.enemies.clone(),
            stages: vds.stages.clone(),
        }
    }

    pub fn apply(self, vds: &mut Vds) {
        vds.cats = self.cats;
        vds.enemies = self.enemies;
        vds.stages = self.stages;
    }

    pub fn save(&self, hash: u64) {
        cache::write::<ContentCache>(hash, self);
    }

    pub fn load(hash: u64) -> Option<Self> {
        let (stored, content) = cache::read::<ContentCache>()?;

        if stored != hash {
            return None;
        }

        Some(content)
    }
}
