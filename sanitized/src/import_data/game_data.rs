use std::fs;
use std::path::Path;
use std::sync::mpsc::Sender;
use zip::ZipArchive;
use crate::patterns; 

// --- FOLDER LOGIC ---
pub fn import_from_folder(source_folder: &str, tx: Sender<String>) -> Result<(), String> {
    let input_path = Path::new(source_folder);
    let output_dir = Path::new("game/raw");
    
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(|e| format!("Could not create 'game/raw': {}", e))?;
    }

    let _ = tx.send("Scanning folder...".to_string());

    let mut file_count = 0;

    // Recursive copy
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
                    if let Some(name_os) = path.file_name() {
                        let name_str = name_os.to_string_lossy().to_string();
                        
                        // Default destination is just the filename
                        let mut dest_filename = name_str.clone();

                        // AUTOMATIC RENAMING LOGIC
                        // If this file is sensitive (e.g. img015.png), append "_au"
                        if patterns::REGION_SENSITIVE_FILES.contains(&name_str.as_str()) {
                            let path_obj = Path::new(&name_str);
                            let stem = path_obj.file_stem().map(|s| s.to_string_lossy()).unwrap_or_default();
                            let ext = path_obj.extension().map(|s| s.to_string_lossy()).unwrap_or_default();
                            dest_filename = format!("{}_au.{}", stem, ext);
                        }

                        let dest_path = output_dir.join(&dest_filename);
                        
                        // Simple copy, overwrite if exists
                        if fs::copy(&path, &dest_path).is_ok() {
                            file_count += 1;
                            // Lively logging
                            if file_count % 50 == 0 {
                                let _ = tx.send(format!("Copied {} files | Current: {}", file_count, dest_filename));
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err("Selected path is not a directory.".to_string());
    }

    let _ = tx.send(format!("Copy complete. {} files moved to game/raw.", file_count));
    Ok(())
}

// --- ZIP LOGIC ---
pub fn import_from_zip(zip_path_str: &str, tx: Sender<String>) -> Result<(), String> {
    let file = fs::File::open(zip_path_str).map_err(|e| format!("Failed to open zip: {}", e))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid zip archive: {}", e))?;

    let _ = tx.send("Validating Archive...".to_string());

    // --- AUTOMATIC VALIDATION ---
    for (dir, prefix, extensions) in patterns::ESSENTIAL_FILES {
        let mut set_satisfied = false;

        for &code in patterns::GLOBAL_CODES {
            if code == "en" { continue; } 

            let mut all_extensions_found = true;
            let mut found_files = Vec::new();

            for &ext in *extensions {
                let expected_path = format!("{}/{}_{}.{}", dir, prefix, code, ext);
                let expected_filename = format!("{}_{}.{}", prefix, code, ext);
                
                if archive.by_name(&expected_path).is_err() {
                    all_extensions_found = false;
                    break;
                } else {
                    found_files.push(expected_filename);
                }
            }

            if all_extensions_found {
                set_satisfied = true;
                // LOG SUCCESS IN GREEN (via "was found!")
                for f in found_files {
                    let _ = tx.send(format!("{} was found!", f));
                }
                break; 
            }
        }

        if !set_satisfied {
            return Err(format!("Import Aborted: ZIP Archive is Missing Essential Files! (Missing set for '{}')", prefix));
        }
    }

    let _ = tx.send("Validation Passed. Extracting...".to_string());

    // 2. Extraction Step
    let len = archive.len();
    for i in 0..len {
        let mut file = archive.by_index(i).unwrap();
        
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        if outpath.starts_with("game") {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    let _ = fs::create_dir_all(p);
                }
            }

            if file.is_dir() {
                if !outpath.exists() {
                    let _ = fs::create_dir_all(&outpath);
                }
            } else {
                let safe_name = outpath.file_name().unwrap_or_default().to_string_lossy().to_string();
                
                if let Ok(mut outfile) = fs::File::create(&outpath) {
                    if std::io::copy(&mut file, &mut outfile).is_ok() {
                        // Lively logging
                        if i % 50 == 0 {
                            let _ = tx.send(format!("Extracted {} files | Current: {}", i + 1, safe_name));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}