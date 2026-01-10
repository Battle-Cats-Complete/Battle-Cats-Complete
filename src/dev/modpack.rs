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
pub fn run(folder_path: &str, tx: Sender<String>) -> Result<(), String> {
    let source_dir = Path::new(folder_path);
    let raw_dir = Path::new("game/raw");
    let game_dir = Path::new("game");
    
    if raw_dir.exists() {
        let _ = tx.send("Mod Mode: Wiping game/raw for fresh decryption...".to_string());
        fs::remove_dir_all(raw_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(raw_dir).map_err(|e| e.to_string())?;

    let shared_index = Arc::new(std::collections::HashMap::new());

    let _ = tx.send("Scanning mod resources...".to_string());
    
    let mut found_paths = Vec::new();
    find_game_files(source_dir, &mut found_paths).map_err(|e| e.to_string())?;

    let _ = tx.send(format!("Found {} tasks. Starting...", found_paths.len()));

    let count = AtomicI32::new(0);

    let valid_paths: Vec<PathBuf> = found_paths.into_iter().filter(|p| {
        let name = p.file_name().unwrap_or_default().to_string_lossy();
        !name_has_country_code(&name)
    }).collect();

    let (priority_tasks, standard_tasks): (Vec<_>, Vec<_>) = valid_paths.into_iter().partition(|p| {
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        name.contains("downloadlocal") || name.contains("downloadextension")
    });

    if !standard_tasks.is_empty() {
        let _ = tx.send("Extracting Base Assets...".to_string());
        standard_tasks.par_iter().for_each(|path| {
            process_task(path, raw_dir, &count, &tx, &shared_index);
        });
    }

    if !priority_tasks.is_empty() {
        let _ = tx.send("Applying DownloadLocal Patch...".to_string());
        priority_tasks.par_iter().for_each(|path| {
            process_task(path, raw_dir, &count, &tx, &shared_index);
        });
    }

    let _ = tx.send("Cleaning up old game data for mod...".to_string());
    if let Ok(entries) = fs::read_dir(game_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().and_then(|n| n.to_str()) != Some("raw") {
                if path.is_dir() { let _ = fs::remove_dir_all(path); }
                else { let _ = fs::remove_file(path); }
            }
        }
    }

    let _ = tx.send(format!("Mod installation complete. Processed {} files.", count.load(Ordering::Relaxed)));
    
    Ok(())
}

#[cfg(feature = "dev")]
fn process_task(
    file_path: &Path, 
    output_dir: &Path, 
    counter: &AtomicI32, 
    tx: &Sender<String>, 
    index: &Arc<std::collections::HashMap<String, Vec<PathBuf>>>
) {
    let ext = file_path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
    
    if ext == "apk" || ext == "xapk" {
        if let Err(e) = process_apk(file_path, output_dir, counter, tx, index) {
             let _ = tx.send(format!("Error processing APK: {}", e));
        }
    } else if ext == "list" {
        let pack_path = file_path.with_extension("pack");
        if pack_path.exists() {
            if let Ok(data) = fs::read(file_path) {
                if let Ok(content) = decrypt_list_content(&data) {
                    let _ = extract_pack(&content, &pack_path, output_dir, counter, tx);
                }
            }
        }
    }
}

#[cfg(feature = "dev")]
fn extract_pack(
    content: &str, 
    pack_path: &Path, 
    output_dir: &Path, 
    counter: &AtomicI32,
    tx: &Sender<String>
) -> Result<(), String> {
    let mut file = fs::File::open(pack_path).map_err(|e| e.to_string())?;
    let pack_name = pack_path.file_name().unwrap_or_default().to_string_lossy();
    
    if name_has_country_code(&pack_name) { return Ok(()); }

    for line in content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 { continue; }
        
        let asset_name = parts[0];
        let offset: u64 = parts[1].parse().unwrap_or(0);
        let size: usize = parts[2].parse().unwrap_or(0);
        
        let aligned_size = if size % 16 == 0 { size } else { ((size / 16) + 1) * 16 };
        if file.seek(SeekFrom::Start(offset)).is_err() { continue; }
        
        let mut buffer = vec![0u8; aligned_size];
        if file.read_exact(&mut buffer).is_err() { continue; }

        if let Ok((decrypted_bytes, _)) = keys::decrypt_pack_chunk(&buffer, asset_name) {
            let final_data = &decrypted_bytes[..std::cmp::min(size, decrypted_bytes.len())];

            let target_path = output_dir.join(asset_name);

            if let Some(parent_dir) = target_path.parent() {
                if !parent_dir.exists() { let _ = fs::create_dir_all(parent_dir); }
            }
            if fs::write(&target_path, final_data).is_ok() {
                let c = counter.fetch_add(1, Ordering::Relaxed);
                if c % 50 == 0 { 
                    let _ = tx.send(format!("Decrypted {} files | Current: {}", c, asset_name)); 
                }
            }
        }
    }
    Ok(())
}

