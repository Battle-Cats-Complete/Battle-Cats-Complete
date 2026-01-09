use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use zip::ZipArchive;
use zip::write::FileOptions;

// [FIX] Only import crypto keys if we are in Dev mode.
#[cfg(feature = "dev")]
use std::sync::atomic::{AtomicI32, Ordering};
#[cfg(feature = "dev")]
use std::sync::Arc; 
#[cfg(feature = "dev")]
use std::collections::HashMap; 
#[cfg(feature = "dev")]
use std::io::{Seek, SeekFrom};
#[cfg(feature = "dev")]
use rayon::prelude::*; 
#[cfg(feature = "dev")]
use crate::dev::keys as crypto;
#[cfg(feature = "dev")]
use crate::core::patterns; 

// --- PUBLIC MODE IMPORT (Simple Copy) ---
#[cfg(not(feature = "dev"))]
pub fn import_all_from_folder(folder_path: &str, _region: &str, tx: Sender<String>) -> Result<String, String> {
    let input_path = Path::new(folder_path);
    let output_dir = Path::new("game/raw");
    
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    }

    let _ = tx.send("Scanning folder...".to_string());
    let mut file_count = 0;

    if input_path.is_dir() {
        let mut stack = vec![input_path.to_path_buf()];
        while let Some(current_dir) = stack.pop() {
            let entries = fs::read_dir(&current_dir).map_err(|e| e.to_string())?;
            for entry in entries {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    if let Some(name) = path.file_name() {
                        let dest_path = output_dir.join(name);
                        if fs::copy(&path, &dest_path).is_ok() {
                            file_count += 1;
                            if file_count % 50 == 0 {
                                let _ = tx.send(format!("Copied {} files...", file_count));
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(format!("Success! Copied {} files.", file_count))
}

#[cfg(not(feature = "dev"))]
pub fn import_all_from_zip(zip_path: &str, _region: &str, tx: Sender<String>) -> Result<String, String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let output_dir = Path::new("game/raw");

    if !output_dir.exists() { fs::create_dir_all(output_dir).map_err(|e| e.to_string())?; }

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        if file.is_dir() { continue; }
        
        // Flatten structure: Ignore folders inside zip, put everything in raw
        let name = Path::new(file.name()).file_name().unwrap_or_default().to_string_lossy().to_string();
        let path = output_dir.join(name);
        
        if let Ok(mut outfile) = fs::File::create(&path) {
            let _ = std::io::copy(&mut file, &mut outfile);
        }
        
        if i % 50 == 0 {
            let _ = tx.send(format!("Extracted {} files...", i));
        }
    }
    Ok("Success! Zip extracted.".to_string())
}

// --- DEV MODE IMPORT (Decryption & Extraction) ---

#[cfg(feature = "dev")]
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
    
    let _ = tx.send(format!("Found {} tasks. Starting...", tasks.len()));

    if tasks.is_empty() {
        return Err("No .list or .apk files found in selected folder.".to_string());
    }

    let count = AtomicI32::new(0);
    let region_ref = region_code.to_string();

    tasks.par_iter().for_each(|file_path| {
        process_task_file(file_path, output_dir, &count, &tx, &index_arc, &region_ref);
    });
    
    // Clean up sensitive region files if any leaked
    for &sensitive_file in patterns::REGION_SENSITIVE_FILES {
        let target_path = output_dir.join(sensitive_file);
        if target_path.exists() {
            let _ = fs::remove_file(target_path);
        }
    }
    Ok(format!("Success! Processed {} files.", count.load(Ordering::Relaxed)))
}

#[cfg(feature = "dev")]
pub fn import_all_from_zip(zip_path: &str, region: &str, tx: Sender<String>) -> Result<String, String> {
    // For dev mode, zip import is less common, but we can reuse the logic
    // or just assume zip contains packs. For now, redirect to the public logic 
    // but wrapped in an Ok for compatibility.
    // If you need specific zip decryption, it would go here.
    let _ = region; // unused suppress
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let output_dir = Path::new("game/raw");
    if !output_dir.exists() { fs::create_dir_all(output_dir).map_err(|e| e.to_string())?; }

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        if file.is_dir() { continue; }
        let name = Path::new(file.name()).file_name().unwrap().to_string_lossy().to_string();
        let path = output_dir.join(name);
        if let Ok(mut outfile) = fs::File::create(&path) {
            let _ = std::io::copy(&mut file, &mut outfile);
        }
    }
    Ok("Success".to_string())
}

// --- SHARED EXPORT LOGIC ---

pub fn create_game_zip(tx: Sender<String>, compression_level: i32) -> Result<(), String> {
    let src_dir = Path::new("game");
    let exports_dir = Path::new("exports");
    let zip_path = exports_dir.join("game.zip");

    if !src_dir.exists() { return Err("No 'game' folder found.".to_string()); }
    if !exports_dir.exists() { fs::create_dir_all(exports_dir).map_err(|e| e.to_string())?; }

    let _ = tx.send("Zipping files...".to_string());
    let file = fs::File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(compression_level)) 
        .unix_permissions(0o755);

    let mut files = Vec::new();
    let mut stack = vec![src_dir.to_path_buf()];

    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.to_string_lossy().contains("game/raw") || path.to_string_lossy().contains("game\\raw") { continue; }
                if path.is_dir() { stack.push(path); } else { files.push(path); }
            }
        }
    }

    for (i, path) in files.iter().enumerate() {
        let name = path.strip_prefix(src_dir).unwrap().to_string_lossy().replace("\\", "/");
        let _ = zip.start_file(name, options);
        if let Ok(mut f) = fs::File::open(path) {
            let mut buffer = Vec::new();
            let _ = f.read_to_end(&mut buffer);
            let _ = zip.write_all(&buffer);
        }
        if i % 100 == 0 { let _ = tx.send(format!("Zipped {} files...", i)); }
    }

    let _ = zip.finish();
    let _ = tx.send(format!("Success! Saved to {}", zip_path.display()));
    Ok(())
}

