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

// [DEV] Main Entry Point
#[cfg(feature = "dev")]
pub fn run_decryption(target_folder: &str, region_code: &str, tx: Sender<String>) -> Result<(), String> {
    let input_path = Path::new(target_folder);
    let output_dir = Path::new("game/raw");
    let game_root = Path::new("game");

    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;
    }

    let _ = tx.send("Indexing existing files...".to_string());
    let index_map = build_file_index(game_root);
    let index_arc = Arc::new(index_map);

    let _ = tx.send(format!("Scanning {} version...", region_code));
    
    let mut tasks = Vec::new();
    find_game_files(input_path, &mut tasks).map_err(|e| e.to_string())?;

    let _ = tx.send(format!("Found {} tasks. Starting...", tasks.len()));

    let count = AtomicI32::new(0);
    let region_ref = region_code.to_string();

    tasks.par_iter().for_each(|task_path| {
        process_task_file(task_path, output_dir, &count, &tx, &index_arc, &region_ref);
    });

    let _ = tx.send(format!("Decryption complete. Processed {} files.", count.load(Ordering::Relaxed)));
    
    crate::core::import::sort::sort_game_files(tx)
}

// --- Helpers ---

#[cfg(feature = "dev")]
fn build_file_index(root: &Path) -> std::collections::HashMap<String, Vec<PathBuf>> {
    let mut index = std::collections::HashMap::new();
    let _ = scan_for_index(root, &mut index);
    index
}

#[cfg(feature = "dev")]
fn scan_for_index(dir: &Path, index: &mut std::collections::HashMap<String, Vec<PathBuf>>) -> std::io::Result<()> {
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
fn find_game_files(dir: &Path, list: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !dir.is_dir() { return Ok(()); }
    for entry in fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_game_files(&path, list)?;
        } else if let Some(ext) = path.extension() {
            let s = ext.to_string_lossy().to_lowercase();
            if s == "list" || s == "apk" || s == "xapk" {
                list.push(path);
            }
        }
    }
    Ok(())
}

#[cfg(feature = "dev")]
fn process_task_file(
    path: &Path, 
    out_dir: &Path, 
    count: &AtomicI32, 
    tx: &Sender<String>, 
    index: &Arc<std::collections::HashMap<String, Vec<PathBuf>>>, 
    region: &str
) {
    let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
    
    if ext == "apk" || ext == "xapk" {
        if let Err(e) = process_apk(path, out_dir, count, tx, index, region) {
             let _ = tx.send(format!("Error processing APK: {}", e));
        }
    } else if ext == "list" {
        let pack_path = path.with_extension("pack");
        if pack_path.exists() {
            if let Ok(data) = fs::read(path) {
                if let Ok(content) = decrypt_list_file(&data) {
                    let _ = extract_pack_contents(&content, &pack_path, out_dir, count, tx, index, region);
                }
            }
        }
    }
}

#[cfg(feature = "dev")]
fn decrypt_list_file(data: &[u8]) -> Result<String, String> {
    let k_pack = keys::get_md5_key("pack");
    if let Ok(b) = keys::decrypt_ecb_with_key(data, &k_pack) {
        if let Ok(s) = String::from_utf8(b) { return Ok(s); }
    }
    let k_bc = keys::get_md5_key("battlecats");
    if let Ok(b) = keys::decrypt_ecb_with_key(data, &k_bc) {
        if let Ok(s) = String::from_utf8(b) { return Ok(s); }
    }
    Err("Decryption failed".into())
}

// [CRITICAL FIX] Logic to determine correct language code from pack filename
#[cfg(feature = "dev")]
fn determine_pack_code(pack_filename: &str, selected_region: &str) -> String {
    if selected_region != "en" {
        return selected_region.to_string();
    }
    
    // Legacy logic: Check if filename contains "_es", "_fr", etc.
    for code in patterns::GLOBAL_CODES {
        if *code == "en" { continue; } 
        if pack_filename.contains(&format!("_{}", code)) {
            return code.to_string();
        }
    }
    "en".to_string()
}

