use std::fs;
use std::path::Path;
use std::sync::mpsc::Sender;
use regex::Regex;
use crate::patterns;

pub fn sort_game_files(tx: Sender<String>) -> Result<(), String> {
    let raw_dir = Path::new("game/raw");
    let cats_dir = Path::new("game/cats");
    let assets_dir = Path::new("game/assets");

    if !raw_dir.exists() {
        return Err("Raw directory not found. Did extraction fail?".to_string());
    }

    let _ = tx.send("Sorting files...".to_string());

    let universal_pattern = Regex::new(patterns::CAT_UNIVERSAL_PATTERN).unwrap();
    let re_stats = Regex::new(patterns::CAT_STATS_PATTERN).unwrap();
    let re_icon = Regex::new(patterns::CAT_ICON_PATTERN).unwrap();
    let re_upgrade = Regex::new(patterns::CAT_UPGRADE_PATTERN).unwrap();
    let re_gacha = Regex::new(patterns::CAT_GACHA_PATTERN).unwrap();
    let re_anim = Regex::new(patterns::CAT_ANIM_PATTERN).unwrap();
    let re_maanim = Regex::new(patterns::CAT_MAANIM_PATTERN).unwrap();
    let re_explain = Regex::new(patterns::CAT_EXPLAIN_PATTERN).unwrap();
    
    // Assets
    let re_img015 = Regex::new(patterns::ASSET_IMG015_PATTERN).unwrap();
    let re_imgcut = Regex::new(patterns::ASSET_015CUT_PATTERN).unwrap();

    let mut moved_count = 0;
    
    for entry in fs::read_dir(raw_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        
        if path.is_dir() { continue; }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // --- NEW: Delete duplicate img015.png ---
        // This file is the "raw" version without a country code. 
        // Since we've already extracted the coded versions (e.g. img015_th.png), this is trash.
        if filename == "img015.png" {
            let _ = fs::remove_file(&path);
            continue;
        }
        // ----------------------------------------

        let mut dest_folder = None;

        if patterns::CAT_UNIVERSAL_FILES.contains(&filename) || universal_pattern.is_match(filename) {
            dest_folder = Some(cats_dir.to_path_buf());
        }
        else if let Some(caps) = re_stats.captures(filename) {
            if let Ok(file_id) = caps[1].parse::<u32>() {
                if file_id > 0 {
                    let unit_id = file_id - 1;
                    let folder_id = format!("{:03}", unit_id);
                    dest_folder = Some(cats_dir.join(folder_id));
                }
            }
        }
        else if let Some(caps) = re_icon.captures(filename) {
            let (id, form) = (&caps[1], &caps[2]);
            dest_folder = Some(cats_dir.join(id).join(form));
        }
        else if let Some(caps) = re_upgrade.captures(filename) {
            let (id, form) = (&caps[1], &caps[2]);
            dest_folder = Some(cats_dir.join(id).join(form));
        }
        else if let Some(caps) = re_gacha.captures(filename) {
            let id = &caps[1];
            dest_folder = Some(cats_dir.join(id));
        }
        else if let Some(caps) = re_anim.captures(filename) {
            let (id, form) = (&caps[1], &caps[2]);
            dest_folder = Some(cats_dir.join(id).join(form).join("anim"));
        }
        else if let Some(caps) = re_maanim.captures(filename) {
            let (id, form) = (&caps[1], &caps[2]);
            dest_folder = Some(cats_dir.join(id).join(form).join("anim"));
        }
        else if let Some(caps) = re_explain.captures(filename) {
            let raw_id = &caps[1];
            if let Ok(file_id) = raw_id.parse::<u32>() {
                if file_id > 0 {
                    let unit_id = file_id - 1;
                    let folder_id = format!("{:03}", unit_id);
                    dest_folder = Some(cats_dir.join(folder_id));
                }
            }
        }
        else if re_img015.is_match(filename) {
            dest_folder = Some(assets_dir.to_path_buf());
        }
        else if re_imgcut.is_match(filename) {
            dest_folder = Some(assets_dir.to_path_buf());
        }

        if let Some(folder) = dest_folder {
            if !folder.exists() {
                fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
            }
            let dest_path = folder.join(filename);
            fs::rename(&path, &dest_path).map_err(|e| e.to_string())?;
            moved_count += 1;
            
            if moved_count % 500 == 0 {
                let _ = tx.send(format!("Sorted {} files...", moved_count));
            }
        }
    }

    let _ = tx.send(format!("Sorting complete! Moved {} files.", moved_count));
    Ok(())
}