use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc; 
use std::collections::HashMap; 
use rayon::prelude::*; 
use bc_crypto as crypto;
use zip::ZipArchive;
use zip::write::FileOptions;
use crate::patterns;

fn build_file_index(root_dir: &Path) -> HashMap<String, Vec<PathBuf>> {
    let mut index = HashMap::new();
    let _ = scan_for_index(root_dir, &mut index);
    index
}

fn scan_for_index(dir: &Path, index: &mut HashMap<String, Vec<PathBuf>>) -> std::io::Result<()> {
    if dir.is_dir() {
        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_dir() {
                            let _ = scan_for_index(&path, index);
                        } else {
                            if let Some(name) = path.file_name() {
                                let key = name.to_string_lossy().to_lowercase();
                                index.entry(key).or_insert_with(Vec::new).push(path);
                            }
                        }
                    }
                }
            }
            Err(_) => { }
        }
    }
    Ok(())
}
#[cfg(feature = "dev")]
fn decrypt_list_file(data: &[u8]) -> Result<String, String> {
    let pack_key = crypto::get_md5_key("pack");
    if let Ok(bytes) = crypto::decrypt_ecb_with_key(data, &pack_key) {
        if let Ok(text) = String::from_utf8(bytes) { return Ok(text); }
    }
    let bc_key = crypto::get_md5_key("battlecats");
    if let Ok(bytes) = crypto::decrypt_ecb_with_key(data, &bc_key) {
        if let Ok(text) = String::from_utf8(bytes) { return Ok(text); }
    }
    Err("Failed to decrypt list file.".to_string())
}

fn write_if_bigger(path: &Path, data: &[u8]) -> bool {
    let new_size = data.len() as u64;
    
    if path.exists() {
        if let Ok(meta) = fs::metadata(path) {
            if meta.len() >= new_size { return false; }
        }
    }

    if let Some(parent) = path.parent() {
        if !parent.exists() { let _ = fs::create_dir_all(parent); }
    }

    let temp_extension = format!("tmp_{:?}", std::thread::current().id())
        .replace("ThreadId(", "")
        .replace(")", "");
        
    let temp_path = path.with_extension(&temp_extension);

    if let Ok(_) = fs::write(&temp_path, data) {
        if let Ok(_) = fs::rename(&temp_path, path) {
            return true;
        } else {
            let _ = fs::remove_file(temp_path);
        }
    }
    
    false
}
#[cfg(feature = "dev")]
fn extract_pack_contents(
    list_content: &str, 
    pack_path: &Path, 
    output_dir: &Path,
    count: &AtomicI32,
    tx: Sender<String>,
    file_index: &Arc<HashMap<String, Vec<PathBuf>>>, 
    selected_region_code: &str 
) -> Result<(), String> {
    
    let mut pack_file = fs::File::open(pack_path)
        .map_err(|e| format!("Failed to open pack: {}", e))?;

    let pack_filename = pack_path.file_name().unwrap().to_string_lossy().to_string();

    let current_pack_code = if selected_region_code == "en" {
        let mut found_code = "en".to_string(); 
        for code in patterns::GLOBAL_CODES {
            if *code == "en" { continue; } 
            let marker = format!("_{}", code);
            if pack_filename.contains(&marker) {
                found_code = code.to_string();
                break;
            }
        }
        found_code
    } else {
        selected_region_code.to_string()
    };

    for line in list_content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 { continue; }

        let filename = parts[0];
        let offset: u64 = parts[1].trim().parse().unwrap_or(0);
        let size: usize = parts[2].trim().parse().unwrap_or(0);

        let lowercase_name = filename.to_lowercase();
        let existing_paths_opt = file_index.get(&lowercase_name);
        
        let is_sensitive = patterns::REGION_SENSITIVE_FILES.iter().any(|&f| filename.ends_with(f));

        if !is_sensitive {
            let mut found_match = false;

            if let Some(paths) = existing_paths_opt {
                for path in paths {
                    if let Ok(meta) = fs::metadata(path) {
                        if meta.len() as usize >= size.saturating_sub(16) {
                            found_match = true;
                            break;
                        }
                    }
                }
            }
            
            if !found_match {
                let raw_path = output_dir.join(filename);
                if raw_path.exists() {
                     if let Ok(meta) = fs::metadata(&raw_path) {
                        if meta.len() as usize >= size.saturating_sub(16) {
                            found_match = true;
                        }
                    }
                }
            }

            if found_match { continue; }
        }

        if filename.ends_with("img015_th.imgcut") { continue; }

        let aligned_size = if size % 16 == 0 { size } else { ((size / 16) + 1) * 16 };
        if pack_file.seek(SeekFrom::Start(offset)).is_err() { continue; }

        let mut buffer = vec![0u8; aligned_size];
        if pack_file.read_exact(&mut buffer).is_err() { continue; }

        match crypto::decrypt_pack_chunk(&buffer, &filename) {
            Ok((decrypted_chunk, _region_found)) => {
                let final_len = std::cmp::min(size, decrypted_chunk.len());
                let final_data = &decrypted_chunk[..final_len];

                if is_sensitive {
                    let path_obj = Path::new(filename);
                    let stem = path_obj.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default();
                    let ext = path_obj.extension().map(|s| s.to_string_lossy()).unwrap_or_default();
                    
                    let new_name = format!("{}_{}.{}", stem, current_pack_code, ext);

                    if write_if_bigger(&output_dir.join(new_name), final_data) {
                        count.fetch_add(1, Ordering::Relaxed);
                    }
                } 
                else {
                    if write_if_bigger(&output_dir.join(filename), final_data) {
                        let filecount = count.fetch_add(1, Ordering::Relaxed);
                        if filecount % 50 == 0 {
                            let _ = tx.send(format!("Extracted {} files | Current: {}", filecount, filename));
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
    file_index: &Arc<HashMap<String, Vec<PathBuf>>>, 
    selected_region_code: &str
) -> Result<(), String> {
    let filename_display = apk_path.file_name().unwrap_or_default().to_string_lossy();
    let _ = tx.send(format!("Processing Archive: {}...", filename_display));
    
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
                        selected_region_code
                    );
                    let _ = fs::remove_file(temp_pack_path);
                }
            }
        }
        else if filename_string.ends_with(".obb") || filename_string.ends_with(".apk") {
            let safe_nested_name = Path::new(&filename_string).file_name().unwrap_or_default().to_string_lossy();
            let temp_nested_name = format!("nested_{}_{}", count.load(Ordering::Relaxed), safe_nested_name);
            let temp_nested_path = output_dir.join(&temp_nested_name);

            let mut extracted = false;
            {
                if let Ok(mut nested_file) = archive.by_index(i) {
                    if let Ok(mut temp_f) = fs::File::create(&temp_nested_path) {
                        if std::io::copy(&mut nested_file, &mut temp_f).is_ok() {
                            extracted = true;
                        }
                    }
                }
            }

            if extracted {
                let _ = process_apk(
                    &temp_nested_path, 
                    output_dir, 
                    count, 
                    tx.clone(), 
                    file_index, 
                    selected_region_code
                );
                let _ = fs::remove_file(temp_nested_path);
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
                if ext_str == "list" || ext_str == "apk" || ext_str == "xapk" {
                    task_list.push(path);
                }
            }
        }
    }
    Ok(())
}