#[cfg(feature = "dev")]
fn extract_pack_contents(
    content: &str, 
    pack_path: &Path, 
    out_dir: &Path, 
    count: &AtomicI32,
    tx: &Sender<String>,
    index: &Arc<std::collections::HashMap<String, Vec<PathBuf>>>, 
    region: &str
) -> Result<(), String> {
    let mut f = fs::File::open(pack_path).map_err(|e| e.to_string())?;
    
    // [CRITICAL FIX] Calculate the correct code for THIS specific pack
    let pack_name_str = pack_path.file_name().unwrap_or_default().to_string_lossy();
    let current_pack_code = determine_pack_code(&pack_name_str, region);

    for line in content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 { continue; }
        
        let name = parts[0];
        let offset: u64 = parts[1].parse().unwrap_or(0);
        let size: usize = parts[2].parse().unwrap_or(0);
        
        if should_skip(name, size, out_dir, index) { continue; }

        let aligned = if size % 16 == 0 { size } else { ((size / 16) + 1) * 16 };
        if f.seek(SeekFrom::Start(offset)).is_err() { continue; }
        
        let mut buf = vec![0u8; aligned];
        if f.read_exact(&mut buf).is_err() { continue; }

        if let Ok((decrypted, _)) = keys::decrypt_pack_chunk(&buf, name) {
            let final_data = &decrypted[..std::cmp::min(size, decrypted.len())];
            
            // [CRITICAL FIX] Use current_pack_code instead of region
            let target = if patterns::REGION_SENSITIVE_FILES.iter().any(|&x| name.ends_with(x)) {
                 let p = Path::new(name);
                 let stem = p.file_stem().unwrap().to_string_lossy();
                 let ext = p.extension().unwrap().to_string_lossy();
                 out_dir.join(format!("{}_{}.{}", stem, current_pack_code, ext))
            } else {
                 out_dir.join(name)
            };

            if write_smart(&target, final_data, name) {
                let c = count.fetch_add(1, Ordering::Relaxed);
                if c % 50 == 0 { let _ = tx.send(format!("Decrypted {} files...", c)); }
            }
        }
    }
    Ok(())
}

#[cfg(feature = "dev")]
fn should_skip(name: &str, size: usize, out_dir: &Path, index: &std::collections::HashMap<String, Vec<PathBuf>>) -> bool {
    if patterns::CHECK_LINE_FILES.contains(&name) { return false; }
    if name.ends_with("img015_th.imgcut") { return true; }
    if patterns::REGION_SENSITIVE_FILES.iter().any(|&x| name.ends_with(x)) { return false; }

    let lower = name.to_lowercase();
    if let Some(paths) = index.get(&lower) {
        for p in paths {
            if let Ok(m) = fs::metadata(p) {
                if m.len() as usize >= size.saturating_sub(16) { return true; }
            }
        }
    }
    let p = out_dir.join(name);
    if p.exists() {
        if let Ok(m) = fs::metadata(&p) {
            if m.len() as usize >= size.saturating_sub(16) { return true; }
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
            if let Ok(existing) = fs::read(path) {
                let old_lines = count_lines_bytes(&existing);
                let new_lines = count_lines_bytes(data);
                if new_lines <= old_lines { return false; }
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

    let temp_ext = format!("tmp_{:?}", std::thread::current().id()).replace("ThreadId(", "").replace(")", "");
    let temp_path = path.with_extension(&temp_ext);

    if fs::write(&temp_path, data).is_err() { return false; }
    let _ = fs::rename(&temp_path, path);
    true
}

#[cfg(feature = "dev")]
fn process_apk(
    apk_path: &Path, 
    output_dir: &Path, 
    count: &AtomicI32, 
    tx: &Sender<String>,
    file_index: &Arc<std::collections::HashMap<String, Vec<PathBuf>>>, 
    region: &str
) -> Result<(), String> {
    let apk_file = fs::File::open(apk_path).map_err(|e| e.to_string())?;
    let mut zip_archive = ZipArchive::new(apk_file).map_err(|e| e.to_string())?;

    let mut list_pack_pairs = Vec::new();
    for i in 0..zip_archive.len() {
        if let Ok(file) = zip_archive.by_index(i) {
            let name = file.name().to_string();
            if name.ends_with(".list") {
                let pack = name.replace(".list", ".pack");
                list_pack_pairs.push((name, pack));
            }
        }
    }

    for (list_name, pack_name) in list_pack_pairs {
        let mut list_data = Vec::new();
        let mut read_ok = false;

        if let Ok(mut f) = zip_archive.by_name(&list_name) {
            if f.read_to_end(&mut list_data).is_ok() { read_ok = true; }
        } 

        if read_ok {
            if let Ok(content) = decrypt_list_file(&list_data) {
                if let Ok(mut pack_file) = zip_archive.by_name(&pack_name) {
                    let safe_name = Path::new(&pack_name).file_name().unwrap().to_string_lossy();
                    let temp_path = output_dir.join(format!("_temp_{}", safe_name));
                    
                    if let Ok(mut temp) = fs::File::create(&temp_path) {
                        if std::io::copy(&mut pack_file, &mut temp).is_ok() {
                            let _ = extract_pack_contents(&content, &temp_path, output_dir, count, tx, file_index, region);
                            let _ = fs::remove_file(temp_path);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}