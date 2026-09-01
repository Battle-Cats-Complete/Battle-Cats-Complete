use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use crate::common::architecture;
use crate::common::job::{JobEvent, ProgressCounter};
use crate::domains::mining;
use crate::domains::settings::{ImportConfig, ImportStructure};

use super::engine::{audit, manifest, router, sort};

pub fn run(
    source_path_string: &str,
    import_config: ImportConfig,
    emit: impl Fn(JobEvent) + Sync,
    abort_flag: &AtomicBool,
    language_priority: &[String],
    progress: &ProgressCounter,
) -> Result<(), String> {
    let source_path = Path::new(source_path_string);
    let game_root_path = Path::new(architecture::GAME);
    let raw_directory_path = game_root_path.join("raw");

    if !raw_directory_path.exists() {
        let _ = fs::create_dir_all(&raw_directory_path);
    }

    if let Ok(source_canonical) = source_path.canonicalize() {
        if let Ok(raw_canonical) = raw_directory_path.canonicalize()
            && source_canonical == raw_canonical
        {
            emit(JobEvent::Log("Organizing recognized raw data.".to_string()));
            return sort_raw_folder(&raw_directory_path, game_root_path, import_config.structure, &emit, abort_flag, progress);
        }

        if let Ok(game_canonical) = game_root_path.canonicalize()
            && source_canonical == game_canonical
        {
            emit(JobEvent::Log("Beginning database restructure...".to_string()));
            flatten_to_raw(game_root_path, &raw_directory_path, &emit, abort_flag, progress)?;
            return sort_raw_folder(&raw_directory_path, game_root_path, import_config.structure, &emit, abort_flag, progress);
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
    progress.reset(total);

    files_to_import.par_iter().for_each(|sorted_file| {
        if abort_flag.load(Ordering::Relaxed) {
            return;
        }

        let destination_path = raw_directory_path.join(&sorted_file.resolved_name);
        let _ = fs::copy(&sorted_file.original_path, destination_path);

        let current_count = progress.advance();
        if current_count.is_multiple_of(progress_step) || current_count == total {
            emit(JobEvent::Progress { current: current_count, total });
        }

        if current_count.is_multiple_of(update_interval) {
            emit(JobEvent::Log(format!("Copied {} files to raw...", current_count)));
        }
    });

    sort_raw_folder(&raw_directory_path, game_root_path, import_config.structure, &emit, abort_flag, progress)
}

fn sort_raw_folder(
    raw_directory: &Path,
    game_root_path: &Path,
    structure: ImportStructure,
    emit: &(dyn Fn(JobEvent) + Sync),
    abort_flag: &AtomicBool,
    progress: &ProgressCounter,
) -> Result<(), String> {
    let mut all_discovered_files = Vec::new();
    collect_files_recursive(raw_directory, &mut all_discovered_files);

    if all_discovered_files.is_empty() {
        emit(JobEvent::Log("Raw folder is empty.".to_string()));
        return Ok(());
    }

    let asset_router = router::AssetRouter::new(game_root_path, structure).map_err(|e| e.to_string())?;

    let mut ledger = manifest::Ledger::load();
    let tracked = ledger.tracks_files();

    if ledger.faulted() {
        emit(JobEvent::Log("The saved manifest could not be read and was discarded.".to_string()));
        emit(JobEvent::Log("This import will rebuild the manifest from scratch.".to_string()));
    }

    let total = all_discovered_files.len();
    let update_interval = (total / 100).max(10);
    let progress_step = (total / 100).max(1);
    emit(JobEvent::Progress { current: 0, total });
    progress.reset(total);

    let updated_placements: Vec<(String, manifest::Placement, Option<mining::FileDelta>)> = all_discovered_files
        .into_par_iter()
        .filter_map(|file_path: PathBuf| {
            if abort_flag.load(Ordering::Relaxed) {
                return None;
            }

            let filename_os = file_path.file_name()?;
            let filename_string = filename_os.to_string_lossy().to_string();

            let target_destination_path = asset_router.resolve_destination(&filename_string, &filename_string);

            let settled = file_path == target_destination_path;

            let Ok(file_data) = fs::read(&file_path) else {
                return None;
            };

            let clean_file_data = audit::strip_carriage_returns(&file_data, &filename_string);
            let checksum = manifest::hash(&clean_file_data);

            let sample = (tracked && !settled && mining::mineable(&filename_string)).then(|| {
                let previous = fs::read(&target_destination_path).ok();

                mining::delta(&filename_string, manifest::NONE, manifest::NONE, previous.as_deref(), &clean_file_data)
            });

            if settled {
                if clean_file_data != file_data {
                    let _ = fs::write(&target_destination_path, &clean_file_data);
                }
            } else {
                if let Some(parent_directory) = target_destination_path.parent() {
                    let _ = fs::create_dir_all(parent_directory);
                }

                if !manifest::holds(&target_destination_path, clean_file_data.len(), checksum) {
                    let _ = fs::write(&target_destination_path, &clean_file_data);
                }

                let _ = fs::remove_file(&file_path);
            }

            let current_count = progress.advance();
            if current_count.is_multiple_of(progress_step) || current_count == total {
                emit(JobEvent::Progress { current: current_count, total });
            }

            if current_count.is_multiple_of(update_interval) {
                emit(JobEvent::Log(format!(
                    "Sorted {} files | Current: {}",
                    current_count, filename_string
                )));
            }

            let placement = manifest::Placement {
                pack: manifest::LOOSE.to_string(),
                record: manifest::FileRecord {
                    winner: manifest::NONE.to_string(),
                    size: clean_file_data.len(),
                    encrypted: file_data.len(),
                    checksum,
                },
            };

            Some((filename_string, placement, sample.flatten()))
        })
        .collect();

    let mut found = Vec::new();
    let mut touched = Vec::new();

    for (filename_key, placement, sample) in updated_placements {
        if tracked {
            let held = ledger.placement(&filename_key).map(|placed| placed.record.checksum);

            touched.extend(mining::touch(&filename_key, held, placement.record.checksum));
        }

        ledger.place(filename_key, placement);
        found.extend(sample);
    }

    if mining::commit(found, touched, Vec::new()) {
        emit(JobEvent::Log("Changes in previous database content detected.".to_string()));
        emit(JobEvent::Log("Open the Mining page to read what changed.".to_string()));
    }

    ledger.save();

    emit(JobEvent::Log("Raw files successfully structured.".to_string()));
    Ok(())
}

fn flatten_to_raw(
    game_root_path: &Path,
    raw_directory: &Path,
    emit: &(dyn Fn(JobEvent) + Sync),
    abort_flag: &AtomicBool,
    progress: &ProgressCounter,
) -> Result<(), String> {
    let mut all_files = Vec::new();

    if let Ok(directory_entries) = fs::read_dir(game_root_path) {
        for entry in directory_entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let directory_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                if directory_name != "raw" {
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
    progress.reset(total);

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

        let current_count = progress.advance();
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
                if directory_name != "raw" {
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
