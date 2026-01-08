#[cfg(feature = "dev")]
use std::fs;
#[cfg(feature = "dev")]
use std::path::{Path, PathBuf};
#[cfg(feature = "dev")]
use std::sync::mpsc::Sender;
#[cfg(feature = "dev")]
use std::sync::atomic::{AtomicI32, Ordering};
#[cfg(feature = "dev")]
use std::sync::Arc;
#[cfg(feature = "dev")]
use std::io::{Read, Seek, SeekFrom};
#[cfg(feature = "dev")]
use rayon::prelude::*;
#[cfg(feature = "dev")]
use zip::ZipArchive;
#[cfg(feature = "dev")]
use crate::dev::keys; 
#[cfg(feature = "dev")]
use crate::core::patterns; 
#[cfg(feature = "dev")]
use crate::core::import::log::Logger;

#[cfg(feature = "dev")]
fn build_file_index(root_directory: &Path) -> std::collections::HashMap<String, Vec<PathBuf>> {
    let mut file_index = std::collections::HashMap::new();
    let _ = scan_for_index(root_directory, &mut file_index);
    file_index
}

#[cfg(feature = "dev")]
fn scan_for_index(directory: &Path, file_index: &mut std::collections::HashMap<String, Vec<PathBuf>>) -> std::io::Result<()> {
    if !directory.is_dir() { return Ok(()); }
    
    let entries = match fs::read_dir(directory) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let _ = scan_for_index(&path, file_index);
            continue;
        } 
        
        if let Some(file_name_os) = path.file_name() {
            let key = file_name_os.to_string_lossy().to_lowercase();
            file_index.entry(key).or_insert_with(Vec::new).push(path);
        }
    }
    Ok(())
}

#[cfg(feature = "dev")]
fn should_skip_file(
    file_name: &str, 
    file_size: usize, 
    output_directory: &Path, 
    file_index: &std::collections::HashMap<String, Vec<PathBuf>>
) -> bool {
    if patterns::CHECK_LINE_FILES.contains(&file_name) {
        return false;
    }
    if file_name.ends_with("img015_th.imgcut") { return true; }
    if patterns::REGION_SENSITIVE_FILES.iter().any(|&sensitive| file_name.ends_with(sensitive)) {
        return false;
    }

    let lowercase_name = file_name.to_lowercase();
    if let Some(existing_paths) = file_index.get(&lowercase_name) {
        for existing_path in existing_paths {
            if let Ok(metadata) = fs::metadata(existing_path) {
                if metadata.len() as usize >= file_size.saturating_sub(16) {
                    return true;
                }
            }
        }
    }

    let raw_output_path = output_directory.join(file_name);
    if raw_output_path.exists() {
         if let Ok(metadata) = fs::metadata(&raw_output_path) {
            if metadata.len() as usize >= file_size.saturating_sub(16) {
                return true;
            }
        }
    }

    false
}

#[cfg(feature = "dev")]
fn write_smart(target_path: &Path, file_data: &[u8], file_name: &str) -> bool {
    let new_file_size = file_data.len() as u64;
    
    if target_path.exists() {
        if patterns::CHECK_LINE_FILES.contains(&file_name) {
            if let Ok(existing_data) = fs::read(target_path) {
                let existing_line_count = existing_data.iter().filter(|&&byte| byte == b'\n').count();
                let new_line_count = file_data.iter().filter(|&&byte| byte == b'\n').count();
                if new_line_count <= existing_line_count { return false; }
            }
        } else {
            if let Ok(metadata) = fs::metadata(target_path) {
                if metadata.len() >= new_file_size { return false; }
            }
        }
    }
    if let Some(parent_dir) = target_path.parent() { 
        if !parent_dir.exists() { let _ = fs::create_dir_all(parent_dir); } 
    }
    
    let temporary_path = target_path.with_extension("tmp");
    if fs::write(&temporary_path, file_data).is_err() { return false; }
    let _ = fs::rename(temporary_path, target_path);
    true
}

fn get_region_display_name(region_code: &str) -> &str {
    match region_code {
        "en" => "Global",
        "ja" => "Japan",
        "tw" => "Taiwan",
        "ko" => "Korea",
        _ => region_code, 
    }
}

