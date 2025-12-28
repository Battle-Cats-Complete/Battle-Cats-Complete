use eframe::egui;
use std::path::Path;
use image::imageops; 
use super::scanner::CatEntry;

pub fn show(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    cat: &CatEntry, 
    current_form: &mut usize,
    texture_cache: &mut Option<egui::TextureHandle>,
    current_key: &mut String
) {
    ui.vertical(|ui| {
        
        // --- 1. Form Tabs ---
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0; 
            ui.horizontal(|ui| {
                let form_labels = ["Normal", "Evolved", "True", "Ultra"];
                for (index, &exists) in cat.forms.iter().enumerate() {
                    if exists {
                        let label = form_labels[index];
                        let is_selected = *current_form == index;
                        let (fill, stroke, text) = if is_selected {
                            (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                        } else {
                            (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                        };
                        let btn = egui::Button::new(egui::RichText::new(label).color(text))
                            .fill(fill).stroke(stroke).rounding(egui::Rounding::ZERO).min_size(egui::vec2(60.0, 30.0));
                        if ui.add(btn).clicked() { *current_form = index; }
                    }
                }
            });
        });

        ui.separator(); 
        ui.add_space(5.0);

        // --- 2. HEADER: Icon + Name + ID ---
        ui.horizontal(|ui| {
            // A. CONSTRUCT PATH
            let form_char = match *current_form {
                0 => "f",
                1 => "c",
                2 => "s",
                _ => "u", 
            };
            
            let expected_path_str = format!(
                "game/cats/{:03}/{}/uni{:03}_{}00.png", 
                cat.id, form_char, cat.id, form_char
            );

            // B. LAZY LOAD TEXTURE (+ AUTO CROP)
            if *current_key != expected_path_str {
                *current_key = expected_path_str.clone(); 
                *texture_cache = None; 

                let path = Path::new(&expected_path_str);
                if path.exists() {
                    if let Ok(img) = image::open(path) {
                        
                        let mut rgba = img.to_rgba8();
                        rgba = autocrop(rgba);

                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.as_flat_samples();
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                        
                        *texture_cache = Some(ctx.load_texture(
                            "detail_icon",
                            color_image,
                            egui::TextureOptions::LINEAR
                        ));
                    }
                }
            }

            if let Some(tex) = texture_cache {
                ui.image(&*tex);
            } else {
                ui.allocate_space(egui::vec2(64.0, 64.0));
            }
            // Horizontal pading
            ui.add_space(1.0); 
            ui.vertical(|ui| {
                // Vertical padding
                ui.add_space(9.0); 

                let form_number = *current_form + 1;
                let raw_name = cat.names.get(*current_form).cloned().unwrap_or_default();
                let mut use_fallback = raw_name.is_empty();
                
                if *current_form > 0 && !use_fallback {
                    let prev_name = cat.names.get(*current_form - 1).cloned().unwrap_or_default();
                    if raw_name == prev_name { use_fallback = true; }
                }
                
                let display_name = if use_fallback {
                    format!("{:03}-{}", cat.id, form_number)
                } else {
                    raw_name
                };

                ui.heading(display_name);

                ui.label(
                    egui::RichText::new(format!("ID: {:03}-{}", cat.id, form_number))
                        .color(egui::Color32::from_gray(100)) 
                        .size(12.0) 
                );
            });
        });
    });
}

fn autocrop(img: image::RgbaImage) -> image::RgbaImage {
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