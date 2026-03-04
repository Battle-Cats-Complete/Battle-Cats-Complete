#![allow(dead_code)]
use eframe::egui;
use crate::core::settings::Settings;
use super::imgcut::SpriteSheet;
use crate::paths::global;

pub const ICON_NP_COST: usize = 97; 

pub fn ensure_loaded(ctx: &egui::Context, sheet: &mut SpriteSheet, settings: &Settings) {
    sheet.update(ctx);

    if settings.game_language == "--" {
        return; 
    }

    if sheet.texture_handle.is_some() || sheet.is_loading_active {
        return;
    }

    let base_dir = global::img022_folder(std::path::Path::new(""));
    let current_language = &settings.game_language;
    
    let codes_to_try: Vec<String> = if current_language.is_empty() {
        crate::core::utils::LANGUAGE_PRIORITY
            .iter()
            .map(|language_code| language_code.to_string())
            .collect()
    } else {
        vec![current_language.clone()]
    };

    for code in codes_to_try {
        let (png_filename, imgcut_filename) = if code.is_empty() {
            ("img022.png".to_string(), "img022.imgcut".to_string())
        } else {
            (format!("img022_{}.png", code), format!("img022_{}.imgcut", code))
        };

        let png_path = base_dir.join(&png_filename);
        let imgcut_path = base_dir.join(&imgcut_filename);

        if png_path.exists() && imgcut_path.exists() {
            sheet.load(ctx, &png_path, &imgcut_path, format!("img022_{}", code));
            return;
        }
    }
    
    // Fallback to English (which we now know is the base APK version) if the regional one is missing
    let fallback_png = base_dir.join("img022_en.png");
    let fallback_cut = base_dir.join("img022_en.imgcut");
    
    if fallback_png.exists() && fallback_cut.exists() {
        sheet.load(ctx, &fallback_png, &fallback_cut, "img022_en".to_string());
    }
}