// --- DEV HELPERS (Only compiled if dev feature is on) ---

#[cfg(feature = "dev")]
fn build_file_index(root_dir: &Path) -> HashMap<String, Vec<PathBuf>> {
    let mut index = HashMap::new();
    let _ = scan_for_index(root_dir, &mut index);
    index
}

#[cfg(feature = "dev")]
fn scan_for_index(dir: &Path, index: &mut HashMap<String, Vec<PathBuf>>) -> std::io::Result<()> {
    if !dir.is_dir() { return Ok(()); }
    for entry in fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let _ = scan_for_index(&path, index);
        } else if let Some(name) = path.file_name() {
            let key = name.to_string_lossy().to_lowercase();
            index.entry(key).or_insert_with(Vec::new).push(path);
        }
    }
    Ok(())
}

#[cfg(feature = "dev")]
fn find_game_files(current_dir: &Path, task_list: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !current_dir.is_dir() { return Ok(()); }
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

#[cfg(feature = "dev")]
fn process_task_file(
    file_path: &Path,
    output_dir: &Path,
    count: &AtomicI32,
    tx: &Sender<String>,
    index: &Arc<HashMap<String, Vec<PathBuf>>>,
    region: &str
) {
    let ext = file_path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
    let safe_filename = file_path.file_name().unwrap_or_default().to_string_lossy();

    if ext == "apk" || ext == "xapk" {
        if let Err(e) = process_apk(file_path, output_dir, count, tx.clone(), index, region) {
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
                        count, 
                        tx.clone(), 
                        index,
                        region
                    ) {
                        let _ = tx.send(format!("Error: {}: {}", safe_filename, e));
                    }
                }
            }
        }
    }
}

