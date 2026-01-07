use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use std::sync::{Arc, mpsc::{self, Receiver}};
use rayon::prelude::*;
use super::stats::{self, CatRaw, CatLevelCurve, UnitBuyRow}; 

pub const SCAN_PRIORITY: &[&str] = &["en", "ja", "tw", "ko", "es", "de", "fr", "it", "th", ""];

#[derive(Clone, Debug)]
pub struct CatEntry {
    pub id: u32,
    pub image_path: PathBuf,
    pub names: Vec<String>, 
    pub forms: [bool; 4],
    pub stats: Vec<Option<CatRaw>>,
    pub curve: Option<CatLevelCurve>,
    pub atk_anim_frames: [i32; 4], 
    pub egg_ids: (i32, i32),
}

pub fn start_scan(lang: String) -> Receiver<CatEntry> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let cats_dir = Path::new("game/cats");
        
        let level_curves = Arc::new(load_level_curves(cats_dir));
        let unit_buy_map = Arc::new(stats::load_unitbuy(cats_dir));
        
        let entries: Vec<PathBuf> = match fs::read_dir(cats_dir) {
            Ok(read_dir) => read_dir
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect(),
            Err(_) => Vec::new(),
        };

        entries.par_iter().for_each(|path| {
            let tx = tx.clone();
            let curves = Arc::clone(&level_curves);
            let unit_buys = Arc::clone(&unit_buy_map);
            
            if let Some(entry) = process_cat_entry(path, &curves, &unit_buys, &lang) {
                let _ = tx.send(entry);
            }
        });
    });
    rx
}

fn load_level_curves(cats_dir: &Path) -> Vec<CatLevelCurve> {
    let mut curves = Vec::new();
    let level_file = cats_dir.join("unitlevel.csv");
    if let Ok(content) = fs::read_to_string(&level_file) {
        for line in content.lines() {
            curves.push(CatLevelCurve::from_csv_line(line));
        }
    }
    curves
}

