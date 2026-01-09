use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicI32, Ordering};
use zip::ZipArchive;
use rayon::prelude::*;

pub fn import_from_folder(source: &str, tx: Sender<String>) -> Result<String, String> {
    let input = Path::new(source);
    let output = Path::new("game/raw");
    
    if !output.exists() { 
        fs::create_dir_all(output).map_err(|e| e.to_string())?; 
    }
    
    let _ = tx.send("Scanning folder structure...".to_string());
    
    // Collect all files first
    let mut files_to_copy = Vec::new();
    if let Err(e) = scan_dir(input, &mut files_to_copy) {
        return Err(format!("Failed to scan folder: {}", e));
    }

    let _ = tx.send(format!("Found {} files. Starting parallel import...", files_to_copy.len()));
    let count = AtomicI32::new(0);

    files_to_copy.par_iter().for_each(|path| {
        if let Some(name) = path.file_name() {
            let dest = output.join(name);
            if fs::copy(path, &dest).is_ok() {
                let c = count.fetch_add(1, Ordering::Relaxed);
                if c % 100 == 0 { 
                    let _ = tx.send(format!("Imported {} files...", c)); 
                }
            }
        }
    });

    Ok(format!("Success! Imported {} files.", count.load(Ordering::Relaxed)))
}

pub fn import_from_zip(source: &str, tx: Sender<String>) -> Result<String, String> {
    let f = fs::File::open(source).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(f).map_err(|e| e.to_string())?;
    let output = Path::new("game/raw");
    
    if !output.exists() { 
        fs::create_dir_all(output).map_err(|e| e.to_string())?; 
    }
    
    let len = archive.len();
    let _ = tx.send(format!("Extracting {} files from archive...", len));

    for i in 0..len {
        let mut file = archive.by_index(i).unwrap();
        if file.is_dir() { continue; }
        
        let name = Path::new(file.name()).file_name().unwrap_or_default().to_string_lossy().to_string();
        let dest = output.join(name);
        
        if let Ok(mut out) = fs::File::create(&dest) {
            let _ = std::io::copy(&mut file, &mut out);
        }
        
        if i % 100 == 0 { 
            let _ = tx.send(format!("Extracted {} files...", i)); 
        }
    }
    Ok("Success! Zip extracted.".to_string())
}

// Recursive helper to find all files
fn scan_dir(dir: &Path, list: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                scan_dir(&path, list)?;
            } else {
                list.push(path);
            }
        }
    }
    Ok(())
}