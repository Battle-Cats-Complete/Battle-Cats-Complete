use std::fs;
use std::io::{Read, Seek, SeekFrom}; // Removed unused imports
use std::path::Path;
use std::sync::mpsc::Sender;
use super::crypto;

// Helper: Takes raw encrypted bytes from a .list file and tries to decrypt it
fn decrypt_list_file(data: &[u8]) -> Result<String, String> {
    // 1. Try "pack" key
    let pack_key = crypto::get_md5_key("pack");
    if let Ok(bytes) = crypto::decrypt_ecb_with_key(data, &pack_key) {
        if let Ok(text) = String::from_utf8(bytes) {
            return Ok(text);
        }
    }

    // 2. Try "battlecats" key
    let bc_key = crypto::get_md5_key("battlecats");
    if let Ok(bytes) = crypto::decrypt_ecb_with_key(data, &bc_key) {
        if let Ok(text) = String::from_utf8(bytes) {
            return Ok(text);
        }
    }

    Err("Failed to decrypt list file: Unknown key.".to_string())
}

fn extract_pack_contents(
    list_content: &str, 
    pack_path: &Path, 
    output_dir: &Path,
    count: &mut i32,
    tx: Sender<String>
) -> Result<(), String> {
    
    let mut pack_file = fs::File::open(pack_path)
        .map_err(|e| format!("Failed to open pack: {}", e))?;

    let pack_filename = pack_path.file_name().unwrap().to_string_lossy().to_string();

    let _ = tx.send(format!("Extracting: {}", pack_filename));

    for line in list_content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 { continue; }

        let filename = parts[0];
        let offset: u64 = parts[1].parse().unwrap_or(0);
        let size: usize = parts[2].parse().unwrap_or(0);

        if *count % 50 == 0 {
            let _ = tx.send(format!("Extracted {} files (Current: {})", count, filename));
        }

        let aligned_size = if size % 16 == 0 { size } else { ((size / 16) + 1) * 16 };
        if pack_file.seek(SeekFrom::Start(offset)).is_err() { continue; }

        let mut buffer = vec![0u8; aligned_size];
        if pack_file.read_exact(&mut buffer).is_err() { continue; }

        match crypto::decrypt_pack_chunk(&buffer, &pack_filename) {
            Ok(decrypted_chunk) => {
                let final_len = std::cmp::min(size, decrypted_chunk.len());
                let final_data = &decrypted_chunk[..final_len];

                let dest_path = output_dir.join(filename);
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }

                if let Err(e) = fs::write(&dest_path, final_data) {
                    println!("Error writing {}: {}", filename, e);
                } else {
                    *count += 1;
                }
            },
            Err(_) => {}
        }
    }
    Ok(())
}

fn process_apk(apk_path: &Path, output_dir: &Path, count: &mut i32, tx: Sender<String>) -> Result<(), String> {
    let _ = tx.send("Opening APK".to_string());
    
    let file = fs::File::open(apk_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Bad APK: {}", e))?;

    let len = archive.len();
    for i in 0..len {
        // SCOPE 1: Get filename ONLY. 
        // We do not keep the file open. We just check the name.
        let filename_string;
        {
            let file = archive.by_index(i).unwrap();
            filename_string = file.name().to_string();
        } // 'file' is dropped here. 'archive' is free again.

        if filename_string.ends_with(".list") {
            // SCOPE 2: Read the list data.
            let mut list_buf = Vec::new();
            {
                let mut file_reader = archive.by_index(i).unwrap(); 
                file_reader.read_to_end(&mut list_buf).unwrap();
            } // 'file_reader' dropped. 'archive' free again.
            
            if let Ok(list_str) = decrypt_list_file(&list_buf) {
                let pack_name = filename_string.replace(".list", ".pack");
                
                // SCOPE 3: Check for pack and extract
                // We assume 'by_name' might fail if pack doesn't exist
                let mut pack_found = false;
                
                // We have to extract the pack to a temp file because we need Seek
                let temp_pack_path = output_dir.join("temp.pack");
                
                {
                    if let Ok(mut pack_file) = archive.by_name(&pack_name) {
                        pack_found = true;
                        let mut temp_f = fs::File::create(&temp_pack_path).map_err(|e| e.to_string())?;
                        std::io::copy(&mut pack_file, &mut temp_f).map_err(|e| e.to_string())?;
                    }
                } // 'pack_file' dropped.

                if pack_found {
                    extract_pack_contents(&list_str, &temp_pack_path, output_dir, count, tx.clone())?;
                    fs::remove_file(temp_pack_path).ok();
                }
            }
        }
    }
    Ok(())
}

fn scan_recursive(current_dir: &Path, output_dir: &Path, count: &mut i32, tx: Sender<String>) -> Result<(), String> {
    let entries = fs::read_dir(current_dir).map_err(|e| e.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_dir() {
            scan_recursive(&path, output_dir, count, tx.clone())?;
        } else {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                
                if ext_str == "apk" {
                     process_apk(&path, output_dir, count, tx.clone())?;
                }
                else if ext_str == "list" {
                    let pack_path = path.with_extension("pack");
                    
                    if pack_path.exists() {
                        let list_data = fs::read(&path).map_err(|e| e.to_string())?;
                        
                        if let Ok(list_content) = decrypt_list_file(&list_data) {
                            extract_pack_contents(&list_content, &pack_path, output_dir, count, tx.clone())?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn import_all_from_folder(folder_path: &str, tx: Sender<String>) -> Result<String, String> {
    let output_dir = Path::new("game");
    if !output_dir.exists() {
        fs::create_dir(output_dir).map_err(|e| format!("Could not create 'game' folder: {}", e))?;
    }

    let mut count = 0;
    scan_recursive(Path::new(folder_path), output_dir, &mut count, tx.clone())?;

    Ok(format!("Success! Processed {} files.", count))
}