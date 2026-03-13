use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::mpsc::Sender;

use crate::features::cat::patterns as cat_patterns; 
use crate::global::io::patterns as global_patterns;
use super::{cat, global, enemy};

pub fn count_lines(path: &Path) -> usize {
    if let Some(ext) = path.extension() {
        let s = ext.to_string_lossy();
        if s == "png" { return 0; } 
    }

    if let Ok(f) = fs::File::open(path) {
        let reader = io::BufReader::new(f);
        reader.lines().count()
    } else {
        0
    }
}

pub fn move_if_bigger(src: &Path, dest: &Path) -> std::io::Result<bool> {
    if dest.exists() {
        let src_lines = count_lines(src);
        let dest_lines = count_lines(dest);

        if src_lines > dest_lines {
            let _ = fs::remove_file(dest);
            fs::rename(src, dest)?;
            Ok(true)
        } else {
            let _ = fs::remove_file(src)?;
            Ok(false)
        }
    } else {
        if let Some(parent) = dest.parent() {
            if !parent.exists() { let _ = fs::create_dir_all(parent); }
        }
        fs::rename(src, dest)?;
        Ok(true)
    }
}

pub fn move_fast(src: &Path, dest: &Path) -> std::io::Result<()> {
    if let Some(parent) = dest.parent() {
        if !parent.exists() { let _ = fs::create_dir_all(parent); }
    }
    if dest.exists() {
        let _ = fs::remove_file(dest);
    }
    fs::rename(src, dest)?;
    Ok(())
}

pub fn sort_game_files(tx: Sender<String>) -> Result<(), String> {
    let raw_dir = Path::new("game/raw");
    let cats_dir = Path::new("game/cats");
    let assets_dir = Path::new("game/assets");
    let enemy_dir = Path::new("game/enemies");

    if !raw_dir.exists() {
        return Err("Raw directory not found.".to_string());
    }

    let files_to_sort = fs::read_dir(raw_dir).map(|iter| iter.count()).unwrap_or(0);
    
    if files_to_sort == 0 {
        let _ = tx.send("No new files to sort.".to_string());
        return Ok(());
    }

    let update_interval = (files_to_sort / 100).max(10);
    let _ = tx.send(format!("Sorting {} new or updated files...", files_to_sort));

    let cat_matcher = cat::CatMatcher::new();
    let global_matcher = global::GlobalMatcher::new();
    let enemy_matcher = enemy::EnemyMatcher::new();

    let mut count = 0;
    
    for entry in fs::read_dir(raw_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() { continue; }

        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        let mut base_name = name.to_string();
        let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let ext = path.extension().unwrap_or_default().to_string_lossy().to_string();

        for &(code, _) in global_patterns::APP_LANGUAGES {
            let suffix = format!("_{}", code);
            // Verify the stem is long enough and ends exactly with "_xx"
            if stem.len() > suffix.len() && stem.ends_with(&suffix) {
                // Ensure the base name is matched without the region tag
                let clean_stem = &stem[..stem.len() - suffix.len()];
                base_name = if ext.is_empty() { 
                    clean_stem.to_string() 
                } else { 
                    format!("{}.{}", clean_stem, ext) 
                };
                break;
            }
        }

        let mut processed = false;
        
        // Match against base_name (clean), Move using name (original)
        if cat_patterns::CAT_UNIVERSAL_FILES.contains(&base_name.as_str()) {
            let dest = cats_dir.join(name);
            if global_patterns::CHECK_LINE_FILES.contains(&base_name.as_str()) {
                if let Ok(moved) = move_if_bigger(&path, &dest) { if moved { processed = true; } }
            } else {
                if move_fast(&path, &dest).is_ok() { processed = true; }
            }
        } else {
            let dest_folder = global_matcher.get_dest(&base_name, assets_dir)
                .or_else(|| cat_matcher.get_dest(&base_name, cats_dir))
                .or_else(|| enemy_matcher.get_dest(&base_name, enemy_dir));

            if let Some(folder) = dest_folder {
                if !folder.exists() { let _ = fs::create_dir_all(&folder); }
                let dest = folder.join(name);
                if global_patterns::CHECK_LINE_FILES.contains(&base_name.as_str()) {
                    if let Ok(moved) = move_if_bigger(&path, &dest) { if moved { processed = true; } }
                } else {
                    if move_fast(&path, &dest).is_ok() { processed = true; }
                }
            }
        }

        if processed {
            count += 1;
            if count % update_interval == 0 {
                let _ = tx.send(format!("Sorted {} files | Current: {}", count, name));
            }
        }
    }

    let _ = tx.send("Success! Files sorted.".to_string());
    Ok(())
}