use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use std::sync::mpsc::{self, Receiver};
use image::GenericImageView; 
use super::stats::CatRaw; 

#[derive(Clone, Debug)]
pub struct CatEntry {
    pub id: u32,
    pub image_path: PathBuf,
    pub names: Vec<String>, 
    pub forms: [bool; 4],
    pub stats: Vec<Option<CatRaw>>, 
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
                            
                            let has_f = path.join("f").exists(); 
                            let has_c = path.join("c").exists();
                            let has_s = path.join("s").exists();
                            let has_u = path.join("u").exists();
                            let forms = [has_f, has_c, has_s, has_u];

                            let filename = format!("udi{:03}_f.png", id);
                            let img_path = path.join("f").join(&filename);

                            if img_path.exists() {
                                if let Ok(img) = image::open(&img_path) {
                                    
                                    // Safety Checks
                                    let (w, h) = img.dimensions();
                                    if w <= 14 || h <= 2 { continue; }
                                    if id > 25 {
                                        let p = img.get_pixel(14, 2);
                                        if p[3] == 0 { continue; }
                                    }
                                    let has_content = img.pixels().any(|(_, _, pixel)| pixel[3] > 0);

                                    if has_content {
                                        let mut names = vec![String::new(); 4];
                                        let file_id = id + 1;
                                        
                                        let p1 = path.join(format!("Unit_Explanation{}_en.csv", file_id));
                                        let p2 = path.join(format!("Unit_Explanation{:03}_en.csv", file_id));
                                        let p3 = path.join(format!("Unit_Explanation{}_en.csv", id));
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
                                                for (i, line) in content.lines().enumerate().take(4) {
                                                    if let Some(name_part) = line.split('|').next() {
                                                        names[i] = name_part.trim().to_string();
                                                    }
                                                }
                                            }
                                        }

                                        let mut stats = vec![None; 4];
                                        let stats_file_name = format!("unit{:03}.csv", file_id); 
                                        let stats_path = path.join(&stats_file_name);
                                        
                                        if stats_path.exists() {
                                            if let Ok(content) = fs::read_to_string(&stats_path) {
                                                for (i, line) in content.lines().enumerate().take(4) {
                                                    stats[i] = CatRaw::from_csv_line(line);
                                                }
                                            }
                                        }

                                        let entry = CatEntry { 
                                            id, 
                                            image_path: img_path, 
                                            names,
                                            forms,
                                            stats, 
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