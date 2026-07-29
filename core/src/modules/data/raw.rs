use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::common::job::JobEvent;

use super::engine::{audit, manifest, router, sort};

pub fn run(
    source_path_string: &str,
    emit: impl Fn(JobEvent) + Sync,
    abort_flag: &AtomicBool,
    language_priority: &[String],
) -> Result<(), String> {
    let source_path = Path::new(source_path_string);
    let game_root_path = Path::new("game");
    let raw_directory_path = game_root_path.join("raw");

    if !raw_directory_path.exists() {
        let _ = fs::create_dir_all(&raw_directory_path);
    }

    if let Ok(source_canonical) = source_path.canonicalize() {
        if let Ok(raw_canonical) = raw_directory_path.canonicalize()
            && source_canonical == raw_canonical
        {
            emit(JobEvent::Log("Organizing recognized raw data.".to_string()));
            return sort_raw_folder(&raw_directory_path, game_root_path, &emit, abort_flag);
        }

        if let Ok(game_canonical) = game_root_path.canonicalize()
            && source_canonical == game_canonical
        {
            emit(JobEvent::Log("Beginning database restructure...".to_string()));
            flatten_to_raw(game_root_path, &raw_directory_path, &emit, abort_flag)?;
            return sort_raw_folder(&raw_directory_path, game_root_path, &emit, abort_flag);
        }
    }

    emit(JobEvent::Log("Importing standard raw files...".to_string()));

    let mut raw_file_paths = Vec::new();
    collect_files_recursive(source_path, &mut raw_file_paths);

    let files_to_import = sort::process_raw_files(raw_file_paths, source_path_string, language_priority);

    if files_to_import.is_empty() {
        emit(JobEvent::Log("No files found in source directory after filtering.".to_string()));
        return Ok(());
    }

    let total = files_to_import.len();
    let update_interval = (total / 100).max(10);
    let progress_step = (total / 100).max(1);
    emit(JobEvent::Progress { current: 0, total });

    let count_tracker = AtomicUsize::new(0);

    files_to_import.par_iter().for_each(|sorted_file| {
        if abort_flag.load(Ordering::Relaxed) {
            return;
        }

        let destination_path = raw_directory_path.join(&sorted_file.resolved_name);
        let _ = fs::copy(&sorted_file.original_path, destination_path);

        let current_count = count_tracker.fetch_add(1, Ordering::Relaxed) + 1;
        if current_count.is_multiple_of(progress_step) || current_count == total {
            emit(JobEvent::Progress { current: current_count, total });
        }

        if current_count.is_multiple_of(update_interval) {
            emit(JobEvent::Log(format!("Copied {} files to raw...", current_count)));
        }
    });

    sort_raw_folder(&raw_directory_path, game_root_path, &emit, abort_flag)
}

fn sort_raw_folder(
    raw_directory: &Path,
    game_root_path: &Path,
    emit: &(dyn Fn(JobEvent) + Sync),
    abort_flag: &AtomicBool,
) -> Result<(), String> {
    let mut all_discovered_files = Vec::new();
    collect_files_recursive(raw_directory, &mut all_discovered_files);

    if all_discovered_files.is_empty() {
        emit(JobEvent::Log("Raw folder is empty.".to_string()));
        return Ok(());
    }

    let asset_router = router::AssetRouter::new(game_root_path).map_err(|e| e.to_string())?;

    let file_manifest_path = game_root_path.join("meta").join("file.json");
    let mut global_file_ledger: HashMap<String, manifest::ManifestEntry> = manifest::load(&file_manifest_path);

    let total = all_discovered_files.len();
    let update_interval = (total / 100).max(10);
    let progress_step = (total / 100).max(1);
    emit(JobEvent::Progress { current: 0, total });

    let extracted_count = AtomicUsize::new(0);

    let updated_manifest_entries: Vec<(String, manifest::ManifestEntry)> = all_discovered_files
        .into_par_iter()
        .filter_map(|file_path: PathBuf| {
            if abort_flag.load(Ordering::Relaxed) {
                return None;
            }

            let filename_os = file_path.file_name()?;
            let filename_string = filename_os.to_string_lossy().to_string();

            let target_destination_path = asset_router.resolve_destination(&filename_string, &filename_string);

            if file_path == target_destination_path {
                return None;
            }

            let Ok(file_data) = fs::read(&file_path) else {
                return None;
            };

            let true_calculated_weight = audit::calculate_true_weight(&file_data, &filename_string);
            let clean_file_data = audit::strip_carriage_returns(&file_data, &filename_string);

            if let Some(parent_directory) = target_destination_path.parent() {
                let _ = fs::create_dir_all(parent_directory);
            }

            let _ = fs::write(&target_destination_path, &clean_file_data);
            let _ = fs::remove_file(&file_path);

            let current_count = extracted_count.fetch_add(1, Ordering::Relaxed) + 1;
            if current_count.is_multiple_of(progress_step) || current_count == total {
                emit(JobEvent::Progress { current: current_count, total });
            }

            if current_count.is_multiple_of(update_interval) {
                emit(JobEvent::Log(format!(
                    "Sorted {} files | Current: {}",
                    current_count, filename_string
                )));
            }

            let manifest_entry = manifest::ManifestEntry {
                winner: "Unknown".to_string(),
                weight: true_calculated_weight,
                size: clean_file_data.len(),
                encrypted: file_data.len(),
                checksum: manifest::hash(&clean_file_data),
            };

            Some((filename_string, manifest_entry))
        })
        .collect();

    for (filename_key, entry_data) in updated_manifest_entries {
        global_file_ledger.insert(filename_key, entry_data);
    }

    manifest::save(&file_manifest_path, &global_file_ledger);

    emit(JobEvent::Log("Raw files successfully structured.".to_string()));
    Ok(())
}

