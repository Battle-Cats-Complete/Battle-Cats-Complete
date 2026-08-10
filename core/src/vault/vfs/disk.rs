use crate::common::io::cache;

use super::Index;

pub(super) struct VfsCache;

impl cache::CacheSpec for VfsCache {
    type Data = Index;
    const FILE: &'static str = "virtual_file_system.bin";
    const VERSION: u32 = 3;
}

pub(super) fn encode(index: &Index, hash: u64) -> Option<Vec<u8>> {
    cache::encode::<VfsCache>(hash, index)
}

pub(super) fn store(bytes: &[u8]) {
    cache::store::<VfsCache>(bytes);
}

pub(super) fn load() -> Option<(u64, Index)> {
    cache::read::<VfsCache>()
}
