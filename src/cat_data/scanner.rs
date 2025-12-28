use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use std::sync::mpsc::{self, Receiver};
use image::GenericImageView; 

#[derive(Clone, Debug)]
pub struct CatEntry {
    pub id: u32,
    pub image_path: PathBuf,
    #[allow(dead_code)]
    pub names: Vec<String>, 
}

pub fn start_scan() -> Receiver<CatEntry> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let cats_dir = Path::new("game/cats");
        
        if let Ok(entries) = fs::read_dir(cats_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_dir() {
                    if let Some(stem) = path.file_name().and_then(|s| s.to_str()) {
                        if let Ok(id) = stem.parse::<u32>() {
                            let filename = format!("udi{:03}_f.png", id);
                            let img_path = path.join("f").join(&filename);

                            if img_path.exists() {
                                if let Ok(img) = image::open(&img_path) {
                                    
                                    // 1. Safety Dimensions Check
                                    let (w, h) = img.dimensions();
                                    if w <= 14 || h <= 2 { continue; }

                                    // 2. Imposter Check (Units > 25)
                                    if id > 25 {
                                        let p = img.get_pixel(14, 2);
                                        if p[3] == 0 { continue; }
                                    }

                                    // 3. Content Check
                                    let has_content = img.pixels().any(|(_, _, pixel)| pixel[3] > 0);

                                    if has_content {
                                        let mut names = Vec::new();

                                        // --- SMART FILE FINDER ---
                                        // Checks for ID+1 (due to sort logic) and ID, both padded and unpadded.
                                        let file_id = id + 1;

                                        let p1 = path.join(format!("Unit_Explanation{}_en.csv", file_id));
                                        let p2 = path.join(format!("Unit_Explanation{:03}_en.csv", file_id));
                                        let p3 = path.join(format!("Unit_Explanation{}_en.csv", id));

                                        // Fallback to resLocal
                                        let res_local = path.parent().unwrap().parent().unwrap().join("resLocal");
                                        let p4 = res_local.join(format!("Unit_Explanation{}_en.csv", file_id));

                                        let target_csv = if p1.exists() { Some(p1) }
                                            else if p2.exists() { Some(p2) }
                                            else if p3.exists() { Some(p3) }
                                            else if p4.exists() { Some(p4) }
                                            else { None };

                                        if let Some(csv_path) = target_csv {
                                            if let Ok(bytes) = fs::read(&csv_path) {
                                                let content = String::from_utf8_lossy(&bytes);
                                                
                                                for line in content.lines().take(4) {
                                                    if let Some(name_part) = line.split('|').next() {
                                                        let trimmed = name_part.trim();
                                                        if !trimmed.is_empty() {
                                                            names.push(trimmed.to_string());
                                                        }
                                                    }
                                                }
                                                names.sort();
                                                names.dedup();
                                            }
                                        }

                                        let entry = CatEntry { 
                                            id, 
                                            image_path: img_path,
                                            names 
                                        };
                                        let _ = tx.send(entry);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    rx
}