pub fn import_all_from_folder(folder_path: &str, region_code: &str, tx: Sender<String>) -> Result<String, String> {
    let input_path = Path::new(folder_path);
    let output_dir = Path::new("game/raw");
    let game_root = Path::new("game"); 
    
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(|e| format!("Could not create 'game/raw': {}", e))?;
    }

    let _ = tx.send("Checking existing files...".to_string());
    let index_map = build_file_index(game_root);
    let index_arc = Arc::new(index_map); 
    
    let _ = tx.send("Scanning for packs...".to_string());
    let mut tasks = Vec::new();
    find_game_files(input_path, &mut tasks).map_err(|e| e.to_string())?;
    
    let total_tasks = tasks.len();
    let _ = tx.send(format!("Found {} tasks. Starting...", total_tasks));

    let count = AtomicI32::new(0);
    let region_ref = region_code.to_string();

    tasks.par_iter().for_each(|file_path| {
        let ext = file_path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
        let index_ref = Arc::clone(&index_arc);
        let safe_filename = file_path.file_name().unwrap_or_default().to_string_lossy();

        if ext == "apk" || ext == "xapk" {
            if let Err(e) = process_apk(file_path, output_dir, &count, tx.clone(), &index_ref, &region_ref) {
                let _ = tx.send(format!("Error: {}: {}", safe_filename, e));
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
                            &region_ref
                        ) {
                            let _ = tx.send(format!("Error: {}: {}", safe_filename, e));
                        }
                    }
                }
            }
        }
    });

    Ok(format!("Success! Processed {} files.", count.load(Ordering::Relaxed)))
}

pub fn create_game_zip(tx: Sender<String>, compression_level: i32) -> Result<(), String> {
    let src_dir = Path::new("game");
    let exports_dir = Path::new("exports");
    let zip_path = exports_dir.join("game.zip");

    if !src_dir.exists() {
        return Err("No 'game' folder found to zip.".to_string());
    }

    if !exports_dir.exists() {
        fs::create_dir_all(exports_dir).map_err(|e| e.to_string())?;
    }

    let _ = tx.send(format!("Creating game.zip with Compression Level {}...", compression_level));

    let file = fs::File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(compression_level)) 
        .unix_permissions(0o755);

    let mut files_to_zip = Vec::new();
    let mut folders_to_visit = vec![src_dir.to_path_buf()];

    while let Some(current_dir) = folders_to_visit.pop() {
        let entries = fs::read_dir(&current_dir).map_err(|e| e.to_string())?;
        
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            
            if path.to_string_lossy().contains(&format!("game{}raw", std::path::MAIN_SEPARATOR)) {
                continue;
            }

            if path.is_dir() {
                folders_to_visit.push(path);
            } else {
                files_to_zip.push(path);
            }
        }
    }

    let total_files = files_to_zip.len();
    for (i, path) in files_to_zip.iter().enumerate() {
        let name = path.to_string_lossy().replace("\\", "/");
        
        if i % 50 == 0 || i == total_files - 1 {
            let _ = tx.send(format!("Zipped {} files | Current: {}", i + 1, name));
        }

        zip.start_file(name, options).map_err(|e| e.to_string())?;
        let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
        zip.write_all(&buffer).map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;
    
    let _ = tx.send(format!("Success! Saved to {}", zip_path.display()));
    Ok(())
}