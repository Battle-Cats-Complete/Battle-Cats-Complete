use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::Sender;
use zip::write::FileOptions;

pub fn create_game_zip(tx: Sender<String>, level: i32) -> Result<(), String> {
    let src = Path::new("game");
    let dest_dir = Path::new("exports");
    let dest_path = dest_dir.join("game.zip");
    
    if !src.exists() { return Err("No 'game' folder found to export.".to_string()); }
    if !dest_dir.exists() { fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?; }
    
    let _ = tx.send("Zipping files...".to_string());
    
    let f = fs::File::create(&dest_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(f);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(level))
        .unix_permissions(0o755);
    
    let mut count = 0;
    let mut stack = vec![src.to_path_buf()];
    
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.to_string_lossy().contains("game/raw") || path.to_string_lossy().contains("game\\raw") { 
                    continue; 
                }
                
                if path.is_dir() {
                    stack.push(path);
                } else {
                    let name = path.strip_prefix(src).unwrap().to_string_lossy().replace("\\", "/");
                    let _ = zip.start_file(name, options);
                    
                    if let Ok(mut f) = fs::File::open(&path) {
                        let mut buf = Vec::new();
                        if f.read_to_end(&mut buf).is_ok() {
                            let _ = zip.write_all(&buf);
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    
    let _ = zip.finish();
    let _ = tx.send(format!("Success! Exported {} files to exports/game.zip", count));
    Ok(())
}