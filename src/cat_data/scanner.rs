use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use std::sync::mpsc::{self, Receiver};
use image::GenericImageView; 

#[derive(Clone, Debug)]
pub struct CatEntry {
    pub id: u32,
    pub image_path: PathBuf,
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
                                    
                                    // 1. Safety Check: Dimensions
                                    // Must be at least 15x3 so we don't crash reading pixel (14, 2)
                                    let (w, h) = img.dimensions();
                                    if w <= 14 || h <= 2 { continue; }

                                    // --- THE COORDINATE CHECK ---
                                    // Rule: Units > 25 must have the card background starting at (14, 2).
                                    // If pixel (14, 2) is transparent, the background is missing -> Imposter.
                                    if id > 25 {
                                        let p = img.get_pixel(14, 2);
                                        // p[3] is Alpha. 
                                        if p[3] == 0 {
                                            continue; // Skip Imposter!
                                        }
                                    }

                                    // 2. Content Check (Backup for IDs <= 25)
                                    let has_content = img.pixels().any(|(_, _, pixel)| pixel[3] > 0);

                                    if has_content {
                                        let entry = CatEntry { id, image_path: img_path };
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