fn flatten_to_raw(
    game_root_path: &Path,
    raw_directory: &Path,
    emit: &(dyn Fn(JobEvent) + Sync),
    abort_flag: &AtomicBool,
) -> Result<(), String> {
    let mut all_files = Vec::new();
    let meta_directories = ["raw", "app", "meta"];

    if let Ok(directory_entries) = fs::read_dir(game_root_path) {
        for entry in directory_entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let directory_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                if !meta_directories.contains(&directory_name.as_str()) {
                    collect_files_recursive(&path, &mut all_files);
                }
            }
        }
    }

    if all_files.is_empty() {
        emit(JobEvent::Log("No valid files to flatten.".to_string()));
        return Ok(());
    }

    emit(JobEvent::Log(format!(
        "Flattening {} files to raw directory...",
        all_files.len()
    )));

    let total = all_files.len();
    let update_interval = (total / 100).max(10);
    let progress_step = (total / 100).max(1);
    emit(JobEvent::Progress { current: 0, total });

    let count_tracker = AtomicUsize::new(0);

    all_files.par_iter().for_each(|path| {
        if abort_flag.load(Ordering::Relaxed) {
            return;
        }

        if let Some(file_name) = path.file_name() {
            let destination_path = raw_directory.join(file_name);

            let source_length = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            let destination_length = fs::metadata(&destination_path).map(|m| m.len()).unwrap_or(0);

            if !destination_path.exists() || source_length != destination_length {
                if fs::rename(path, &destination_path).is_err() {
                    let _ = fs::copy(path, &destination_path);
                    let _ = fs::remove_file(path);
                }
            } else {
                let _ = fs::remove_file(path);
            }
        }

        let current_count = count_tracker.fetch_add(1, Ordering::Relaxed) + 1;
        if current_count.is_multiple_of(progress_step) || current_count == total {
            emit(JobEvent::Progress { current: current_count, total });
        }

        if current_count.is_multiple_of(update_interval) {
            let safe_name = path.file_name().unwrap_or_default().to_string_lossy();
            emit(JobEvent::Log(format!(
                "Moved {} files to raw | Current: {}",
                current_count, safe_name
            )));
        }
    });

    if let Ok(directory_entries) = fs::read_dir(game_root_path) {
        for entry in directory_entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let directory_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                if !meta_directories.contains(&directory_name.as_str()) {
                    remove_empty_directories(&path);
                }
            }
        }
    }

    emit(JobEvent::Log("Flattening complete.".to_string()));
    Ok(())
}

fn collect_files_recursive(directory: &Path, list: &mut Vec<PathBuf>) {
    if let Ok(directory_entries) = fs::read_dir(directory) {
        for entry in directory_entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, list);
            } else {
                list.push(path);
            }
        }
    }
}

fn remove_empty_directories(directory: &Path) {
    if !directory.is_dir() {
        return;
    }

    if let Ok(directory_entries) = fs::read_dir(directory) {
        for entry in directory_entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                remove_empty_directories(&path);
            }
        }
    }
    let _ = fs::remove_dir(directory);
}