#[cfg(feature = "dev")]
pub fn run_extraction(target_folder: String, region_code: String, status_sender: Sender<String>) -> Result<(), String> {
    let logger = Logger::new(status_sender.clone());
    let input_path = Path::new(&target_folder);
    let output_directory = Path::new("game/raw");
    let game_root_directory = Path::new("game");

    if !output_directory.exists() {
        fs::create_dir_all(output_directory).map_err(|e| format!("IO Error: {}", e))?;
    }

    logger.info("Indexing existing files...");
    let file_index_map = build_file_index(game_root_directory);
    let file_index_arc = Arc::new(file_index_map);

    let region_display_name = get_region_display_name(&region_code);
    logger.info(format!("Scanning {} version for packs...", region_display_name));
    
    let mut extraction_tasks = Vec::new();
    find_game_files(input_path, &mut extraction_tasks).map_err(|e| e.to_string())?;

    logger.info(format!("Found {} tasks. Starting parallel extraction...", extraction_tasks.len()));

    let global_file_count = AtomicI32::new(0);
    let region_code_reference = region_code.to_string();

    extraction_tasks.par_iter().for_each(|task_path| {
        process_task_file(task_path, output_directory, &global_file_count, &logger, &file_index_arc, &region_code_reference);
    });

    logger.success(format!("Extraction complete. Processed {} files.", global_file_count.load(Ordering::Relaxed)));
    
    crate::core::import::sort_data::sort_files(status_sender)
}

#[cfg(feature = "dev")]
fn find_game_files(directory: &Path, task_list: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !directory.is_dir() { return Ok(()); }
    for entry_result in fs::read_dir(directory)? {
        let entry = entry_result?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            find_game_files(&entry_path, task_list)?;
        } else if let Some(extension) = entry_path.extension() {
            let extension_str = extension.to_string_lossy().to_lowercase();
            if extension_str == "list" || extension_str == "apk" || extension_str == "xapk" {
                task_list.push(entry_path);
            }
        }
    }
    Ok(())
}

#[cfg(feature = "dev")]
fn process_task_file(
    file_path: &Path, 
    output_directory: &Path, 
    global_count: &AtomicI32, 
    logger: &Logger, 
    file_index: &Arc<std::collections::HashMap<String, Vec<PathBuf>>>, 
    region_code: &str
) {
    let file_extension = file_path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
    let safe_file_name = file_path.file_stem().unwrap_or_default().to_string_lossy(); 

    if file_extension == "apk" || file_extension == "xapk" {
        logger.info(format!("Found APK: {}", file_path.file_name().unwrap_or_default().to_string_lossy()));
        if let Err(e) = process_apk(file_path, output_directory, global_count, logger, file_index, region_code) {
            logger.error(format!("Error: {}: {}", safe_file_name, e));
        }
    } else if file_extension == "list" {
        let pack_file_path = file_path.with_extension("pack");
        
        if pack_file_path.exists() {
            logger.info(format!("Found pack: {}", safe_file_name));
            
            if let Ok(file_data) = fs::read(file_path) {
                if let Ok(decrypted_list_content) = decrypt_list_file(&file_data) {
                    let pack_language_code = determine_pack_code(&safe_file_name, region_code);
                    let _ = extract_pack_contents(&decrypted_list_content, &pack_file_path, output_directory, global_count, logger, file_index, &pack_language_code);
                }
            }
        } else {
            logger.info(format!("Warning: Pack '{}' is missing its .pack file.", safe_file_name));
        }
    }
}