#[cfg(feature = "dev")]
fn process_apk(
    apk_path: &Path, 
    output_dir: &Path, 
    counter: &AtomicI32, 
    tx: &Sender<String>,
    _index: &Arc<std::collections::HashMap<String, Vec<PathBuf>>>
) -> Result<(), String> {
    let file = fs::File::open(apk_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

    let mut list_pack_pairs = Vec::new();
    for i in 0..archive.len() {
        if let Ok(file_in_zip) = archive.by_index(i) {
            let name = file_in_zip.name().to_string();
            if name.ends_with(".list") {
                let pack_name = name.replace(".list", ".pack");
                if name_has_country_code(&pack_name) { continue; }
                list_pack_pairs.push((name, pack_name));
            }
        }
    }

    let (priority, standard): (Vec<_>, Vec<_>) = list_pack_pairs.into_iter().partition(|(name, _)| {
        let n = name.to_lowercase();
        n.contains("downloadlocal") || n.contains("downloadextension")
    });

    let mut process_internal = |pairs: Vec<(String, String)>, stage: &str| {
        for (list_name, pack_name) in pairs {
            let mut list_content_bytes = Vec::new();
            let mut read_success = false;
            if let Ok(mut list_file) = archive.by_name(&list_name) {
                if list_file.read_to_end(&mut list_content_bytes).is_ok() { read_success = true; }
            } 
            if read_success {
                if let Ok(decrypted_content) = decrypt_list_content(&list_content_bytes) {
                    if let Ok(mut pack_file) = archive.by_name(&pack_name) {
                        let safe_filename = Path::new(&pack_name).file_name().unwrap().to_string_lossy();
                        let temp_pack_path = output_dir.join(format!("_temp_{}", safe_filename));
                        if let Ok(mut temp_file) = fs::File::create(&temp_pack_path) {
                            if std::io::copy(&mut pack_file, &mut temp_file).is_ok() {
                                let _ = tx.send(format!("APK [{}]: Extracting {}", stage, safe_filename));
                                let _ = extract_pack(&decrypted_content, &temp_pack_path, output_dir, counter, tx);
                                let _ = fs::remove_file(temp_pack_path);
                            }
                        }
                    }
                }
            }
        }
    };

    process_internal(standard, "Standard");
    process_internal(priority, "Priority");
    Ok(())
}

#[cfg(feature = "dev")]
fn decrypt_list_content(data: &[u8]) -> Result<String, String> {
    let pack_key = keys::get_md5_key("pack");
    if let Ok(bytes) = keys::decrypt_ecb_with_key(data, &pack_key) {
        if let Ok(s) = String::from_utf8(bytes) { return Ok(s); }
    }
    let bc_key = keys::get_md5_key("battlecats");
    if let Ok(bytes) = keys::decrypt_ecb_with_key(data, &bc_key) {
        if let Ok(s) = String::from_utf8(bytes) { return Ok(s); }
    }
    Err("Decryption failed".into())
}

#[cfg(feature = "dev")]
fn find_game_files(search_dir: &Path, path_list: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !search_dir.is_dir() { return Ok(()); }
    for entry_result in fs::read_dir(search_dir)?.flatten() {
        let path = entry_result.path();
        if path.is_dir() {
            find_game_files(&path, path_list)?;
        } else if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if ext_str == "list" || ext_str == "apk" || ext_str == "xapk" {
                path_list.push(path);
            }
        }
    }
    Ok(())
}

#[cfg(feature = "dev")]
fn name_has_country_code(name: &str) -> bool {
    let codes = ["_en", "_ja", "_tw", "_ko", "_es", "_de", "_fr", "_it", "_th"];
    codes.iter().any(|code| name.contains(code))
}