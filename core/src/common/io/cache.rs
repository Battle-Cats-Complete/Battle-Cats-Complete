use std::fs;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;
use std::path::PathBuf;

use rayon::prelude::*;
use rustc_hash::FxHasher;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;

use crate::modules::settings::ScannerConfig;

use crate::common::dirs;
use crate::modules::data::architecture;

fn hash_directory_parallel(directory_path: &Path) -> u64 {
    if !directory_path.exists() {
        tracing::trace!("Directory {:?} does not exist, skipping hash", directory_path);
        return 0;
    }

    let Ok(read_directory) = fs::read_dir(directory_path) else {
        tracing::warn!("Failed to read directory for hashing: {:?}", directory_path);
        return 0;
    };

    let mut file_entries = Vec::new();
    for directory_entry in read_directory.flatten() {
        file_entries.push(directory_entry.path());
    }

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

fn hash_game_data() -> u64 {
    let root = Path::new(architecture::GAME);

    let Ok(entries) = fs::read_dir(root) else {
        tracing::trace!("Game directory {:?} does not exist, skipping hash", root);
        return 0;
    };

    let mut targets: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_none_or(|name| !architecture::TRANSIENT.contains(&name))
        })
        .collect();

    targets.sort_unstable();

    let child_hashes: Vec<u64> = targets
        .par_iter()
        .map(|path| {
            let mut hasher = FxHasher::default();

            if path.is_dir() {
                hash_directory_parallel(path).hash(&mut hasher);
            } else if let Ok(metadata) = path.metadata()
                && let Ok(modified) = metadata.modified() {
                modified.hash(&mut hasher);
            }

            hasher.finish()
        })
        .collect();

    let mut hasher = FxHasher::default();
    for child_hash in child_hashes {
        child_hash.hash(&mut hasher);
    }

    targets.len().hash(&mut hasher);
    hasher.finish()
}

#[tracing::instrument(level = "debug", skip(active_mod))]
pub(crate) fn get_game_hash(active_mod: Option<&str>) -> u64 {
    tracing::trace!("Calculating global game hash across assets and tables...");
    let mut final_game_hasher = FxHasher::default();

    hash_game_data().hash(&mut final_game_hasher);
    hash_directory_parallel(Path::new(architecture::MODS)).hash(&mut final_game_hasher);

    if let Some(mod_name) = active_mod {
        tracing::trace!("Including active mod in hash: {}", mod_name);
        mod_name.hash(&mut final_game_hasher);
    } else {
        "vanilla_base_game".hash(&mut final_game_hasher);
    }

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

    hasher.finish()
}

const SIZE_LIMIT: u64 = 1024 * 1024 * 100;

#[derive(Serialize, Deserialize)]
struct CachePayload<T> {
    schema_version: u32,
    hash: u64,
    data: T,
}

pub(crate) trait CacheSpec {
    type Data: Serialize + DeserializeOwned;
    const FILE: &'static str;
    const VERSION: u32;
}

pub(crate) fn read<C: CacheSpec>() -> Option<(u64, C::Data)> {
    load_payload(C::FILE, C::VERSION)
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
    let payload = CachePayload { schema_version: C::VERSION, hash, data };

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
fn load_payload<T: DeserializeOwned>(filename: &str, expected_version: u32) -> Option<(u64, T)> {
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
            if payload.schema_version != expected_version {
                tracing::warn!(
                    "Cache schema mismatch for {} (found v{}, expected v{}). Purging stale cache file.",
                    filename, payload.schema_version, expected_version
                );
                let _ = fs::remove_file(&cache_path);
                return None;
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
