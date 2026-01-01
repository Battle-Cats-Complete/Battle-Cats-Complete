use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use std::collections::HashMap; 
use rayon::prelude::*; 
use super::crypto;
use zip::ZipArchive;

fn build_file_index(root_dir: &Path) -> HashMap<String, PathBuf> {
    let mut index = HashMap::new();
    let _ = scan_for_index(root_dir, &mut index);
    index
}

fn scan_for_index(dir: &Path, index: &mut HashMap<String, PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                scan_for_index(&path, index)?;
            } else {
                if let Some(name) = path.file_name() {
                    index.insert(name.to_string_lossy().to_string(), path);
                }
            }
        }
    }
    Ok(())
}

fn decrypt_list_file(data: &[u8]) -> Result<String, String> {
    let pack_key = crypto::get_md5_key("pack");
    if let Ok(bytes) = crypto::decrypt_ecb_with_key(data, &pack_key) {
        if let Ok(text) = String::from_utf8(bytes) { return Ok(text); }
    }
    let bc_key = crypto::get_md5_key("battlecats");
    if let Ok(bytes) = crypto::decrypt_ecb_with_key(data, &bc_key) {
        if let Ok(text) = String::from_utf8(bytes) { return Ok(text); }
    }
    Err("Failed to decrypt list file: Unknown key.".to_string())
}

fn extract_pack_contents(
    list_content: &str, 
    pack_path: &Path, 
    output_dir: &Path,
    count: &AtomicI32,
    tx: Sender<String>,
    file_index: &Arc<HashMap<String, PathBuf>>,
    region_found: &AtomicBool,
    shared_region: &Arc<RwLock<Option<String>>> 
) -> Result<(), String> {
    
    let mut unused_global_codes = vec!["de", "en", "es", "fr", "it", "th"];
    let mut pending_global_img015: Option<Vec<u8>> = None;

    let mut pack_file = fs::File::open(pack_path)
        .map_err(|e| format!("Failed to open pack: {}", e))?;

    let pack_filename = pack_path.file_name().unwrap().to_string_lossy().to_string();
    
    for line in list_content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 { continue; }

        let filename = parts[0];
        let offset: u64 = parts[1].parse().unwrap_or(0);
        let size: usize = parts[2].parse().unwrap_or(0);

        let existing_path_opt = file_index.get(filename);
        let raw_dest_path = output_dir.join(filename);
        
        let aligned_size = if size % 16 == 0 { size } else { ((size / 16) + 1) * 16 };
        if pack_file.seek(SeekFrom::Start(offset)).is_err() { continue; }

        let mut buffer = vec![0u8; aligned_size];
        if pack_file.read_exact(&mut buffer).is_err() { continue; }

        match crypto::decrypt_pack_chunk(&buffer, &pack_filename) {
            Ok((decrypted_chunk, region_code)) => {
                
                if region_code != "None" && region_code != "Server" {
                    if let Ok(mut lock) = shared_region.write() {
                        if lock.is_none() {
                            *lock = Some(region_code.clone());
                        }
                    }

                    if !region_found.load(Ordering::Relaxed) {
                        region_found.store(true, Ordering::Relaxed);
                        let display_name = match region_code.as_str() {
                            "EN" => "Global",
                            "JP" => "Japan",
                            "TW" => "Taiwan",
                            "KR" => "Korean",
                            _ => "Unknown",
                        };
                        let _ = tx.send(format!("REGION:{}", display_name));
                    }
                }

                let final_len = std::cmp::min(size, decrypted_chunk.len());
                let final_data = &decrypted_chunk[..final_len];
                let new_size = final_data.len();

                if filename.ends_with("img015.png") {
                    match region_code.as_str() {
                        "JP" => { let _ = fs::write(output_dir.join("img015_ja.png"), final_data); },
                        "TW" => { let _ = fs::write(output_dir.join("img015_tw.png"), final_data); },
                        "KR" => { let _ = fs::write(output_dir.join("img015_ko.png"), final_data); },
                        "EN" => { pending_global_img015 = Some(final_data.to_vec()); },
                        _ => {}
                    }
                }

                if let Some(data) = &pending_global_img015 {
                    let current_codes = unused_global_codes.clone();
                    for code in current_codes {
                        let marker = format!("_{}.", code); 
                        if filename.contains(&marker) {
                            let save_name = format!("img015_{}.png", code);
                            let _ = fs::write(output_dir.join(save_name), data);
                            unused_global_codes.retain(|x| x != &code);
                            break;
                        }
                    }
                }

                if filename == "img015.imgcut" {
                    let mut attempts = 0;
                    let suffix = loop {
                        if let Ok(lock) = shared_region.read() {
                            if let Some(code) = &*lock {
                                break match code.as_str() {
                                    "JP" => "ja",
                                    "TW" => "tw",
                                    "KR" => "ko",
                                    "EN" => "en",
                                    _ => "en",
                                };
                            }
                        }
                        
                        attempts += 1;
                        if attempts > 30 { 
                            break "en"; 
                        }
                        thread::sleep(Duration::from_millis(100));
                    };

                    let new_name = format!("img015_{}.imgcut", suffix);
                    let _ = fs::write(output_dir.join(new_name), final_data);
                }

                let mut perform_write = true;
                let mut comparison_target: Option<PathBuf> = None;

                if let Some(p) = existing_path_opt {
                    comparison_target = Some(p.clone());
                } else if raw_dest_path.exists() {
                    comparison_target = Some(raw_dest_path.clone());
                }

                if let Some(target_path) = comparison_target {
                    if let Ok(meta) = fs::metadata(&target_path) {
                        let old_size = meta.len() as usize;
                        if old_size > 2048 && new_size < 2048 { perform_write = false; }
                        else {
                            if let Ok(old_data) = fs::read(&target_path) {
                                if old_data == final_data { perform_write = false; }
                            }
                        }
                    }
                }

                if perform_write {
                    if let Some(parent) = raw_dest_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if let Err(e) = fs::write(&raw_dest_path, final_data) {
                        println!("Error writing {}: {}", filename, e);
                    } else {
                        let current = count.fetch_add(1, Ordering::Relaxed);
                        if current % 50 == 0 {
                            let _ = tx.send(format!("Extracted {} files | Current: {}", current, filename));
                        }
                    }
                }
            },
            Err(_) => {}
        }
    }
    Ok(())
}

