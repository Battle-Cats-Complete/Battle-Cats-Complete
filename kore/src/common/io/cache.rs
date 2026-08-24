use std::fs;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use rayon::prelude::*;
use rustc_hash::FxHasher;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;

use crate::common::architecture;
use crate::common::dirs;
use crate::domains::settings::ScannerConfig;

fn hash_directory_parallel(directory_path: &Path) -> u64 {
    if !directory_path.exists() {
        tracing::trace!("Directory {:?} does not exist, skipping hash", directory_path);
        return 0;
    }

    let Ok(read_directory) = fs::read_dir(directory_path) else {
        tracing::warn!("Failed to read directory for hashing: {:?}", directory_path);
        return 0;
    };

    let mut file_entries: Vec<PathBuf> = read_directory.flatten().map(|entry| entry.path()).collect();
    file_entries.sort_unstable();

    let child_hashes: Vec<u64> = file_entries.par_iter().map(|child_path| {
        let mut local_hasher = FxHasher::default();

        if child_path.is_dir() {
            let subdirectory_hash = hash_directory_parallel(child_path);
            subdirectory_hash.hash(&mut local_hasher);
        } else if let Ok(file_metadata) = child_path.metadata()
            && let Ok(modified_time) = file_metadata.modified() {
            modified_time.hash(&mut local_hasher);
        }

        local_hasher.finish()
    }).collect();

    let mut final_hasher = FxHasher::default();
    for child_hash in child_hashes {
        child_hash.hash(&mut final_hasher);
    }

    file_entries.len().hash(&mut final_hasher);
    final_hasher.finish()
}

#[tracing::instrument(level = "debug", skip(active_mod))]
pub(crate) fn get_game_hash(active_mod: Option<&str>) -> u64 {
    tracing::trace!("Calculating global game hash across assets and tables...");
    let mut final_game_hasher = FxHasher::default();

    hash_directory_parallel(Path::new(architecture::GAME)).hash(&mut final_game_hasher);
    hash_directory_parallel(Path::new(architecture::MODS)).hash(&mut final_game_hasher);

    if let Some(mod_name) = active_mod {
        tracing::trace!("Including active mod in hash: {}", mod_name);
        mod_name.hash(&mut final_game_hasher);
    } else {
        "vanilla_base_game".hash(&mut final_game_hasher);
    }

    BUILD_STAMP.hash(&mut final_game_hasher);
    force_token().hash(&mut final_game_hasher);

    let hash_result = final_game_hasher.finish();
    tracing::debug!("Generated game hash: {}", hash_result);
    hash_result
}

pub(crate) fn content_hash(config: &ScannerConfig) -> u64 {
    content_key(get_game_hash(config.active_mod.as_deref()), config)
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
