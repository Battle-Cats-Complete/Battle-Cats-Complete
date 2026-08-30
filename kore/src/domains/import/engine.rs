pub mod apk;
pub(crate) mod audit;
pub(crate) mod hardcoded;
pub mod keys;
pub(crate) mod manifest;
pub(crate) mod router;
pub mod rules;
pub mod sort;

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};

use nyanko::common::Region;
use nyanko::pack::chronology;
use nyanko::pack::cryptology;
use rayon::prelude::*;
use tracing::warn;

use crate::common::architecture;
use crate::common::io;
use crate::common::job::{JobEvent, ProgressCounter};
use crate::domains::settings::ImportStructure;
use crate::domains::settings::RuleHandling;
use crate::domains::settings::UserKeys;

#[derive(Clone)]
struct UniversalTask {
    pub pack_path: PathBuf,
    pub original_name: String,
    pub final_name: String,
    pub byte_offset: u64,
    pub byte_size: usize,
    pub region_code: String,
    pub chrono_score: u64,
    pub is_loose: bool,
}

impl UniversalTask {
    fn pack_name(&self) -> String {
        if self.is_loose {
            return manifest::LOOSE.to_string();
        }

        self.pack_path.file_name().unwrap_or_default().to_string_lossy().into_owned()
    }
}

struct DecryptedCandidate {
    pub task: UniversalTask,
    pub clean_data: Vec<u8>,
}

fn determine_region_code(filename: &str, folder_region: &str) -> String {
    if folder_region != "en" {
        return folder_region.to_string();
    }

    for &(language_code, _) in io::APP_LANGUAGES {
        if language_code == "en" {
            continue;
        }
        let language_suffix = format!("_{}", language_code);
        if filename.contains(&language_suffix) {
            return language_code.to_string();
        }
    }
    "en".to_string()
}

fn get_region_priority(region_code: &str) -> u8 {
    match region_code {
        "ja" => 4,
        "en" => 3,
        "tw" => 2,
        "ko" => 1,
        _ => 0,
    }
}

fn absorb_pack_hashes(ledger: &mut manifest::Ledger, hashes: HashMap<String, HashMap<String, u64>>) {
    for (pack_name, region_map) in hashes {
        for (region_key, checksum) in region_map {
            ledger.track(pack_name.clone(), region_key, checksum);
        }
    }
}

fn cleanup_temporary_directories(directories: &[PathBuf]) {
    for directory in directories {
        let _ = fs::remove_dir_all(directory);
    }
}

