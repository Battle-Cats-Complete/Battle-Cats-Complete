use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::mpsc::Sender;
use regex::Regex;
use crate::core::patterns; 

fn count_lines(path: &Path) -> usize {
    if let Some(ext) = path.extension() {
        let s = ext.to_string_lossy();
        if s == "png" || s == "imgcut" || s == "mamodel" { return 0; }
    }

    if let Ok(file) = fs::File::open(path) {
        let reader = io::BufReader::new(file);
        reader.lines().count()
    } else {
        0
    }
}

fn move_if_bigger(src: &Path, dest: &Path) -> std::io::Result<bool> {
    if dest.exists() {
        let src_lines = count_lines(src);
        let dest_lines = count_lines(dest);

        if src_lines > dest_lines {
            let _ = fs::remove_file(dest);
            fs::rename(src, dest)?;
            Ok(true)
        } else {
            fs::remove_file(src)?;
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

fn move_fast(src: &Path, dest: &Path) -> std::io::Result<()> {
    if let Some(parent) = dest.parent() {
        if !parent.exists() { let _ = fs::create_dir_all(parent); }
    }
    if dest.exists() {
        let _ = fs::remove_file(dest);
    }
    fs::rename(src, dest)?;
    Ok(())
}

fn map_egg_form(code: &str) -> &str {
    match code { "00" => "f", _ => "c" }
}

pub fn sort_game_files(tx: Sender<String>) -> Result<(), String> {
    let raw_dir = Path::new("game/raw");
    let cats_dir = Path::new("game/cats");
    let assets_dir = Path::new("game/assets");

    if !raw_dir.exists() {
        return Err("Raw directory not found.".to_string());
    }

    let _ = tx.send("Sorting files...".to_string());

    let universal_pattern = Regex::new(patterns::CAT_UNIVERSAL_PATTERN).unwrap();
    let skill_desc_pattern = Regex::new(patterns::SKILL_DESC_PATTERN).unwrap();
    let stats_regex = Regex::new(patterns::CAT_STATS_PATTERN).unwrap();
    let icon_regex = Regex::new(patterns::CAT_ICON_PATTERN).unwrap();
    let upgrade_regex = Regex::new(patterns::CAT_UPGRADE_PATTERN).unwrap();
    let gacha_regex = Regex::new(patterns::CAT_GACHA_PATTERN).unwrap();
    let anim_regex = Regex::new(patterns::CAT_ANIM_PATTERN).unwrap();
    let maanim_regex = Regex::new(patterns::CAT_MAANIM_PATTERN).unwrap();
    let explain_regex = Regex::new(patterns::CAT_EXPLAIN_PATTERN).unwrap();
    
    let egg_icon_regex = Regex::new(patterns::EGG_ICON_PATTERN).unwrap();
    let egg_upgrade_regex = Regex::new(patterns::EGG_UPGRADE_PATTERN).unwrap();
    let egg_gacha_regex = Regex::new(patterns::EGG_GACHA_PATTERN).unwrap();
    let egg_anim_regex = Regex::new(patterns::EGG_ANIM_PATTERN).unwrap();
    let egg_maanim_regex = Regex::new(patterns::EGG_MAANIM_PATTERN).unwrap();

    let mut moved_count = 0;
    
    for entry_result in fs::read_dir(raw_dir).map_err(|e| e.to_string())? {
        let entry = entry_result.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() { continue; }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        let mut dest_folder = None;

        if skill_desc_pattern.is_match(filename) {
            dest_folder = Some(cats_dir.join("SkillDescriptions"));
        }
        else if universal_pattern.is_match(filename) {
            dest_folder = Some(cats_dir.join("unitevolve"));
        }
        else if patterns::CAT_UNIVERSAL_FILES.contains(&filename) {
            let dest_path = cats_dir.join(filename);
            
            if patterns::CHECK_LINE_FILES.contains(&filename) {
                if let Ok(moved) = move_if_bigger(&path, &dest_path) {
                    if moved { moved_count += 1; }
                }
            } else {
                if move_fast(&path, &dest_path).is_ok() { moved_count += 1; }
            }
            continue; 
        }
        else if let Some(caps) = stats_regex.captures(filename) {
            if let Ok(id) = caps[1].parse::<u32>() {
                if id > 0 { dest_folder = Some(cats_dir.join(format!("{:03}", id - 1))); }
            }
        }
        else if let Some(caps) = icon_regex.captures(filename) { dest_folder = Some(cats_dir.join(&caps[1]).join(&caps[2])); }
        else if let Some(caps) = upgrade_regex.captures(filename) { dest_folder = Some(cats_dir.join(&caps[1]).join(&caps[2])); }
        else if let Some(caps) = gacha_regex.captures(filename) { dest_folder = Some(cats_dir.join(&caps[1])); }
        else if let Some(caps) = anim_regex.captures(filename) { dest_folder = Some(cats_dir.join(&caps[1]).join(&caps[2]).join("anim")); }
        else if let Some(caps) = maanim_regex.captures(filename) { dest_folder = Some(cats_dir.join(&caps[1]).join(&caps[2]).join("anim")); }
        else if let Some(caps) = explain_regex.captures(filename) {
            if let Ok(id) = caps[1].parse::<u32>() {
                if id > 0 { dest_folder = Some(cats_dir.join(format!("{:03}", id - 1)).join("lang")); }
            }
        }
        else if let Some(caps) = egg_icon_regex.captures(filename) {
            dest_folder = Some(cats_dir.join(format!("egg_{}", &caps[1])).join(map_egg_form(&caps[2])));
        }
        else if let Some(caps) = egg_upgrade_regex.captures(filename) {
            dest_folder = Some(cats_dir.join(format!("egg_{}", &caps[1])).join(map_egg_form(&caps[2])));
        }
        else if let Some(caps) = egg_gacha_regex.captures(filename) {
            dest_folder = Some(cats_dir.join(format!("egg_{}", &caps[1])));
        }
        else if let Some(caps) = egg_anim_regex.captures(filename) {
            dest_folder = Some(cats_dir.join(format!("egg_{}", &caps[1])).join("anim"));
        }
        else if let Some(caps) = egg_maanim_regex.captures(filename) {
            dest_folder = Some(cats_dir.join(format!("egg_{}", &caps[1])).join("anim"));
        }
        else if filename.starts_with("img015") {
            dest_folder = Some(assets_dir.join("img015"));
        }

        if let Some(folder) = dest_folder {
            if !folder.exists() { 
                let _ = fs::create_dir_all(&folder); 
            }
            let dest_path = folder.join(filename);

            if patterns::CHECK_LINE_FILES.contains(&filename) {
                if let Ok(moved) = move_if_bigger(&path, &dest_path) {
                    if moved { moved_count += 1; }
                }
            } else {
                if move_fast(&path, &dest_path).is_ok() { moved_count += 1; }
            }
            
            if moved_count % 500 == 0 {
                let _ = tx.send(format!("Sorted {} files...", moved_count));
            }
        }
    }

    let _ = tx.send("Success! Files sorted.".to_string());
    Ok(())
}