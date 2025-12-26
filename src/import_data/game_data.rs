use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc; 
use std::collections::HashMap; // Needed for the Index
use rayon::prelude::*; 
use super::crypto;
use zip::ZipArchive;

// --- HELPER: Build File Index (The "Map") ---
// Scans the existing game folder so we know where files are (even if sorted)
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

// --- HELPER: Extract Loose Pack (With Smart Merging) ---
fn extract_pack_contents(
    list_content: &str, 
    pack_path: &Path, 
    output_dir: &Path,
    count: &AtomicI32,
    tx: Sender<String>,
    file_index: &Arc<HashMap<String, PathBuf>>
) -> Result<(), String> {
    
    let mut pack_file = fs::File::open(pack_path)
        .map_err(|e| format!("Failed to open pack: {}", e))?;

    let pack_filename = pack_path.file_name().unwrap().to_string_lossy().to_string();
    
    for line in list_content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 { continue; }

        let filename = parts[0];
        let offset: u64 = parts[1].parse().unwrap_or(0);
        let size: usize = parts[2].parse().unwrap_or(0);

        // --- WARNING FIX 1: removed 'mut' (not needed) ---
        let existing_path_opt = file_index.get(filename);
        
        // --- WARNING FIX 2: We now USE this variable! ---
        // This is the path where the file MIGHT be if another thread just wrote it.
        let raw_dest_path = output_dir.join(filename);
        
        // --- WARNING FIX 3: Removed unused 'should_extract' ---
        
        let aligned_size = if size % 16 == 0 { size } else { ((size / 16) + 1) * 16 };
        if pack_file.seek(SeekFrom::Start(offset)).is_err() { continue; }

        let mut buffer = vec![0u8; aligned_size];
        if pack_file.read_exact(&mut buffer).is_err() { continue; }

        match crypto::decrypt_pack_chunk(&buffer, &pack_filename) {
            Ok((decrypted_chunk, _)) => {
                let final_len = std::cmp::min(size, decrypted_chunk.len());
                let final_data = &decrypted_chunk[..final_len];
                let new_size = final_data.len();

                // --- SMART OVERWRITE LOGIC ---
                let mut perform_write = true;

                // Step 1: Identify "Old Data" Source
                // Priority A: The Sorted Index (Historical data)
                // Priority B: The Raw Folder (Live data from this run)
                let mut comparison_target: Option<PathBuf> = None;

                if let Some(p) = existing_path_opt {
                    comparison_target = Some(p.clone());
                } else if raw_dest_path.exists() {
                    comparison_target = Some(raw_dest_path.clone());
                }

                // Step 2: Perform the Comparison
                if let Some(target_path) = comparison_target {
                    // Check METADATA first (Super fast)
                    if let Ok(meta) = fs::metadata(&target_path) {
                        let old_size = meta.len() as usize;

                        // Rule A: Placeholder Protection (Size Heuristic)
                        // If Old is Big (>2KB) and New is Tiny (<2KB), assume New is junk.
                        // We skip WITHOUT even reading the file bytes. Fast!
                        if old_size > 2048 && new_size < 2048 {
                            perform_write = false;
                        }
                        // Rule B: Identity Check (Only if sizes match-ish)
                        // We only pay the cost of reading the disk if we are unsure.
                        else {
                            if let Ok(old_data) = fs::read(&target_path) {
                                if old_data == final_data {
                                    perform_write = false; // Exact duplicate
                                }
                            }
                        }
                    }
                }

                if perform_write {
                    // Note: We use raw_dest_path here, effectively using the variable we defined earlier
                    if let Some(parent) = raw_dest_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }

                    if let Err(e) = fs::write(&raw_dest_path, final_data) {
                        println!("Error writing {}: {}", filename, e);
                    } else {
                        // LOGGING UPDATE: Batch log + Filename sample
                        let current = count.fetch_add(1, Ordering::Relaxed);
                        if current % 50 == 0 {
                            let _ = tx.send(format!("Extracted {} files | Current: {}", filename, current));
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
    file_index: &Arc<HashMap<String, PathBuf>>
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
                let temp_pack_name = format!("temp_{}_{}", count.load(Ordering::Relaxed), Path::new(&pack_name).file_name().unwrap().to_string_lossy());
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
                    let _ = extract_pack_contents(&list_str, &temp_pack_path, output_dir, count, tx.clone(), file_index);
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
    let game_root = Path::new("game"); // The root to check for duplicates
    
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(|e| format!("Could not create 'game/raw': {}", e))?;
    }

    let _ = tx.send("Building file index (Checking existing files)...".to_string());
    
    // 1. Build Index (The Smart Part)
    let index_map = build_file_index(game_root);
    let index_arc = Arc::new(index_map); // Share it across threads
    
    let _ = tx.send("Scanning for game packs...".to_string());

    // 2. Gather Work
    let mut tasks = Vec::new();
    find_game_files(input_path, &mut tasks).map_err(|e| e.to_string())?;
    
    let total_tasks = tasks.len();
    let _ = tx.send(format!("Found {} tasks. Starting Smart Extract...", total_tasks));

    let count = AtomicI32::new(0);

    // 3. Execute Parallel
    tasks.par_iter().for_each(|file_path| {
        let ext = file_path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
        // Clone the Index Reference for each thread
        let index_ref = Arc::clone(&index_arc);

        if ext == "apk" {
            if let Err(e) = process_apk(file_path, output_dir, &count, tx.clone(), &index_ref) {
                let _ = tx.send(format!("Error processing APK: {}", e));
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
                            &index_ref
                        ) {
                            let _ = tx.send(format!("Error in {:?}: {}", pack_path, e));
                        }
                    }
                }
            }
        }
    });

    Ok(format!("Success! Processed {} files.", count.load(Ordering::Relaxed)))
}