fn process_cat_entry(
    original_path: &Path, 
    level_curves: &Vec<CatLevelCurve>, 
    unit_buys: &std::collections::HashMap<u32, UnitBuyRow>,
    lang: &str
) -> Option<CatEntry> {
    
    let stem = original_path.file_name()?.to_str()?;
    let id = stem.parse::<u32>().ok()?;

    let unit_buy = unit_buys.get(&id);
    if let Some(ub) = unit_buy {
        let is_egg = ub.egg_id_norm != -1;
        
        if !is_egg && ub.guide_order == -1 && id != 673 {
            return None; 
        }
    } else {
        return None;
    }
    let ub = unit_buy.unwrap(); 

    let cats_root = Path::new("game/cats");
    
    let get_form_path = |form_idx: usize, form_char: char| -> PathBuf {
        let is_egg_norm = form_idx == 0 && ub.egg_id_norm != -1;
        let is_egg_evol = form_idx == 1 && ub.egg_id_evol != -1;

        if is_egg_norm {
            cats_root.join(format!("egg_{:03}", ub.egg_id_norm)).join(form_char.to_string())
        } else if is_egg_evol {
            cats_root.join(format!("egg_{:03}", ub.egg_id_evol)).join(form_char.to_string())
        } else {
            original_path.join(form_char.to_string())
        }
    };

    let get_anim_file = |form_idx: usize, form_char: char| -> PathBuf {
        let is_egg_norm = form_idx == 0 && ub.egg_id_norm != -1;
        let is_egg_evol = form_idx == 1 && ub.egg_id_evol != -1;

        if is_egg_norm {
            cats_root.join(format!("egg_{:03}", ub.egg_id_norm))
                     .join("anim")
                     .join(format!("{:03}_m02.maanim", ub.egg_id_norm))
        } else if is_egg_evol {
            cats_root.join(format!("egg_{:03}", ub.egg_id_evol))
                     .join("anim")
                     .join(format!("{:03}_m02.maanim", ub.egg_id_evol))
        } else {
            original_path.join(form_char.to_string())
                         .join("anim")
                         .join(format!("{:03}_{}02.maanim", id, form_char))
        }
    };

    let forms_chars = ['f', 'c', 's', 'u'];
    let forms_paths = [
        get_form_path(0, 'f'),
        get_form_path(1, 'c'),
        get_form_path(2, 's'),
        get_form_path(3, 'u'),
    ];

    let forms_exist = [
        forms_paths[0].exists(),
        forms_paths[1].exists(),
        forms_paths[2].exists(),
        forms_paths[3].exists(),
    ];

    if !forms_exist[1] && id != 673 {
        return None;
    }

    let mut atk_anim_frames = [0; 4];
    for i in 0..4 {
        if forms_exist[i] {
            let anim_path = get_anim_file(i, forms_chars[i]);
            if let Ok(content) = fs::read_to_string(&anim_path) {
                atk_anim_frames[i] = parse_anim_length(&content);
            }
        }
    }

    // --- ICON FINDER ---
    let find_image = |base_path: &Path, name_no_ext: &str| -> Option<PathBuf> {
        let p1 = base_path.join(format!("{}.png", name_no_ext));
        if p1.exists() { return Some(p1); }
        let p2 = base_path.join(format!("{}.PNG", name_no_ext));
        if p2.exists() { return Some(p2); }
        None
    };

    let final_img_path = if ub.egg_id_norm != -1 {
        find_image(&forms_paths[0], &format!("udi{:03}_m00", ub.egg_id_norm))
            .or_else(|| find_image(&forms_paths[0], &format!("uni{:03}_m00", ub.egg_id_norm)))
    } else {
        find_image(&forms_paths[0], &format!("udi{:03}_f", id))
            .or_else(|| find_image(&forms_paths[0], &format!("uni{:03}_f00", id)))
    };

    let image_path = match final_img_path {
        Some(p) => p,
        None => return None, 
    };
    
    let mut names = vec![String::new(); 4];
    let target_file_id = id + 1;
    let lang_dir = original_path.join("lang"); 

    let codes_to_try: Vec<&str> = if lang.is_empty() {
        SCAN_PRIORITY.to_vec()
    } else {
        vec![lang]
    };

    for code in codes_to_try {
        if let Some(name_file_path) = find_name_file_for_code(&lang_dir, target_file_id, code) {
            if let Ok(bytes) = fs::read(&name_file_path) {
                let content = String::from_utf8_lossy(&bytes);
                let separator = if code == "ja" { ',' } else { '|' };

                let mut temp_names = vec![String::new(); 4];
                let mut found_valid_name = false;

                for (i, line) in content.lines().enumerate().take(4) {
                    if let Some(name_part) = line.split(separator).next() {
                        let trimmed = name_part.trim();
                        if !trimmed.is_empty() && !looks_like_garbage_id(trimmed) {
                            found_valid_name = true;
                            temp_names[i] = trimmed.to_string();
                        }
                    }
                }

                if found_valid_name {
                    names = temp_names;
                    break;
                }
            }
        }
    }
    
    if id == 673 && names[0].is_empty() {
        names[0] = "Cheetah Cat".to_string();
    }
    
    let mut stats = vec![None; 4];
    let stats_path = original_path.join(format!("unit{:03}.csv", target_file_id));
    if let Ok(content) = fs::read_to_string(&stats_path) {
        for (i, line) in content.lines().enumerate().take(4) {
            stats[i] = CatRaw::from_csv_line(line);
        }
    }

    Some(CatEntry { 
        id, 
        image_path,
        names,
        forms: forms_exist,
        stats, 
        curve: level_curves.get(id as usize).cloned(),
        atk_anim_frames,
        egg_ids: (ub.egg_id_norm, ub.egg_id_evol),
    })
}

fn looks_like_garbage_id(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '_')
}

fn find_name_file_for_code(lang_dir: &Path, target_id: u32, code: &str) -> Option<PathBuf> {
    if !lang_dir.exists() { return None; }
    
    let suffix = if code.is_empty() {
        ".csv".to_string()
    } else {
        format!("_{}.csv", code)
    };

    fs::read_dir(lang_dir).ok()?
        .flatten()
        .find_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("Unit_Explanation") || !name.ends_with(&suffix) {
                return None;
            }
            let num_part = name.trim_start_matches("Unit_Explanation").trim_end_matches(&suffix);
            if let Ok(num) = num_part.parse::<u32>() {
                if num == target_id { return Some(entry.path()); }
            }
            None
        })
}

fn parse_anim_length(content: &str) -> i32 {
    let mut max_frame = 0;
    let lines: Vec<Vec<i32>> = content
        .lines()
        .map(|line| {
            line.split(',')
                .filter_map(|c| c.trim().parse::<i32>().ok())
                .collect()
        })
        .collect();

    for (i, line) in lines.iter().enumerate() {
        if line.len() < 5 {
            continue;
        }

        let following_lines_amt = lines.get(i + 1).and_then(|l| l.get(0)).cloned().unwrap_or(0) as usize;
        if following_lines_amt == 0 {
            continue;
        }

        let first_anim_frame = lines.get(i + 2).and_then(|l| l.get(0)).cloned().unwrap_or(0);
        let last_anim_frame = lines.get(i + following_lines_amt + 1).and_then(|l| l.get(0)).cloned().unwrap_or(0);
        
        let duration = last_anim_frame - first_anim_frame;
        let repeats = std::cmp::max(line[2], 1); 

        let last_frame_used = (duration * repeats) + first_anim_frame;
        max_frame = std::cmp::max(last_frame_used, max_frame);
    }

    max_frame + 1 
}