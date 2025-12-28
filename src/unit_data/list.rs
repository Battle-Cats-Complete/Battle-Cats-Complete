use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use super::scanner::UnitEntry;

pub struct UnitList {
    texture_cache: HashMap<u32, egui::TextureHandle>,
    auto_scroll: bool,
}

impl Default for UnitList {
    fn default() -> Self {
        Self {
            texture_cache: HashMap::new(),
            auto_scroll: false,
        }
    }
}

impl UnitList {
    // --- ADDED THIS FUNCTION ---
    pub fn clear_cache(&mut self) {
        self.texture_cache.clear();
    }

    pub fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, units: &[UnitEntry], selected_id: &mut Option<u32>) {
        
        let row_height = 50.0; 
        let total_rows = units.len();

        let output = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(self.auto_scroll)
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                
                for index in row_range {
                    if let Some(unit) = units.get(index) {
                        
                        let texture = self.get_or_load_texture(ctx, unit.id, &unit.image_path);

                        if let Some(tex) = texture {
                            let size = tex.size_vec2();
                            let scale = 40.0 / size.y; 
                            let btn_size = size * scale;

                            let is_selected = Some(unit.id) == *selected_id;
                            
                            // --- FIXED: Wrapped arguments in Double Parentheses (( ... )) ---
                            let btn = egui::ImageButton::new((tex.id(), btn_size))
                                .selected(is_selected);

                            if ui.add(btn).clicked() {
                                *selected_id = Some(unit.id);
                            }
                        } else {
                            ui.allocate_space(egui::vec2(200.0, 40.0));
                        }
                    }
                }
            });

        let scroll_y = output.state.offset.y;
        let content_height = output.content_size.y;
        let view_height = output.inner_rect.height();
        let max_scroll = content_height - view_height;

        if max_scroll > 0.0 {
            let is_at_bottom = scroll_y >= (max_scroll - 10.0);
            self.auto_scroll = is_at_bottom;
        }
    }

    fn get_or_load_texture(&mut self, ctx: &egui::Context, id: u32, path: &PathBuf) -> Option<&egui::TextureHandle> {
        if self.texture_cache.contains_key(&id) {
            return self.texture_cache.get(&id);
        }

        if let Ok(image_buffer) = image::open(path) {
            let image_buffer = image_buffer.to_rgba8();
            let size = [image_buffer.width() as usize, image_buffer.height() as usize];
            let pixels = image_buffer.as_flat_samples();

            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                size,
                pixels.as_slice(),
            );

            let texture = ctx.load_texture(
                format!("unit_{}", id),
                color_image,
                egui::TextureOptions::LINEAR
            );

            self.texture_cache.insert(id, texture);
            return self.texture_cache.get(&id);
        }

        None
    }
}