#[cfg(feature = "dev")]
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
        let file_name = {
            let file = archive.by_index(i).unwrap();
            file.name().to_string()
        };

        if file_name.ends_with(".list") {
             handle_list_file(i, &file_name, &mut archive, output_dir, count, &tx, file_index, selected_region_code)?;
        } 
        else if file_name.ends_with(".obb") || file_name.ends_with(".apk") {
             handle_nested_apk(i, &file_name, &mut archive, output_dir, count, &tx, file_index, selected_region_code)?;
        }
    }
    Ok(())
}

#[cfg(feature = "dev")]
fn handle_list_file(
    index: usize,
    filename: &str,
    archive: &mut ZipArchive<fs::File>,
    output_dir: &Path,
    count: &AtomicI32,
    tx: &Sender<String>,
    file_index: &Arc<HashMap<String, Vec<PathBuf>>>,
    region: &str
) -> Result<(), String> {
    let mut list_buf = Vec::new();
    archive.by_index(index).unwrap().read_to_end(&mut list_buf).unwrap();

    let list_str = match decrypt_list_file(&list_buf) {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let pack_name = filename.replace(".list", ".pack");
    let safe_pack_name = Path::new(&pack_name).file_name().unwrap_or_default().to_string_lossy();
    let temp_pack_name = format!("temp_{}_{}", count.load(Ordering::Relaxed), safe_pack_name);
    let temp_pack_path = output_dir.join(&temp_pack_name);

    let mut pack_found = false;
    if let Ok(mut pack_file) = archive.by_name(&pack_name) {
        if let Ok(mut temp_f) = fs::File::create(&temp_pack_path) {
            if std::io::copy(&mut pack_file, &mut temp_f).is_ok() {
                pack_found = true;
            }
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
            region
        );
        let _ = fs::remove_file(temp_pack_path);
    }
    Ok(())
}

#[cfg(feature = "dev")]
fn handle_nested_apk(
    index: usize,
    filename: &str,
    archive: &mut ZipArchive<fs::File>,
    output_dir: &Path,
    count: &AtomicI32,
    tx: &Sender<String>,
    file_index: &Arc<HashMap<String, Vec<PathBuf>>>,
    region: &str
) -> Result<(), String> {
    let safe_name = Path::new(filename).file_name().unwrap_or_default().to_string_lossy();
    let temp_name = format!("nested_{}_{}", count.load(Ordering::Relaxed), safe_name);
    let temp_path = output_dir.join(&temp_name);

    let mut extracted = false;
    if let Ok(mut nested_file) = archive.by_index(index) {
        if let Ok(mut temp_f) = fs::File::create(&temp_path) {
            if std::io::copy(&mut nested_file, &mut temp_f).is_ok() {
                extracted = true;
            }
        }
    }

    if extracted {
        let _ = process_apk(&temp_path, output_dir, count, tx.clone(), file_index, region);
        let _ = fs::remove_file(temp_path);
    }
    Ok(())
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
    let current_pack_code = determine_pack_code(&pack_filename, selected_region_code);

    for line in list_content.lines() {
        process_pack_line(
            line, 
            &mut pack_file, 
            output_dir, 
            &count, 
            &tx, 
            file_index, 
            &current_pack_code
        );
    }
    Ok(())
}

#[cfg(feature = "dev")]
fn determine_pack_code(pack_filename: &str, selected_region: &str) -> String {
    if selected_region != "en" {
        return selected_region.to_string();
    }
    for code in patterns::GLOBAL_CODES {
        if *code == "en" { continue; } 
        if pack_filename.contains(&format!("_{}", code)) {
            return code.to_string();
        }
    }
    "en".to_string()
}

#[cfg(feature = "dev")]
fn process_pack_line(
    line: &str,
    pack_file: &mut fs::File,
    output_dir: &Path,
    count: &AtomicI32,
    tx: &Sender<String>,
    file_index: &HashMap<String, Vec<PathBuf>>,
    pack_code: &str
) {
    let parts: Vec<&str> = line.split(',').collect();
    if parts.len() < 3 { return; }

    let filename = parts[0];
    let offset: u64 = parts[1].trim().parse().unwrap_or(0);
    let size: usize = parts[2].trim().parse().unwrap_or(0);

    if should_skip_file(filename, size, output_dir, file_index) {
        return;
    }

    let aligned_size = if size % 16 == 0 { size } else { ((size / 16) + 1) * 16 };
    if pack_file.seek(SeekFrom::Start(offset)).is_err() { return; }

    let mut buffer = vec![0u8; aligned_size];
    if pack_file.read_exact(&mut buffer).is_err() { return; }

    if let Ok((decrypted_chunk, _)) = crypto::decrypt_pack_chunk(&buffer, filename) {
        let final_len = std::cmp::min(size, decrypted_chunk.len());
        let final_data = &decrypted_chunk[..final_len];

        let target_path = if patterns::REGION_SENSITIVE_FILES.iter().any(|&f| filename.ends_with(f)) {
            let path_obj = Path::new(filename);
            let stem = path_obj.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default();
            let ext = path_obj.extension().map(|s| s.to_string_lossy()).unwrap_or_default();
            output_dir.join(format!("{}_{}.{}", stem, pack_code, ext))
        } else {
            output_dir.join(filename)
        };

        if write_smart(&target_path, final_data, filename) {
            let c = count.fetch_add(1, Ordering::Relaxed);
            if c > 0 && c % 50 == 0 {
                let _ = tx.send(format!("Extracted {} files | Current: {}", c, filename));
            }
        }
    }
}

#[cfg(feature = "dev")]
fn should_skip_file(
    filename: &str, 
    size: usize, 
    output_dir: &Path, 
    file_index: &HashMap<String, Vec<PathBuf>>
) -> bool {
    if patterns::CHECK_LINE_FILES.contains(&filename) { return false; }
    if filename.ends_with("img015_th.imgcut") { return true; }
    if patterns::REGION_SENSITIVE_FILES.iter().any(|&f| filename.ends_with(f)) { return false; }

    let lowercase_name = filename.to_lowercase();
    if let Some(paths) = file_index.get(&lowercase_name) {
        for path in paths {
            if let Ok(meta) = fs::metadata(path) {
                if meta.len() as usize >= size.saturating_sub(16) { return true; }
            }
        }
    }

    let raw_path = output_dir.join(filename);
    if raw_path.exists() {
         if let Ok(meta) = fs::metadata(&raw_path) {
            if meta.len() as usize >= size.saturating_sub(16) { return true; }
        }
    }
    false
}

#[cfg(feature = "dev")]
fn count_lines_bytes(data: &[u8]) -> usize {
    data.iter().filter(|&&b| b == b'\n').count()
}

#[cfg(feature = "dev")]
fn write_smart(path: &Path, data: &[u8], filename: &str) -> bool {
    let new_size = data.len() as u64;
    
    if path.exists() {
        if patterns::CHECK_LINE_FILES.contains(&filename) {
            if let Ok(existing_data) = fs::read(path) {
                let existing_lines = count_lines_bytes(&existing_data);
                let new_lines = count_lines_bytes(data);
                if new_lines <= existing_lines { return false; }
            }
        } else {
            if let Ok(meta) = fs::metadata(path) {
                if meta.len() >= new_size { return false; }
            }
        }
    }

    if let Some(parent) = path.parent() {
        if !parent.exists() { let _ = fs::create_dir_all(parent); }
    }

    let temp_extension = format!("tmp_{:?}", std::thread::current().id())
        .replace("ThreadId(", "")
        .replace(")", "");
    let temp_path = path.with_extension(&temp_extension);

    if fs::write(&temp_path, data).is_err() { return false; }

    if fs::rename(&temp_path, path).is_ok() {
        return true;
    } else {
        let _ = fs::remove_file(temp_path);
    }
    false
}