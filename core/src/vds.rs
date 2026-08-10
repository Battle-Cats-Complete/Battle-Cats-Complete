mod cat;
mod content;
mod enemy;
mod stage;

use std::fs;
use std::sync::{Arc, RwLock};

use tracing::warn;

use crate::Vfs;

pub use cat::CatStore;
pub use content::ContentStore;
pub use enemy::EnemyStore;
pub use stage::StageStore;

type Slot<T> = RwLock<Option<Arc<T>>>;

#[derive(Default)]
pub struct Vds {
    pub cats: CatStore,
    pub enemies: EnemyStore,
    pub stages: StageStore,
}

impl Vds {
    pub fn evict(&self, filename: &str) {
        self.cats.evict(filename);
        self.enemies.evict(filename);
        self.stages.evict(filename);
    }

    pub fn clear(&self) {
        self.cats.clear();
        self.enemies.clear();
        self.stages.clear();
    }
}

fn cached<T>(slot: &Slot<T>, build: impl FnOnce() -> T) -> Arc<T> {
    if let Ok(current) = slot.read()
        && let Some(value) = current.as_ref()
    {
        return Arc::clone(value);
    }

    let value = Arc::new(build());

    if let Ok(mut current) = slot.write() {
        *current = Some(Arc::clone(&value));
    }

    value
}

fn reset<T>(slot: &Slot<T>) {
    if let Ok(mut current) = slot.write() {
        *current = None;
    }
}

fn snapshot<T>(slot: &Slot<T>) -> Slot<T> {
    RwLock::new(slot.read().ok().and_then(|current| current.clone()))
}

fn parsed<T, E>(vfs: &Vfs, filename: &str, parse: impl FnOnce(Arc<[u8]>) -> Result<T, E>) -> Option<T> {
    let bytes = vfs.load(filename)?;

    let Ok(value) = parse(bytes) else {
        return None;
    };

    vfs.evict(filename);
    Some(value)
}

fn layered(vfs: &Vfs, filename: &str) -> Vec<Vec<u8>> {
    vfs.list(filename)
        .iter()
        .filter_map(|path| {
            fs::read(path)
                .inspect_err(|err| warn!(path = %path.display(), "vds layered read failed: {}", err))
                .ok()
        })
        .collect()
}
