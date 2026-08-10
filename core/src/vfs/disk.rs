use crate::common::io::cache;

use super::Index;

pub(super) struct VfsCache;

impl cache::CacheSpec for VfsCache {
    type Data = Index;
    const FILE: &'static str = "vfs.bin";
    const VERSION: u32 = 2;
}

pub(super) fn save(index: &Index, hash: u64) {
    cache::write::<VfsCache>(hash, index);
}

pub(super) fn load() -> Option<(u64, Index)> {
    cache::read::<VfsCache>()
}
