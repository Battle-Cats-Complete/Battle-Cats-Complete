use eframe::egui;
use std::path::Path;
use std::fs;

#[derive(Clone)]
pub struct SpriteCut {
    pub uv: egui::Rect,
    pub original_size: egui::Vec2,
    pub line_num: usize,
}

pub struct SpriteSheet {
    pub texture: Option<egui::TextureHandle>,
    pub cuts: Vec<SpriteCut>, 
    pub loaded: bool,
}

impl Default for SpriteSheet {
    fn default() -> Self {
        Self {
            texture: None,
            cuts: Vec::new(),
            loaded: false,
        }
    }
}

impl SpriteSheet {
    pub fn load(&mut self, ctx: &egui::Context, image_path: &Path, cut_path: &Path) {
        if self.loaded { return; }

        let image = match image::open(image_path) {
            Ok(img) => img.to_rgba8(),
            Err(_) => return, 
        };

        let w_img = image.width() as f32;
        let h_img = image.height() as f32;

        self.texture = Some(ctx.load_texture(
            "img015_atlas",
            egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_flat_samples().as_slice()
            ),
            egui::TextureOptions::LINEAR
        ));

        if let Ok(content) = fs::read_to_string(cut_path) {
            self.cuts.clear();
            for (i, line) in content.lines().enumerate() {
                let parts: Vec<&str> = line.split(',').collect();
                
                // Format: x, y, w, h
                if parts.len() >= 4 {
                    if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                        parts[0].trim().parse::<f32>(),
                        parts[1].trim().parse::<f32>(),
                        parts[2].trim().parse::<f32>(), 
                        parts[3].trim().parse::<f32>(), 
                    ) {
                        let min = egui::pos2(x / w_img, y / h_img);
                        let max = egui::pos2((x + w) / w_img, (y + h) / h_img);
                        
                        self.cuts.push(SpriteCut {
                            uv: egui::Rect::from_min_max(min, max),
                            original_size: egui::vec2(w, h),
                            line_num: i + 1,
                        });
                    }
                }
            }
        }
        self.loaded = true;
    }

    pub fn get_sprite_by_line(&self, target_line: usize) -> Option<egui::Image<'_>> {
        if let Some(tex) = &self.texture {
            if let Some(cut) = self.cuts.iter().find(|c| c.line_num == target_line) {
                return Some(
                    egui::Image::new(tex)
                        .uv(cut.uv)
                        // Disable aspect ratio logic so it doesn't turn into a rectangle
                        .maintain_aspect_ratio(false)
                        .fit_to_exact_size(cut.original_size)
                );
            }
        }
        None
    }
}