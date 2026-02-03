use eframe::egui;
use std::path::{Path};
use std::fs;
use std::thread;
use std::sync::mpsc::{self, Receiver};
use std::collections::HashMap;
use crate::core::utils;

#[derive(Clone, Debug)]
pub struct SpriteCut {
    pub uv_coordinates: egui::Rect,
    pub original_size: egui::Vec2,
    pub name: String,
}

pub struct SpriteSheet {
    pub texture_handle: Option<egui::TextureHandle>,
    pub cuts_map: HashMap<usize, SpriteCut>, 
    pub is_loading_active: bool,
    pub data_receiver: Option<Receiver<(String, egui::ColorImage, HashMap<usize, SpriteCut>)>>,
    pub sheet_name: String, 
}

impl Default for SpriteSheet {
    fn default() -> Self {
        Self {
            texture_handle: None,
            cuts_map: HashMap::new(),
            is_loading_active: false,
            data_receiver: None,
            sheet_name: String::new(),
        }
    }
}

impl SpriteSheet {
    pub fn is_loading(&self) -> bool {
        self.is_loading_active
    }
    
    pub fn is_ready(&self) -> bool {
        self.texture_handle.is_some()
    }

    /// Spawns a thread to load PNG + ImgCut
    pub fn load(&mut self, ctx: &egui::Context, png_path: &Path, imgcut_path: &Path, id_str: String) {
        if self.is_loading_active { return; }
        
        self.is_loading_active = true;
        let ctx_clone = ctx.clone();
        let png_p = png_path.to_path_buf();
        let cut_p = imgcut_path.to_path_buf();
        
        let (tx, rx) = mpsc::channel();
        self.data_receiver = Some(rx);

        thread::spawn(move || {
            if let Some((img, cuts)) = Self::load_internal(&png_p, &cut_p) {
                let _ = tx.send((id_str, img, cuts));
                ctx_clone.request_repaint();
            }
        });
    }

    /// Receives loaded data from the background thread and creates the texture
    pub fn update(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.data_receiver {
            if let Ok((name, img, cuts)) = rx.try_recv() {
                self.sheet_name = name.clone(); 
                self.texture_handle = Some(ctx.load_texture(&name, img, Default::default()));
                self.cuts_map = cuts;
                self.is_loading_active = false;
                self.data_receiver = None;
            }
        }
    }

    /// Returns an Image widget for the specified line index.
    /// Note: This returns the image at its natural size. Use .fit_to_exact_size() in the UI.
    pub fn get_sprite_by_line(&self, index: usize) -> Option<egui::Image<'_>> {
        let tex = self.texture_handle.as_ref()?;
        let cut = self.cuts_map.get(&index)?;
        
        Some(egui::Image::new(tex).uv(cut.uv_coordinates))
    }

    // --- INTERNAL PARSING LOGIC ---
    fn load_internal(png_path: &Path, cut_path: &Path) -> Option<(egui::ColorImage, HashMap<usize, SpriteCut>)> {
        // 1. Load Image
        let image_data = fs::read(png_path).ok()?;
        let image = image::load_from_memory(&image_data).ok()?;
        let size = [image.width() as usize, image.height() as usize];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.as_flat_samples();
        let egui_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

        // 2. Load ImgCut Text
        let content = fs::read_to_string(cut_path).ok()?;
        let delimiter = utils::detect_csv_separator(&content);
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

        // 3. Find Header & Count
        let mut sprite_count = 0;
        let mut data_start_index = 0;
        let mut found_header = false;

        for (i, line) in lines.iter().enumerate() {
            if !line.contains(',') {
                if let Ok(val) = line.trim().parse::<usize>() {
                    if val > 0 && val < 5000 {
                        sprite_count = val;
                        data_start_index = i + 1;
                        found_header = true;
                    }
                }
            } else if found_header { 
                break; 
            }
        }

        if !found_header || sprite_count == 0 {
            data_start_index = 0;
            sprite_count = lines.len();
        }

        // 4. Parse Cuts
        let w = size[0] as f32;
        let h = size[1] as f32;
        let mut parsed_cuts = HashMap::new();
        let mut current_id = 0;

        for i in 0..sprite_count {
            let line_idx = data_start_index + i;
            if line_idx >= lines.len() { break; }
            
            let line = lines[line_idx];
            let p: Vec<&str> = line.split(delimiter).collect();
            
            if p.len() >= 4 {
                if let (Ok(x), Ok(y), Ok(cw), Ok(ch)) = (
                    p[0].trim().parse::<f32>(),
                    p[1].trim().parse::<f32>(),
                    p[2].trim().parse::<f32>(),
                    p[3].trim().parse::<f32>(),
                ) {
                    let uv_min = egui::pos2(x / w, y / h);
                    let uv_max = egui::pos2((x + cw) / w, (y + ch) / h);
                    
                    let name = if p.len() > 4 { p[4].to_string() } else { String::new() };

                    parsed_cuts.insert(current_id, SpriteCut {
                        uv_coordinates: egui::Rect::from_min_max(uv_min, uv_max),
                        original_size: egui::vec2(cw, ch),
                        name,
                    });
                    current_id += 1;
                }
            }
        }

        Some((egui_image, parsed_cuts))
    }
}