pub(crate) fn run_universal_import(
    source_directories: &[PathBuf],
    structure: ImportStructure,
    emit: &(dyn Fn(JobEvent) + Sync),
    abort_flag: &AtomicBool,
    progress: &ProgressCounter,
) -> Result<(), String> {
    let user_keys = UserKeys::load();
    if user_keys.is_empty() {
        return Err("Missing Decryption Keys".into());
    }

    let owned_tuples = user_keys.as_tuples();
    let reference_tuples: Vec<(Region, &str, &str)> = owned_tuples
        .iter()
        .map(|(key_string, iv, region_enum)| (*region_enum, key_string.as_str(), iv.as_str()))
        .collect();

    let nyanko_keys = cryptology::Keys::parse(&reference_tuples).map_err(|error| error.to_string())?;

    let game_root_path = Path::new(architecture::GAME);
    let mut ledger = manifest::Ledger::load();

    let asset_router_utility = router::AssetRouter::new(game_root_path, structure).map_err(|error| error.to_string())?;
    let (compiled_regex_set, compiled_exception_rules) = rules::compile();

    emit(JobEvent::Log("Collecting game data...".to_string()));

    let mut universal_task_map: HashMap<String, Vec<UniversalTask>> = HashMap::new();
    let mut global_temporary_directories: Vec<PathBuf> = Vec::new();
    let mut current_pack_hashes: HashMap<String, HashMap<String, u64>> = HashMap::new();

    let mut has_notified_extraction = false;

    for source_directory in source_directories {
        if abort_flag.load(Ordering::Relaxed) {
            cleanup_temporary_directories(&global_temporary_directories);
            return Err("Job Aborted".into());
        }

        let mut folder_region_name = source_directory.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        if folder_region_name == "files"
            && let Some(parent_directory) = source_directory.parent() {
            folder_region_name = parent_directory.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        }

        let current_region_code = match folder_region_name.as_str() {
            s if s.ends_with("tw") => "tw",
            s if s.ends_with("kr") || s.ends_with("ko") => "ko",
            s if s.ends_with("en") => "en",
            s if s.ends_with("battlecats") => "ja",
            _ => "en",
        };

        let mut discovered_list_files = Vec::new();
        let mut discovered_apk_files = Vec::new();
        let mut discovered_loose_files = Vec::new();

        let _ = apk::find_files(
            source_directory,
            &mut discovered_list_files,
            &mut discovered_apk_files,
            &mut discovered_loose_files,
        );

        if !discovered_apk_files.is_empty() && !has_notified_extraction {
            emit(JobEvent::Log("Extracting update data...".to_string()));
            has_notified_extraction = true;
        }

        let (mut new_list_paths, mut new_temporary_dirs, mut new_loose_paths) = apk::extract_all(&discovered_apk_files);

        discovered_list_files.append(&mut new_list_paths);
        global_temporary_directories.append(&mut new_temporary_dirs);
        discovered_loose_files.append(&mut new_loose_paths);

        for loose_path in discovered_loose_files {
            let filename = loose_path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let byte_size = fs::metadata(&loose_path).map(|m| m.len() as usize).unwrap_or(0);
            let file_chrono_score = chronology::calculate_weight(&loose_path, &global_temporary_directories);

            let matched_user_rule = compiled_regex_set
                .matches(&filename)
                .into_iter()
                .next()
                .map(|index| &compiled_exception_rules[index]);

            if matched_user_rule.is_some_and(|rule| rule.handling == RuleHandling::Ignore) {
                continue;
            }

            let mut final_resolved_filename = filename.clone();

            if let Some(rule) = matched_user_rule
                && rule.languages.values().any(|&is_active| is_active) {
                let asset_path_object = Path::new(&filename);

                if let Some(asset_stem) = asset_path_object.file_stem() {
                    let asset_stem_string = asset_stem.to_string_lossy();
                    let asset_extension_string = asset_path_object.extension().unwrap_or_default().to_string_lossy();

                    let mut cleaned_stem = asset_stem_string.to_string();
                    for &(code, _) in io::APP_LANGUAGES {
                        let suffix = format!("_{}", code);
                        if cleaned_stem.ends_with(&suffix) {
                            cleaned_stem = cleaned_stem.trim_end_matches(&suffix).to_string();
                            break;
                        }
                    }

                    let is_region_enabled = rule.languages.get(current_region_code).copied().unwrap_or(false);
                    if rule.handling == RuleHandling::Only && !is_region_enabled {
                        continue;
                    }

                    let is_single = rule.handling == RuleHandling::Only
                        && rule.languages.values().filter(|&&is_active| is_active).count() == 1;

                    if is_region_enabled {
                        if is_single {
                            final_resolved_filename = if asset_extension_string.is_empty() {
                                cleaned_stem
                            } else {
                                format!("{}.{}", cleaned_stem, asset_extension_string)
                            };
                        } else if !current_region_code.is_empty() {
                            final_resolved_filename = if asset_extension_string.is_empty() {
                                format!("{}_{}", cleaned_stem, current_region_code)
                            } else {
                                format!("{}_{}.{}", cleaned_stem, current_region_code, asset_extension_string)
                            };
                        }
                    }
                }
            }

            let extraction_task = UniversalTask {
                pack_path: loose_path,
                original_name: filename,
                final_name: final_resolved_filename.clone(),
                byte_offset: 0,
                byte_size,
                region_code: current_region_code.to_string(),
                chrono_score: file_chrono_score,
                is_loose: true,
            };

            universal_task_map.entry(final_resolved_filename).or_default().push(extraction_task);
        }

        for item_path in discovered_list_files {
            let corresponding_pack_path = item_path.with_extension("pack");
            if !corresponding_pack_path.exists() {
                continue;
            }

            let pack_filename = corresponding_pack_path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let final_region_code = determine_region_code(&pack_filename, current_region_code);
            let file_chrono_score = chronology::calculate_weight(&corresponding_pack_path, &global_temporary_directories);
            let pack_region_map = current_pack_hashes.entry(pack_filename.clone()).or_default();

            if !pack_region_map.contains_key(&final_region_code)
                && let Ok(pack_hash_value) = manifest::hash_file(&corresponding_pack_path) {
                pack_region_map.insert(final_region_code.clone(), pack_hash_value);
            }

            let Ok(list_file_data) = fs::read(&item_path) else {
                continue;
            };

            let Some(decoded_string_content) = cryptology::decrypt_list(&list_file_data).ok() else {
                continue;
            };

            for entry in cryptology::PackEntry::parse(&decoded_string_content) {
                let raw_asset_name = entry.name.as_str();

                let matched_user_rule = compiled_regex_set
                    .matches(raw_asset_name)
                    .into_iter()
                    .next()
                    .map(|index| &compiled_exception_rules[index]);

                if matched_user_rule.is_some_and(|rule| rule.handling == RuleHandling::Ignore) {
                    continue;
                }

                let mut final_resolved_filename = raw_asset_name.to_string();

                if let Some(rule) = matched_user_rule
                    && rule.languages.values().any(|&is_active| is_active) {
                    let asset_path_object = Path::new(raw_asset_name);

                    if let Some(asset_stem) = asset_path_object.file_stem() {
                        let asset_stem_string = asset_stem.to_string_lossy();
                        let asset_extension_string = asset_path_object.extension().unwrap_or_default().to_string_lossy();

                        let mut cleaned_stem = asset_stem_string.to_string();
                        for &(code, _) in io::APP_LANGUAGES {
                            let suffix = format!("_{}", code);
                            if cleaned_stem.ends_with(&suffix) {
                                cleaned_stem = cleaned_stem.trim_end_matches(&suffix).to_string();
                                break;
                            }
                        }

                        let is_region_enabled = rule.languages.get(final_region_code.as_str()).copied().unwrap_or(false);
                        if rule.handling == RuleHandling::Only && !is_region_enabled {
                            continue;
                        }

                        let is_single = rule.handling == RuleHandling::Only
                            && rule.languages.values().filter(|&&is_active| is_active).count() == 1;

                        if is_region_enabled {
                            if is_single {
                                final_resolved_filename = if asset_extension_string.is_empty() {
                                    cleaned_stem
                                } else {
                                    format!("{}.{}", cleaned_stem, asset_extension_string)
                                };
                            } else if !final_region_code.is_empty() {
                                final_resolved_filename = if asset_extension_string.is_empty() {
                                    format!("{}_{}", cleaned_stem, final_region_code)
                                } else {
                                    format!("{}_{}.{}", cleaned_stem, final_region_code, asset_extension_string)
                                };
                            }
                        }
                    }
                }

                let extraction_task = UniversalTask {
                    pack_path: corresponding_pack_path.clone(),
                    original_name: raw_asset_name.to_string(),
                    final_name: final_resolved_filename.clone(),
                    byte_offset: entry.offset,
                    byte_size: entry.size,
                    region_code: final_region_code.clone(),
                    chrono_score: file_chrono_score,
                    is_loose: false,
                };

                universal_task_map.entry(final_resolved_filename).or_default().push(extraction_task);
            }
        }
    }

    let hardcoded_rules = hardcoded::generate_rules();
    let mut final_extraction_queue: Vec<(String, Vec<UniversalTask>, PathBuf)> = Vec::new();

    for (resolved_filename, duplicate_tasks) in universal_task_map {
        let matched_rule = duplicate_tasks.first().and_then(|task| hardcoded_rules.get(&task.original_name).copied());

        let mut tasks_by_region: HashMap<String, Vec<UniversalTask>> = HashMap::new();
        for processing_task in duplicate_tasks {
            tasks_by_region.entry(processing_task.region_code.clone()).or_default().push(processing_task);
        }

        let mut regional_winners_to_decrypt: Vec<UniversalTask> = Vec::new();
        for (_, mut regional_tasks) in tasks_by_region {
            if let Some(rule) = matched_rule {
                match rule {
                    hardcoded::HardcodedType::Oldest => {
                        regional_tasks.sort_by_key(|task| std::cmp::Reverse(task.chrono_score));
                    }
                }
            } else {
                regional_tasks.sort_by_key(|task| task.chrono_score);
            }

            if let Some(winning_task) = regional_tasks.pop() {
                regional_winners_to_decrypt.push(winning_task);
            }
        }

        let Some(representative_candidate) = regional_winners_to_decrypt.first() else {
            continue;
        };

        let target_destination_path = asset_router_utility.resolve_destination(&representative_candidate.original_name, &resolved_filename);

        let mut requires_memory_decryption = false;

        if let Some(existing_placement) = ledger.placement(&resolved_filename) {
            for candidate in &regional_winners_to_decrypt {
                if candidate.is_loose {
                    if candidate.byte_size > existing_placement.record.encrypted {
                        requires_memory_decryption = true;
                        break;
                    }
                } else {
                    let pack_filename = candidate.pack_name();

                    let newly_calculated_hash = current_pack_hashes
                        .get(&pack_filename)
                        .and_then(|region_map| region_map.get(&candidate.region_code))
                        .copied();

                    let saved_manifest_hash = ledger.pack_checksum(&pack_filename, &candidate.region_code);

                    if newly_calculated_hash.is_none() || newly_calculated_hash != saved_manifest_hash {
                        requires_memory_decryption = true;
                        break;
                    }
                }
            }

            if !requires_memory_decryption && !target_destination_path.exists() {
                requires_memory_decryption = true;
            }
        } else {
            requires_memory_decryption = true;
        }

        if requires_memory_decryption {
            final_extraction_queue.push((resolved_filename, regional_winners_to_decrypt, target_destination_path));
        }
    }

    if final_extraction_queue.is_empty() {
        emit(JobEvent::Log("Workspace is completely up to date.".to_string()));

        absorb_pack_hashes(&mut ledger, current_pack_hashes);
        ledger.save();

        cleanup_temporary_directories(&global_temporary_directories);
        return Ok(());
    }

    let total = final_extraction_queue.len();
    emit(JobEvent::Progress { current: 0, total });
    progress.reset(total);

    let progress_step = (total / 100).max(1);
    let advance_progress = || {
        let current = progress.advance();
        if current.is_multiple_of(progress_step) || current == total {
            emit(JobEvent::Progress { current, total });
        }
    };

    let successfully_extracted_count = AtomicI32::new(0);
    let failed_decryption_count = AtomicUsize::new(0);
    let console_update_interval = (total / 100).max(10);

    emit(JobEvent::Log(format!("Comparing and organizing {} game files...", final_extraction_queue.len())));

    let updated_placements: Vec<(String, manifest::Placement)> = final_extraction_queue
        .into_par_iter()
        .filter_map(|(resolved_filename, regional_tasks_to_decrypt, target_destination_path)| {
            if abort_flag.load(Ordering::Relaxed) {
                return None;
            }

            let mut decrypted_candidates: Vec<DecryptedCandidate> = Vec::new();

            for processing_task in regional_tasks_to_decrypt {
                if processing_task.is_loose {
                    match fs::read(&processing_task.pack_path) {
                        Ok(raw_data) => decrypted_candidates.push(DecryptedCandidate {
                            task: processing_task.clone(),
                            clean_data: raw_data,
                        }),
                        Err(error) => {
                            failed_decryption_count.fetch_add(1, Ordering::Relaxed);
                            warn!("Could not read loose file {}: {}", processing_task.pack_path.display(), error);
                        }
                    }
                    continue;
                }

                let pack_display = processing_task.pack_path.display();

                let Ok(mut input_pack_file) = fs::File::open(&processing_task.pack_path) else {
                    failed_decryption_count.fetch_add(1, Ordering::Relaxed);
                    warn!("Could not open {} while extracting {}", pack_display, processing_task.final_name);
                    continue;
                };

                let memory_aligned_size = processing_task.byte_size.div_ceil(16) * 16;

                let bytes_remaining = input_pack_file
                    .metadata()
                    .map_or(memory_aligned_size, |data| data.len().saturating_sub(processing_task.byte_offset) as usize);

                if bytes_remaining == 0 && processing_task.byte_size > 0 {
                    failed_decryption_count.fetch_add(1, Ordering::Relaxed);
                    warn!("{} starts past the end of {}", processing_task.final_name, pack_display);
                    continue;
                }

                let mut encrypted_byte_buffer = vec![0u8; memory_aligned_size.min(bytes_remaining)];

                if input_pack_file.seek(SeekFrom::Start(processing_task.byte_offset)).is_err() {
                    failed_decryption_count.fetch_add(1, Ordering::Relaxed);
                    warn!("Could not seek to {} in {}", processing_task.byte_offset, pack_display);
                    continue;
                }

                if let Err(error) = input_pack_file.read_exact(&mut encrypted_byte_buffer) {
                    failed_decryption_count.fetch_add(1, Ordering::Relaxed);
                    warn!("Could not read {} from {}: {}", processing_task.final_name, pack_display, error);
                    continue;
                }

                let (decrypted_byte_vector, _) = cryptology::decrypt_chunk(&encrypted_byte_buffer, &processing_task.original_name, &nyanko_keys);

                let strict_size_limit = std::cmp::min(processing_task.byte_size, decrypted_byte_vector.len());
                let exact_data_slice = &decrypted_byte_vector[..strict_size_limit];

                let cleaned_data_vector = audit::strip_carriage_returns(exact_data_slice, &processing_task.final_name);

                decrypted_candidates.push(DecryptedCandidate {
                    task: processing_task,
                    clean_data: cleaned_data_vector,
                });
            }

            if decrypted_candidates.is_empty() {
                warn!("No readable source for {}, it will be retried on the next import", resolved_filename);
                advance_progress();
                return None;
            }

            decrypted_candidates.sort_by(|candidate_a, candidate_b| {
                let weight_cmp = candidate_a.clean_data.len().cmp(&candidate_b.clean_data.len());
                if weight_cmp == std::cmp::Ordering::Equal {
                    get_region_priority(&candidate_a.task.region_code).cmp(&get_region_priority(&candidate_b.task.region_code))
                } else {
                    weight_cmp
                }
            });

            let Some(winning_candidate) = decrypted_candidates.pop() else {
                advance_progress();
                return None;
            };

            let winning_checksum = manifest::hash(&winning_candidate.clean_data);
            let winning_size = winning_candidate.clean_data.len();
            let mut already_on_disk = false;

            if let Some(existing_placement) = ledger.placement(&resolved_filename) {
                let is_same_region = winning_candidate.task.region_code == existing_placement.record.winner;

                if !is_same_region && winning_size < existing_placement.record.size {
                    advance_progress();
                    return None;
                }

                already_on_disk = winning_size == existing_placement.record.size
                    && winning_checksum == existing_placement.record.checksum
                    && target_destination_path.exists();
            }

            if !already_on_disk {
                if let Some(parent_directory) = target_destination_path.parent() {
                    let _ = fs::create_dir_all(parent_directory);
                }
                if let Err(error) = fs::write(&target_destination_path, &winning_candidate.clean_data) {
                    failed_decryption_count.fetch_add(1, Ordering::Relaxed);
                    warn!("Could not write {}: {}", target_destination_path.display(), error);
                    advance_progress();
                    return None;
                }

                let current_extracted_total = successfully_extracted_count.fetch_add(1, Ordering::Relaxed) + 1;
                if (current_extracted_total as usize).is_multiple_of(console_update_interval) {
                    emit(JobEvent::Log(format!("Processed {} files | Routing: {}", current_extracted_total, resolved_filename)));
                }
            }

            advance_progress();

            let winning_pack = winning_candidate.task.pack_name();

            Some((
                resolved_filename,
                manifest::Placement {
                    pack: winning_pack,
                    record: manifest::FileRecord {
                        winner: winning_candidate.task.region_code,
                        size: winning_size,
                        encrypted: winning_candidate.task.byte_size,
                        checksum: winning_checksum,
                    },
                },
            ))
        })
        .collect();

    if abort_flag.load(Ordering::Relaxed) {
        cleanup_temporary_directories(&global_temporary_directories);
        return Err("Job Aborted".into());
    }

    let final_errors = failed_decryption_count.load(Ordering::Relaxed);
    if final_errors > 0 {
        emit(JobEvent::Log(format!("Encountered {} errors reading pack chunks. See the log for details.", final_errors)));
    }

    for (filename_key, placement) in updated_placements {
        ledger.place(filename_key, placement);
    }

    absorb_pack_hashes(&mut ledger, current_pack_hashes);
    ledger.save();

    cleanup_temporary_directories(&global_temporary_directories);

    emit(JobEvent::Log("Files successfully organized and updated.".to_string()));
    Ok(())
}