fn process_apk(
    apk_path: &Path, 
    output_dir: &Path, 
    count: &AtomicI32, 
    tx: Sender<String>,
    file_index: &Arc<HashMap<String, PathBuf>>,
    region_found: &AtomicBool,
    shared_region: &Arc<RwLock<Option<String>>>
) -> Result<(), String> {
    let _ = tx.send("Processing APK found in folder...".to_string());
    
    let file = fs::File::open(apk_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Bad APK: {}", e))?;

    let len = archive.len();
    for i in 0..len {
        let filename_string;
        {
            let file = archive.by_index(i).unwrap();
            filename_string = file.name().to_string();
        } 

        if filename_string.ends_with(".list") {
            let mut list_buf = Vec::new();
            {
                let mut file_reader = archive.by_index(i).unwrap(); 
                file_reader.read_to_end(&mut list_buf).unwrap();
            } 
            
            if let Ok(list_str) = decrypt_list_file(&list_buf) {
                let pack_name = filename_string.replace(".list", ".pack");
                let mut pack_found = false;
                
                let safe_pack_name = Path::new(&pack_name).file_name().unwrap_or_default().to_string_lossy();
                let temp_pack_name = format!("temp_{}_{}", count.load(Ordering::Relaxed), safe_pack_name);
                let temp_pack_path = output_dir.join(&temp_pack_name);
                
                {
                    if let Ok(mut pack_file) = archive.by_name(&pack_name) {
                        pack_found = true;
                        if let Ok(mut temp_f) = fs::File::create(&temp_pack_path) {
                            let _ = std::io::copy(&mut pack_file, &mut temp_f);
                        } else { pack_found = false; }
                    }
                } 

                if pack_found {
                    let _ = extract_pack_contents(
                        &list_str, 
                        &temp_pack_path, 
                        output_dir, 
                        count, 
                        tx.clone(), 
                        file_index,
                        region_found,
                        shared_region
                    );
                    let _ = fs::remove_file(temp_pack_path);
                }
            }
        }
    }
    Ok(())
}

fn find_game_files(current_dir: &Path, task_list: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if current_dir.is_dir() {
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                find_game_files(&path, task_list)?;
            } else if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "list" || ext_str == "apk" {
                    task_list.push(path);
                }
            }
        }
    }
    Ok(())
}

pub fn import_all_from_folder(folder_path: &str, tx: Sender<String>) -> Result<String, String> {
    let input_path = Path::new(folder_path);
    let output_dir = Path::new("game/raw");
    let game_root = Path::new("game"); 
    
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(|e| format!("Could not create 'game/raw': {}", e))?;
    }

    let _ = tx.send("Building file index (Checking existing files)...".to_string());
    
    let index_map = build_file_index(game_root);
    let index_arc = Arc::new(index_map); 
    
    let _ = tx.send("Scanning for game packs...".to_string());

    let mut tasks = Vec::new();
    find_game_files(input_path, &mut tasks).map_err(|e| e.to_string())?;
    
    let total_tasks = tasks.len();
    let _ = tx.send(format!("Found {} tasks. Starting Smart Extract...", total_tasks));

    let count = AtomicI32::new(0);
    let region_found = Arc::new(AtomicBool::new(false));
    
    let shared_region = Arc::new(RwLock::new(None));

    tasks.par_iter().for_each(|file_path| {
        let ext = file_path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
        let index_ref = Arc::clone(&index_arc);
        let region_ref = Arc::clone(&region_found);
        let shared_region_ref = Arc::clone(&shared_region);

        let safe_filename = file_path.file_name().unwrap_or_default().to_string_lossy();

        if ext == "apk" {
            if let Err(e) = process_apk(file_path, output_dir, &count, tx.clone(), &index_ref, &region_ref, &shared_region_ref) {
                let _ = tx.send(format!("Error processing APK {}: {}", safe_filename, e));
            }
        } else if ext == "list" {
            let pack_path = file_path.with_extension("pack");
            if pack_path.exists() {
                if let Ok(list_data) = fs::read(file_path) {
                    if let Ok(list_content) = decrypt_list_file(&list_data) {
                        if let Err(e) = extract_pack_contents(
                            &list_content, 
                            &pack_path, 
                            output_dir, 
                            &count, 
                            tx.clone(),
                            &index_ref,
                            &region_ref,
                            &shared_region_ref
                        ) {
                            let _ = tx.send(format!("Error in pack {}: {}", safe_filename, e));
                        }
                    }
                }
            }
        }
    });

    Ok(format!("Success! Processed {} files.", count.load(Ordering::Relaxed)))
}