#[cfg(feature = "dev")]
fn process_apk(
    apk_path: &Path, 
    output_directory: &Path, 
    global_count: &AtomicI32, 
    logger: &Logger, 
    file_index: &Arc<std::collections::HashMap<String, Vec<PathBuf>>>, 
    region_code: &str
) -> Result<(), String> {
    let apk_file = fs::File::open(apk_path).map_err(|e| e.to_string())?;
    let mut zip_archive = ZipArchive::new(apk_file).map_err(|e| e.to_string())?;

    let mut list_pack_pairs = Vec::new();
    for i in 0..zip_archive.len() {
        if let Ok(file_in_zip) = zip_archive.by_index(i) {
            let file_name = file_in_zip.name().to_string();
            if file_name.ends_with(".list") {
                let pack_name = file_name.replace(".list", ".pack");
                list_pack_pairs.push((file_name, pack_name));
            }
        }
    }

    for (list_name, pack_name) in list_pack_pairs {
        let mut list_file_data = Vec::new();
        let mut read_successful = false;

        if let Ok(mut zip_list_file) = zip_archive.by_name(&list_name) {
            if zip_list_file.read_to_end(&mut list_file_data).is_ok() {
                read_successful = true;
            }
        } 

        if read_successful {
            if let Ok(decrypted_list_content) = decrypt_list_file(&list_file_data) {
                if let Ok(mut zip_pack_file) = zip_archive.by_name(&pack_name) {
                    let safe_pack_filename = Path::new(&pack_name).file_name().unwrap().to_string_lossy();
                    let temporary_pack_path = output_directory.join(format!("_temp_{}", safe_pack_filename));
                    
                    let mut temp_file_handle = fs::File::create(&temporary_pack_path).map_err(|e| e.to_string())?;
                    if std::io::copy(&mut zip_pack_file, &mut temp_file_handle).is_ok() {
                        let pack_language_code = determine_pack_code(&list_name, region_code);
                        let _ = extract_pack_contents(&decrypted_list_content, &temporary_pack_path, output_directory, global_count, logger, file_index, &pack_language_code);
                        let _ = fs::remove_file(temporary_pack_path);
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(feature = "dev")]
fn determine_pack_code(pack_filename: &str, selected_region: &str) -> String {
    if selected_region != "en" {
        return selected_region.to_string();
    }
    for region_code in patterns::GLOBAL_CODES {
        if *region_code == "en" { continue; } 
        if pack_filename.contains(&format!("_{}", region_code)) {
            return region_code.to_string();
        }
    }
    "en".to_string()
}

#[cfg(feature = "dev")]
fn decrypt_list_file(encrypted_data: &[u8]) -> Result<String, String> {
    let key_pack = keys::get_md5_key("pack");
    if let Ok(decrypted_bytes) = keys::decrypt_ecb_with_key(encrypted_data, &key_pack) {
        if let Ok(decrypted_string) = String::from_utf8(decrypted_bytes) { return Ok(decrypted_string); }
    }
    let key_battlecats = keys::get_md5_key("battlecats");
    if let Ok(decrypted_bytes) = keys::decrypt_ecb_with_key(encrypted_data, &key_battlecats) {
        if let Ok(decrypted_string) = String::from_utf8(decrypted_bytes) { return Ok(decrypted_string); }
    }
    Err("Decryption failed".into())
}

#[cfg(feature = "dev")]
fn extract_pack_contents(
    list_content_str: &str, 
    pack_file_path: &Path, 
    output_directory: &Path, 
    global_count: &AtomicI32,
    logger: &Logger, 
    file_index: &std::collections::HashMap<String, Vec<PathBuf>>, 
    pack_language_code: &str
) -> Result<(), String> {
    let mut pack_file_handle = fs::File::open(pack_file_path).map_err(|e| e.to_string())?;
    let pack_file_name = pack_file_path.file_name().unwrap_or_default().to_string_lossy().to_string();

    for line in list_content_str.lines() {
        let line_parts: Vec<&str> = line.split(',').collect();
        if line_parts.len() < 3 { continue; }
        
        let file_name_in_pack = line_parts[0];
        let file_offset: u64 = line_parts[1].parse().unwrap_or(0);
        let file_size: usize = line_parts[2].parse().unwrap_or(0);
        
        if should_skip_file(file_name_in_pack, file_size, output_directory, file_index) {
            continue;
        }

        let aligned_size = if file_size % 16 == 0 { file_size } else { ((file_size / 16) + 1) * 16 };
        
        if pack_file_handle.seek(SeekFrom::Start(file_offset)).is_err() { 
            logger.error(format!("Warning: {} is missing file {}", pack_file_name, file_name_in_pack));
            continue; 
        }

        let mut data_buffer = vec![0u8; aligned_size];
        if pack_file_handle.read_exact(&mut data_buffer).is_err() { 
            logger.error(format!("Warning: {} is missing file {}", pack_file_name, file_name_in_pack));
            continue; 
        }

        if let Ok((decrypted_data, _)) = keys::decrypt_pack_chunk(&data_buffer, file_name_in_pack) {
            let final_file_data = &decrypted_data[..std::cmp::min(file_size, decrypted_data.len())];
            
            let target_file_path = if patterns::REGION_SENSITIVE_FILES.iter().any(|&sensitive| file_name_in_pack.ends_with(sensitive)) {
                 let path_obj = Path::new(file_name_in_pack);
                 let file_stem = path_obj.file_stem().unwrap_or_default().to_string_lossy();
                 let file_extension = path_obj.extension().unwrap_or_default().to_string_lossy();
                 output_directory.join(format!("{}_{}.{}", file_stem, pack_language_code, file_extension))
            } else {
                 output_directory.join(file_name_in_pack)
            };

            if write_smart(&target_file_path, final_file_data, file_name_in_pack) {
                let current_files_processed = global_count.fetch_add(1, Ordering::Relaxed);
                if current_files_processed % 50 == 0 { 
                    logger.info(format!("Extracted {} files | Current: {}", current_files_processed, file_name_in_pack)); 
                }
            }
        }
    }
    Ok(())
}