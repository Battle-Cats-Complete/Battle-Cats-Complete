use eframe::egui;
use std::path::Path;
use image::imageops; 
use super::scanner::CatEntry;
use super::sprites::SpriteSheet; 

// Trait Icons
const TRAIT_ICONS: &[usize] = &[
    224, // Red
    225, // Floating
    226, // Black
    227, // Metal
    228, // Angel
    229, // Alien
    230, // Zombie
    231, // Relic
    299, // Aku
    232  // Traitless
];

pub fn show(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    cat: &CatEntry, 
    current_form: &mut usize,
    texture_cache: &mut Option<egui::TextureHandle>,
    current_key: &mut String,
    sprite_sheet: &mut SpriteSheet 
) {
    let base_dir = Path::new("game/assets");
    let tex_en = base_dir.join("img015_en.png");
    let tex_ja = base_dir.join("img015_ja.png");
    let tex_raw = base_dir.join("img015.png");
    
    let texture_path = if tex_en.exists() { tex_en } 
        else if tex_ja.exists() { tex_ja } 
        else { tex_raw };

    let cut_path = base_dir.join("img015.imgcut");

    sprite_sheet.load(ctx, &texture_path, &cut_path);

    ui.vertical(|ui| {
        // --- Form Tabs ---
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

        ui.horizontal(|ui| {
            // Deploy Icon
            let form_char = match *current_form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };
            let expected = format!("game/cats/{:03}/{}/uni{:03}_{}00.png", cat.id, form_char, cat.id, form_char);

            if *current_key != expected {
                *current_key = expected.clone(); 
                *texture_cache = None; 
                let p = Path::new(&expected);
                let f = Path::new("game/cats/uni.png");
                let load = if p.exists() { Some(p) } else if f.exists() { Some(f) } else { None };

                if let Some(path) = load {
                    if let Ok(img) = image::open(path) {
                        let mut rgba = img.to_rgba8();
                        rgba = autocrop(rgba);
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.as_flat_samples();
                        *texture_cache = Some(ctx.load_texture("detail_icon", egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()), egui::TextureOptions::LINEAR));
                    }
                }
            }

            if let Some(tex) = texture_cache { ui.image(&*tex); } 
            else { ui.allocate_space(egui::vec2(64.0, 64.0)); }

            ui.add_space(2.0);

            // Name & ID
            ui.vertical(|ui| {
                ui.add_space(4.0);
                
                let form_num = *current_form + 1;
                let raw_name = cat.names.get(*current_form).cloned().unwrap_or_default();
                let disp_name = if raw_name.is_empty() { format!("{:03}-{}", cat.id, form_num) } else { raw_name };

                ui.heading(disp_name);
                ui.label(egui::RichText::new(format!("ID: {:03}-{}", cat.id, form_num)).color(egui::Color32::from_gray(100)).size(12.0));
            });
        });

        ui.add_space(5.0); 

        // Target Traits
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0; 
            
            for &line_num in TRAIT_ICONS {
                if let Some(sprite) = sprite_sheet.get_sprite_by_line(line_num) {
                    ui.add(sprite.tint(egui::Color32::from_gray(77)));
                }
            }
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