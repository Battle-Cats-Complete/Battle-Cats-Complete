use eframe::egui;
use std::collections::HashMap;
use std::path::{PathBuf};
use super::scanner::CatEntry;
use image::{imageops}; 

pub struct CatList {
    texture_cache: HashMap<u32, egui::TextureHandle>,
    bg_cache: Option<image::RgbaImage>,
    
    hovered_id: Option<egui::Id>, 
    hover_start_time: f64,
    

    hover_lost_time: Option<f64>,
}

impl Default for CatList {
    fn default() -> Self {
        Self {
            texture_cache: HashMap::new(),
            bg_cache: None,
            hovered_id: None,
            hover_start_time: 0.0,
            hover_lost_time: None,
        }
    }
}

impl CatList {
    pub fn clear_cache(&mut self) {
        self.texture_cache.clear();
        self.bg_cache = None;
        self.hovered_id = None;
        self.hover_lost_time = None;
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, units: &[CatEntry], selected_id: &mut Option<u32>, search_query: &str) {
        
        if self.bg_cache.is_none() {
            const BG_BYTES: &[u8] = include_bytes!("../../assets/udi_bg.png");
            if let Ok(bg) = image::load_from_memory(BG_BYTES) {
                self.bg_cache = Some(bg.to_rgba8());
            }
        }

        let query_lower = search_query.to_lowercase();
        let filtered_units: Vec<&CatEntry> = units.iter()
            .filter(|unit| {
                if search_query.is_empty() { return true; }
                let id_str = format!("{:03}", unit.id);
                if id_str.contains(search_query) { return true; }
                for name in &unit.names {
                    if name.to_lowercase().contains(&query_lower) { return true; }
                }
                false
            })
            .collect();

        let target_height = 50.0; 
        let padding = 5.0;
        let row_height = target_height + padding;
        let total_rows = filtered_units.len() + 1; 

        let now = ui.input(|i| i.time);

        let scroll_output = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                
                let mut hovered_this_frame = None;

                for index in row_range {
                    if let Some(&unit) = filtered_units.get(index) {
                        
                        let texture = self.get_or_load_texture(ctx, unit.id, &unit.image_path);

                        if let Some(tex) = texture {
                            let size = tex.size_vec2();
                            let scale = target_height / size.y;
                            let btn_size = size * scale;

                            let is_selected = Some(unit.id) == *selected_id;
                            
                            let btn = egui::ImageButton::new((tex.id(), btn_size))
                                .selected(is_selected);
                            
                            let mut response = ui.add(btn);

                            if response.hovered() {
                                hovered_this_frame = Some(response.id);

                                if self.hovered_id != Some(response.id) {
                                    self.hovered_id = Some(response.id);
                                    self.hover_start_time = now;
                                }

                                if now - self.hover_start_time > 1.0 {
                                    response = response.on_hover_ui(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("[ID]").weak());
                                            ui.label(format!("{:03}", unit.id));
                                        });

                                        let labels = ["Normal", "Evolved", "True", "Ultra"];
                                        let mut previous_name = "";

                                        for i in 0..4 {
                                            if unit.forms[i] {
                                                let raw_name = &unit.names[i];
                                                let display_name = if raw_name.is_empty() {
                                                    format!("{:03}-{}", unit.id, i + 1)
                                                } else {
                                                    raw_name.clone()
                                                };

                                                if i > 0 && raw_name == previous_name && !raw_name.is_empty() {
                                                    continue; 
                                                }

                                                if !raw_name.is_empty() {
                                                    previous_name = raw_name;
                                                }

                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(format!("[{}]", labels[i])).weak());
                                                    ui.label(display_name);
                                                });
                                            }
                                        }
                                    });
                                }
                            }

                            if response.clicked() {
                                *selected_id = Some(unit.id);
                            }

                        } else {
                            ui.allocate_space(egui::vec2(100.0, target_height));
                        }
                    }
                }
                
                hovered_this_frame
            });

        if scroll_output.inner.is_some() {
            self.hover_lost_time = None;
        } else {
            if self.hover_lost_time.is_none() {
                self.hover_lost_time = Some(now);
            }

            if let Some(lost_start) = self.hover_lost_time {
                if now - lost_start > 0.1 {
                    self.hovered_id = None;
                }
            }
        }
    }

    fn get_or_load_texture(&mut self, ctx: &egui::Context, id: u32, path: &PathBuf) -> Option<&egui::TextureHandle> {
        if self.texture_cache.contains_key(&id) {
            return self.texture_cache.get(&id);
        }

        if let Ok(image_buffer) = image::open(path) {
            let mut unit_img = image_buffer.to_rgba8();

            if self.bg_cache.is_none() {
                const BG_BYTES: &[u8] = include_bytes!("../../assets/udi_bg.png");
                if let Ok(bg) = image::load_from_memory(BG_BYTES) {
                    self.bg_cache = Some(bg.to_rgba8());
                }
            }

            if let Some(bg) = &self.bg_cache {
                let mut final_image = bg.clone();
                let bg_w = final_image.width() as i64;
                let bg_h = final_image.height() as i64;

                let (x, y) = if id <= 25 {
                    let fixed_x: i64 = -2; 
                    let fixed_y: i64 = 9;  
                    (fixed_x, fixed_y)
                } else {
                    unit_img = self.autocrop(unit_img);
                    let unit_w = unit_img.width() as i64;
                    let unit_h = unit_img.height() as i64;
                    let cx = (bg_w - unit_w) / 2;
                    let cy = (bg_h - unit_h) / 2;
                    (cx, cy)
                };

                imageops::overlay(&mut final_image, &unit_img, x, y);

                let size = [final_image.width() as usize, final_image.height() as usize];
                let pixels = final_image.as_flat_samples();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

                let texture = ctx.load_texture(
                    format!("unit_{}", id),
                    color_image,
                    egui::TextureOptions::LINEAR
                );
                self.texture_cache.insert(id, texture);
            }
            return self.texture_cache.get(&id);
        }
        None
    }

    fn autocrop(&self, img: image::RgbaImage) -> image::RgbaImage {
        let (width, height) = img.dimensions();
        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut found_pixel = false;

        for (x, y, pixel) in img.enumerate_pixels() {
            if pixel[3] > 0 { 
                if x < min_x { min_x = x; }
                if x > max_x { max_x = x; }
                if y < min_y { min_y = y; }
                if y > max_y { max_y = y; }
                found_pixel = true;
            }
        }
        if !found_pixel { return img; }
        let new_width = max_x - min_x + 1;
        let new_height = max_y - min_y + 1;
        imageops::crop_imm(&img, min_x, min_y, new_width, new_height).to_image()
    }
}