use std::fs;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use rustc_hash::FxHasher;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;

use crate::common::dirs;
use crate::domains::settings::ScannerConfig;

pub(crate) fn index_key(fingerprint: u64, active_mod: Option<&str>) -> u64 {
    let mut hasher = FxHasher::default();

    fingerprint.hash(&mut hasher);

    if let Some(mod_name) = active_mod {
        mod_name.hash(&mut hasher);
    } else {
        "vanilla_base_game".hash(&mut hasher);
    }

    BUILD_STAMP.hash(&mut hasher);
    force_token().hash(&mut hasher);

    hasher.finish()
}

pub(crate) fn content_hash(fingerprint: u64, config: &ScannerConfig) -> u64 {
    content_key(index_key(fingerprint, config.active_mod.as_deref()), config)
}

pub(crate) fn content_key(index: u64, config: &ScannerConfig) -> u64 {
    let mut hasher = FxHasher::default();

    index.hash(&mut hasher);
    config.hash(&mut hasher);
    BUILD_STAMP.hash(&mut hasher);
    force_token().hash(&mut hasher);

    hasher.finish()
}

const SIZE_LIMIT: u64 = 1024 * 1024 * 100;

const BUILD_STAMP: &str = env!("CORE_FINGERPRINT");

pub const FORCE_RESCAN: &str = "FORCE_RESCAN";

static FORCE_STAMP: OnceLock<Option<u128>> = OnceLock::new();

fn force_token() -> Option<u128> {
    *FORCE_STAMP.get_or_init(|| {
        std::env::var(FORCE_RESCAN)
            .is_ok_and(|value| value.trim().eq_ignore_ascii_case("true"))
            .then(|| SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos()))
            .inspect(|stamp| tracing::info!(stamp, "{} is true, forcing a rescan for this run", FORCE_RESCAN))
    })
}

pub struct Scan<T> {
    pub data: T,
    pub key: Option<u64>,
    pub payload: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
struct CachePayload<T> {
    build_stamp: String,
    hash: u64,
    data: T,
}

pub(crate) trait CacheSpec {
    type Data: Serialize + DeserializeOwned;
    const FILE: &'static str;
}

pub(crate) fn read<C: CacheSpec>() -> Option<(u64, C::Data)> {
    load_payload(C::FILE)
}

pub(crate) fn purge<C: CacheSpec>() {
    let Some(cache_directory) = dirs::cache_path() else {
        return;
    };

    let target_path = cache_directory.join(C::FILE);

    if target_path.exists() && fs::remove_file(&target_path).is_err() {
        tracing::warn!("Failed to purge stale cache file {}", C::FILE);
    }
}

pub(crate) fn write<C: CacheSpec>(hash: u64, data: &C::Data) {
    let Some(bytes) = encode::<C>(hash, data) else {
        return;
    };

    store::<C>(&bytes);
}

pub(crate) fn encode<C: CacheSpec>(hash: u64, data: &C::Data) -> Option<Vec<u8>> {
    let payload = CachePayload { build_stamp: BUILD_STAMP.to_string(), hash, data };

    postcard::to_allocvec(&payload)
        .inspect_err(|err| tracing::error!("Failed to serialize cache payload for {}: {}", C::FILE, err))
        .ok()
}

pub(crate) fn store<C: CacheSpec>(bytes: &[u8]) {
    let Some(cache_directory) = dirs::cache() else {
        tracing::warn!("Cache directory unavailable; skipping save for {}", C::FILE);
        return;
    };

    let target_path = cache_directory.join(C::FILE);
    let tmp_path = super::hidden_temp(&target_path);

    if let Err(err) = fs::write(&tmp_path, bytes) {
        tracing::error!("Failed to write temporary cache file at {:?}: {}", tmp_path, err);
        return;
    }

    if let Err(err) = fs::rename(&tmp_path, &target_path) {
        tracing::error!("Failed to promote cache file {:?}: {}", target_path, err);
        let _ = fs::remove_file(&tmp_path);
    }
}

#[tracing::instrument(level = "debug", skip_all, fields(file = %filename))]
fn load_payload<T: DeserializeOwned>(filename: &str) -> Option<(u64, T)> {
    let cache_directory = dirs::cache_path()?;

    let cache_path = cache_directory.join(filename);

    let Ok(metadata) = fs::metadata(&cache_path) else {
        tracing::trace!("Cache file {} does not exist or cannot be read", filename);
        return None;
    };

    if metadata.len() > SIZE_LIMIT {
        tracing::warn!("Cache file {} is {} bytes, past the {} byte limit. Purging oversized cache file.", filename, metadata.len(), SIZE_LIMIT);
        let _ = fs::remove_file(&cache_path);
        return None;
    }

    let Ok(bytes) = fs::read(&cache_path) else {
        tracing::trace!("Cache file {} could not be read", filename);
        return None;
    };

    match postcard::from_bytes::<CachePayload<T>>(&bytes) {
        Ok(payload) => {
            if payload.build_stamp != BUILD_STAMP {
                tracing::info!(
                    "Cache {} is from build {} (this is {}). Serving it while the rescan runs.",
                    filename, payload.build_stamp, BUILD_STAMP
                );
            }


            tracing::debug!("Successfully loaded cache payload for {}", filename);
            Some((payload.hash, payload.data))
        },
        Err(err) => {
            tracing::info!("Cache {} was written in a format this version cannot read ({}). Rebuilding it.", filename, err);
            let _ = fs::remove_file(&cache_path);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_index_fingerprint_drives_the_key_it_composes() {
        // persist_index derives this from the live index instead of walking the disk,
        // so a changed index must never compose to the key the last launch stored.
        assert_ne!(index_key(1, None), index_key(2, None));
        assert_eq!(index_key(1, None), index_key(1, None));

        // The active mod still separates keys for an otherwise identical index.
        assert_ne!(index_key(1, None), index_key(1, Some("MyMod")));
    }
}
