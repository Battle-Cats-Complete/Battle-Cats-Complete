use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use bincode::Options;
use directories::BaseDirs;
use rayon::prelude::*;
use rustc_hash::FxHasher;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub fn get_cache_dir() -> Option<PathBuf> {
    let Some(base_dirs) = BaseDirs::new() else {
        tracing::error!("Failed to retrieve BaseDirs. Caching system is disabled.");
        return None;
    };

    let cache_directory = base_dirs.data_local_dir().join("battle_cats_complete").join("cache");

    if !cache_directory.exists() {
        tracing::debug!("Creating missing cache directory at {:?}", cache_directory);
        if let Err(err) = fs::create_dir_all(&cache_directory) {
            tracing::error!("Failed to create cache directory: {}", err);
            return None;
        }
    }

    Some(cache_directory)
}

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

#[tracing::instrument(level = "debug", skip(active_mod))]
pub fn get_game_hash(active_mod: Option<&str>) -> u64 {
    tracing::trace!("Calculating global game hash across assets and tables...");
    let mut final_game_hasher = FxHasher::default();
    
    let target_paths = ["game/tables", "game/cats", "game/enemies", "game/stages", "mods"];
    for path_string in target_paths {
        let directory_hash = hash_directory_parallel(Path::new(path_string));
        directory_hash.hash(&mut final_game_hasher);
    }

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

#[derive(Serialize, Deserialize)]
struct CachePayload<T> {
    hash: u64,
    data: T,
}

#[tracing::instrument(level = "debug", skip_all, fields(file = %filename))]
pub fn load_with_hash<T: DeserializeOwned>(filename: &str) -> Option<(u64, T)> {
    let Some(cache_directory) = get_cache_dir() else {
        return None;
    };

    let cache_path = cache_directory.join(filename);

    let Ok(cache_file) = File::open(&cache_path) else {
        tracing::trace!("Cache file {} does not exist or cannot be read", filename);
        return None;
    };

    let reader = BufReader::new(cache_file);

    let options = bincode::DefaultOptions::new()
        .with_limit(1024 * 1024 * 100);

    match options.deserialize_from::<_, CachePayload<T>>(reader) {
        Ok(payload) => {
            tracing::debug!("Successfully loaded cache payload for {}", filename);
            Some((payload.hash, payload.data))
        },
        Err(err) => {
            tracing::warn!("Failed to deserialize cache payload for {}: {}. Purging corrupted cache file.", filename, err);
            let _ = fs::remove_file(&cache_path);
            None
        }
    }
}

#[tracing::instrument(level = "debug", skip(data))]
pub fn save<T: Serialize>(filename: &str, hash: u64, data: &T) {
    let Some(cache_directory) = get_cache_dir() else {
        tracing::warn!("Cache directory unavailable; skipping save for {}", filename);
        return;
    };

    let target_path = cache_directory.join(filename);
    let tmp_path = target_path.with_extension("tmp");

    let Ok(cache_file) = File::create(&tmp_path) else {
        tracing::error!("Failed to create temporary cache file at {:?}", tmp_path);
        return;
    };

    let mut writer = BufWriter::new(cache_file);
    let payload = CachePayload { hash, data };
    let options = bincode::DefaultOptions::new()
        .with_limit(1024 * 1024 * 100);

    if let Err(err) = options.serialize_into(&mut writer, &payload) {
        tracing::error!("Failed to serialize cache payload: {}", err);
        let _ = fs::remove_file(&tmp_path);
        return;
    }

    if writer.into_inner().is_err() {
        tracing::error!("Failed to flush cache file writer for {:?}", tmp_path);
        return;
    }

    if let Err(err) = fs::rename(&tmp_path, &target_path) {
        tracing::error!("Failed to commit cache file rename: {}", err);
    } else {
        tracing::debug!("Successfully committed cache file {} to disk", filename);
    }
}