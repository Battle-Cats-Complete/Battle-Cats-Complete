use std::borrow::Cow;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use image::{RgbaImage, Rgba};
use ab_glyph::{FontRef, PxScale};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut, text_size};
use imageproc::rect::Rect;
use arboard::{Clipboard, ImageData};
use eframe::egui;

use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::logic::stats::CatRaw;
use crate::features::cat::paths::{self, AssetType};
use crate::core::utils::autocrop;
use crate::global::imgcut::SpriteCut;
use crate::features::cat::logic::abilities::{collect_ability_data, CustomIcon, AbilityItem};
use crate::features::settings::logic::Settings;

fn get_icon_image(
    item: &AbilityItem, 
    cuts_map: &HashMap<usize, SpriteCut>,
    img015_base: &RgbaImage,
    multihit_base: &RgbaImage,
    kamikaze_base: &RgbaImage,
    bosswave_base: &RgbaImage,
    export_icon_size: u32,
) -> RgbaImage {
    let mut icon = match item.custom_icon {
        CustomIcon::Multihit => multihit_base.clone(),
        CustomIcon::Kamikaze => kamikaze_base.clone(),
        CustomIcon::BossWave => bosswave_base.clone(),
        CustomIcon::None => {
            if let Some(cut) = cuts_map.get(&item.icon_id) {
                let w = img015_base.width() as f32;
                let h = img015_base.height() as f32;
                
                let px = (cut.uv_coordinates.min.x * w).round() as u32;
                let py = (cut.uv_coordinates.min.y * h).round() as u32;
                let pw = cut.original_size.x.round() as u32;
                let ph = cut.original_size.y.round() as u32;
                
                if px + pw <= img015_base.width() && py + ph <= img015_base.height() {
                    image::imageops::crop_imm(img015_base, px, py, pw, ph).to_image()
                } else {
                    RgbaImage::new(export_icon_size, export_icon_size)
                }
            } else {
                RgbaImage::new(export_icon_size, export_icon_size)
            }
        }
    };

    if let Some(border_id) = item.border_id {
        if let Some(cut) = cuts_map.get(&border_id) {
            let w = img015_base.width() as f32;
            let h = img015_base.height() as f32;
            let px = (cut.uv_coordinates.min.x * w).round() as u32;
            let py = (cut.uv_coordinates.min.y * h).round() as u32;
            let pw = cut.original_size.x.round() as u32;
            let ph = cut.original_size.y.round() as u32;
            
            if px + pw <= img015_base.width() && py + ph <= img015_base.height() {
                let border = image::imageops::crop_imm(img015_base, px, py, pw, ph).to_image();
                image::imageops::overlay(&mut icon, &border, 0, 0);
            }
        }
    }
    
    if icon.width() != export_icon_size || icon.height() != export_icon_size {
        icon = image::imageops::resize(&icon, export_icon_size, export_icon_size, image::imageops::FilterType::Lanczos3);
    }
    icon
}

fn wrap_text(text: &str, font: &impl ab_glyph::Font, scale: PxScale, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current_line = String::new();
        let mut current_word = String::new();
        
        for c in paragraph.chars() {
            let is_cjk = (c >= '\u{4E00}' && c <= '\u{9FFF}') || 
                         (c >= '\u{3040}' && c <= '\u{30FF}') || 
                         (c >= '\u{AC00}' && c <= '\u{D7AF}');
                         
            if c.is_whitespace() || is_cjk {
                if !current_word.is_empty() {
                    let sep = if current_line.is_empty() { "" } else { " " };
                    let test_line = format!("{}{}{}", current_line, sep, current_word);
                    let (w, _) = text_size(scale, font, &test_line);
                    
                    if w as f32 > max_width {
                        if !current_line.is_empty() {
                            lines.push(current_line.clone());
                            current_line = current_word.clone();
                        } else {
                            lines.push(current_word.clone());
                            current_line.clear();
                        }
                    } else {
                        current_line = test_line;
                    }
                    current_word.clear();
                }
                
                if is_cjk {
                    let test_line = if current_line.is_empty() { c.to_string() } else { format!("{}{}", current_line, c) };
                    let (w, _) = text_size(scale, font, &test_line);
                    if w as f32 > max_width {
                        if !current_line.is_empty() {
                            lines.push(current_line.clone());
                        }
                        current_line = c.to_string();
                    } else {
                        current_line = test_line;
                    }
                }
            } else {
                current_word.push(c);
            }
        }
        
        if !current_word.is_empty() {
            let sep = if current_line.is_empty() { "" } else { " " };
            let test_line = format!("{}{}{}", current_line, sep, current_word);
            let (w, _) = text_size(scale, font, &test_line);
            if w as f32 > max_width {
                if !current_line.is_empty() {
                    lines.push(current_line.clone());
                    current_line = current_word;
                } else {
                    lines.push(current_word);
                    current_line.clear();
                }
            } else {
                current_line = test_line;
            }
        }
        if !current_line.is_empty() { lines.push(current_line); }
    }
    if lines.is_empty() { lines.push(String::new()); }
    lines
}

fn draw_centered_text(img: &mut RgbaImage, color: Rgba<u8>, rect: Rect, scale: PxScale, font: &impl ab_glyph::Font, text: &str) {
    let (tw, _) = text_size(scale, font, text);
    let visual_h = scale.y * 0.7; 
    let tx = rect.left() + (rect.width() as i32 - tw as i32) / 2;
    let ty = rect.top() + (rect.height() as i32 - visual_h as i32) / 2;
    draw_text_mut(img, color, tx.max(rect.left()), ty.max(rect.top()), scale, font, text);
}

fn draw_time_cell(img: &mut RgbaImage, bg: Rgba<u8>, rect: Rect, frames: i32, font: &impl ab_glyph::Font, scale_f: f32, scale_i: i32) {
    draw_filled_rect_mut(img, rect, bg);
    
    let sec = frames as f32 / 30.0;
    let sec_str = format!("{:.2}s", sec);
    let f_str = format!("({}f)", frames);
    
    let scale_sec = PxScale::from(15.0 * scale_f);
    let scale_f_text = PxScale::from(12.0 * scale_f); 
    
    let (sec_w, _) = text_size(scale_sec, font, &sec_str);
    let (f_w, _) = text_size(scale_f_text, font, &f_str);
    
    let gap = 3 * scale_i;
    let total_w = sec_w + f_w + gap as u32;
    
    let visual_h = scale_sec.y * 0.7;
    let start_x = rect.left() + (rect.width() as i32 - total_w as i32) / 2;
    let start_y = rect.top() + (rect.height() as i32 - visual_h as i32) / 2;
    
    draw_text_mut(img, Rgba([255, 255, 255, 255]), start_x, start_y, scale_sec, font, &sec_str);
    
    let f_visual_h = scale_f_text.y * 0.7;
    let f_y_offset = (visual_h - f_visual_h) / 2.0;
    
    draw_text_mut(img, Rgba([150, 150, 150, 255]), start_x + sec_w as i32 + gap, start_y + f_y_offset as i32, scale_f_text, font, &f_str);
}

fn build_statblock_image(
    language: &str,
    cat: &CatEntry,
    stats: &CatRaw,
    form: usize,
    level: i32,
    cuts_map: HashMap<usize, SpriteCut>,
    talent_levels: Option<HashMap<u8, u8>>
) -> RgbaImage {
    // --- SUPERSAMPLING CONSTANTS ---
    let scale: i32 = 2;
    let scale_f: f32 = 2.0;

    let padding = 8 * scale;
    let col_w = 70 * scale;
    let gap = 2 * scale;
    let export_icon_size = 40 * scale;
    let icon_gap_x = 2 * scale;
    let icon_gap_y = 2 * scale;
    let list_text_y_offset = 2 * scale;
    let list_text_gap_x = 8 * scale;
    
    let base_grid_width: f32 = (8.0 * 2.0) + (70.0 * 5.0) + (2.0 * 4.0); // 374 px (unscaled)

    let font_data: &[u8] = match language {
        "kr" => include_bytes!("../../../assets/NotoSansKR-Regular.ttf"),
        "tw" => include_bytes!("../../../assets/NotoSansTC-Regular.ttf"),
        "th" => include_bytes!("../../../assets/NotoSansThai-Regular.ttf"),
        _ => include_bytes!("../../../assets/NotoSansJP-Regular.ttf"), 
    };
    let font = FontRef::try_from_slice(font_data).expect("Failed to load font");

    let mut dummy_settings = Settings::default();
    dummy_settings.game_language = language.to_string();

    let (traits, h1, h2, b1, b2, footer) = collect_ability_data(
        stats, level, cat.curve.as_ref(), &dummy_settings, false, 
        cat.talent_data.as_ref(), talent_levels.as_ref()
    );

    // --- DYNAMIC WIDTH CALCULATION (Pre-pass) ---
    let mut max_needed_width: f32 = base_grid_width;

    let check_icon_row_width = |items: &Vec<AbilityItem>| -> f32 {
        if items.is_empty() { return 0.0; }
        8.0 + (items.len() as f32 * 42.0) - 2.0 + 8.0 
    };

    let check_list_width = |items: &Vec<AbilityItem>| -> f32 {
        let mut max_w = 0.0_f32;
        for item in items {
            let mut max_line_w = 0.0;
            // No wrapping! If it has a natural newline, split and measure.
            for line in item.text.split('\n') {
                let (tw, _) = text_size(PxScale::from(14.0), &font, line); 
                if tw as f32 > max_line_w { max_line_w = tw as f32; }
            }
            let w = 8.0 + 40.0 + 8.0 + max_line_w + 8.0; 
            if w > max_w { max_w = w; }
        }
        max_w
    };

    max_needed_width = max_needed_width.max(check_icon_row_width(&traits));
    max_needed_width = max_needed_width.max(check_icon_row_width(&h1));
    max_needed_width = max_needed_width.max(check_icon_row_width(&h2));
    max_needed_width = max_needed_width.max(check_icon_row_width(&footer));
    max_needed_width = max_needed_width.max(check_list_width(&b1));
    max_needed_width = max_needed_width.max(check_list_width(&b2));

    let canvas_width = (max_needed_width.ceil() as i32) * scale;
    
    let mut img = RgbaImage::new(canvas_width as u32, 4000 * scale as u32); 
    
    let bg_color = Rgba([33, 33, 33, 255]);
    let separator_color = Rgba([60, 60, 60, 255]);
    let text_white = Rgba([255, 255, 255, 255]);
    let text_weak = Rgba([150, 150, 150, 255]);
    let header_bg = Rgba([50, 50, 50, 255]);
    let data_bg = Rgba([40, 40, 40, 255]);

    draw_filled_rect_mut(&mut img, Rect::at(0, 0).of_size(canvas_width as u32, 4000 * scale as u32), bg_color);

    let img015_folder = crate::global::paths::img015_folder(Path::new(""));
    
    let codes_to_try: Vec<String> = if language == "--" || language.is_empty() {
        crate::core::utils::LANGUAGE_PRIORITY.iter().map(|s| s.to_string()).collect()
    } else {
        vec![language.to_string()]
    };

    let mut img015_base_opt = None;
    for code in codes_to_try {
        let png_filename = if code.is_empty() { "img015.png".to_string() } else { format!("img015_{}.png", code) };
        let full_png_path = img015_folder.join(&png_filename);
        if full_png_path.exists() {
            if let Ok(loaded) = image::open(&full_png_path) {
                img015_base_opt = Some(loaded.to_rgba8());
                break;
            }
        }
    }
    let img015_base = img015_base_opt.unwrap_or_else(|| RgbaImage::new(1024, 1024));
    
    let multihit_base = image::load_from_memory(include_bytes!("../../../assets/multihit.png")).unwrap().to_rgba8();
    let kamikaze_base = image::load_from_memory(include_bytes!("../../../assets/kamikaze.png")).unwrap().to_rgba8();
    let bosswave_base = image::load_from_memory(include_bytes!("../../../assets/boss_wave_immune.png")).unwrap().to_rgba8();

    // === HEADER ===
    let icon_path = paths::image(Path::new(paths::DIR_CATS), AssetType::Icon, cat.id, form, cat.egg_ids);
    if let Some(path) = icon_path {
        if let Ok(icon_img) = image::open(path) {
            let mut rgba = autocrop(icon_img.to_rgba8());
            let target_w = 110 * scale as u32;
            let target_h = 85 * scale as u32;
            if rgba.width() != target_w || rgba.height() != target_h {
                rgba = image::imageops::resize(&rgba, target_w, target_h, image::imageops::FilterType::Lanczos3);
            }
            image::imageops::overlay(&mut img, &rgba, padding as i64, padding as i64);
        }
    }

    let form_num = form + 1;
    let disp_name = if cat.names[form].is_empty() { format!("{:03}-{}", cat.id, form_num) } else { cat.names[form].clone() };

    let text_x = padding + 110 * scale + 12 * scale;
    
    // Tighter unscaled bounds so text wraps similar to UI box
    let max_name_width = 200.0 * scale_f;
    // Increased height to 48px to allow two full lines of 20px text without triggering the shrink mechanism!
    let name_box_height = 48.0 * scale_f; 
    
    let mut name_scale = 20.0;
    
    // WRAP FIRST: Wrap text freely at standard size
    let mut name_lines = wrap_text(&disp_name, &font, PxScale::from(name_scale * scale_f), max_name_width);
    let mut line_height = (name_scale * scale_f) as i32 + (2 * scale);
    let mut total_text_h = name_lines.len() as i32 * line_height;

    // SHRINK LATER: Only shrink if the wrapped lines burst out of the vertical container height
    while total_text_h > name_box_height as i32 && name_scale > 10.0 {
        name_scale -= 1.0;
        name_lines = wrap_text(&disp_name, &font, PxScale::from(name_scale * scale_f), max_name_width);
        line_height = (name_scale * scale_f) as i32 + (2 * scale);
        total_text_h = name_lines.len() as i32 * line_height;
    }

    let base_box_y = padding + 8 * scale;
    let mut current_name_y = base_box_y + ((name_box_height as i32 - total_text_h) / 2).max(0);

    for line in &name_lines {
        draw_text_mut(&mut img, text_white, text_x, current_name_y, PxScale::from(name_scale * scale_f), &font, line);
        current_name_y += line_height;
    }

    let final_id_y = padding + 52 * scale;
    let final_level_y = padding + 70 * scale;

    draw_text_mut(&mut img, text_weak, text_x, final_id_y, PxScale::from(14.0 * scale_f), &font, &format!("ID: {:03}-{}", cat.id, form_num));
    draw_text_mut(&mut img, text_white, text_x, final_level_y, PxScale::from(16.0 * scale_f), &font, &format!("Level: {}", level));

    let mut cursor_y = std::cmp::max(padding + 85 * scale, final_level_y + 18 * scale) + 12 * scale;
    draw_filled_rect_mut(&mut img, Rect::at(padding, cursor_y).of_size(canvas_width as u32 - (padding * 2) as u32, 1 * scale as u32), separator_color);
    cursor_y += 10 * scale;


    // === STAT GRID ===
    let curve = cat.curve.as_ref();
    let hp = curve.map_or(stats.hitpoints, |c| c.calculate_stat(stats.hitpoints, level));
    let atk_1 = curve.map_or(stats.attack_1, |c| c.calculate_stat(stats.attack_1, level));
    let atk_2 = curve.map_or(stats.attack_2, |c| c.calculate_stat(stats.attack_2, level));
    let atk_3 = curve.map_or(stats.attack_3, |c| c.calculate_stat(stats.attack_3, level));
    let total_atk = atk_1 + atk_2 + atk_3;
    let cycle = stats.attack_cycle(cat.atk_anim_frames[form]);
    let dps = if cycle > 0 { (total_atk as f32 * 30.0 / cycle as f32) as i32 } else { 0 };
    let atk_type = if stats.area_attack == 0 { "Single" } else { "Area" };
    let cd_val = stats.effective_cooldown();
    
    let stat_headers_1 = ["Atk", "Dps", "Range", "Atk Cycle", "Atk Type"];
    let stat_data_1 = [
        total_atk.to_string(), 
        dps.to_string(), 
        stats.standing_range.to_string(), 
        "".to_string(), 
        atk_type.to_string()
    ];
    let stat_headers_2 = ["Hp", "Kb", "Speed", "Cooldown", "Cost"];
    let stat_data_2 = [
        hp.to_string(), 
        stats.knockbacks.to_string(), 
        stats.speed.to_string(), 
        "".to_string(), 
        format!("{}¢", stats.eoc1_cost * 3 / 2)
    ];

    let row_h = 24 * scale;

    for col in 0..5 {
        let x = padding + (col * (col_w + gap));
        
        let h1_rect = Rect::at(x, cursor_y).of_size(col_w as u32, row_h as u32);
        draw_filled_rect_mut(&mut img, h1_rect, header_bg);
        draw_centered_text(&mut img, text_white, h1_rect, PxScale::from(14.0 * scale_f), &font, stat_headers_1[col as usize]);
        
        let d1_rect = Rect::at(x, cursor_y + row_h).of_size(col_w as u32, row_h as u32);
        if col == 3 {
            draw_time_cell(&mut img, data_bg, d1_rect, cycle, &font, scale_f, scale);
        } else {
            draw_filled_rect_mut(&mut img, d1_rect, data_bg);
            draw_centered_text(&mut img, text_white, d1_rect, PxScale::from(15.0 * scale_f), &font, &stat_data_1[col as usize]);
        }
        
        let h2_rect = Rect::at(x, cursor_y + (row_h * 2) + gap).of_size(col_w as u32, row_h as u32);
        draw_filled_rect_mut(&mut img, h2_rect, header_bg);
        draw_centered_text(&mut img, text_white, h2_rect, PxScale::from(14.0 * scale_f), &font, stat_headers_2[col as usize]);
        
        let d2_rect = Rect::at(x, cursor_y + (row_h * 3) + gap).of_size(col_w as u32, row_h as u32);
        if col == 3 {
            draw_time_cell(&mut img, data_bg, d2_rect, cd_val, &font, scale_f, scale);
        } else {
            draw_filled_rect_mut(&mut img, d2_rect, data_bg);
            draw_centered_text(&mut img, text_white, d2_rect, PxScale::from(15.0 * scale_f), &font, &stat_data_2[col as usize]);
        }
    }

    cursor_y += (row_h * 4) + gap + 15 * scale;
    draw_filled_rect_mut(&mut img, Rect::at(padding, cursor_y).of_size(canvas_width as u32 - (padding * 2) as u32, 1 * scale as u32), separator_color);
    cursor_y += 10 * scale;


    // === ABILITIES ===
    let draw_icon_row = |img: &mut RgbaImage, items: &Vec<AbilityItem>, y: i32| -> i32 {
        if items.is_empty() { return y; }
        let mut x = padding;
        
        for item in items {
            let icon = get_icon_image(item, &cuts_map, &img015_base, &multihit_base, &kamikaze_base, &bosswave_base, export_icon_size as u32);
            image::imageops::overlay(img, &icon, x as i64, y as i64);
            x += export_icon_size + icon_gap_x; 
        }
        y + export_icon_size 
    };

    let draw_list = |img: &mut RgbaImage, items: &Vec<AbilityItem>, mut y: i32| -> i32 {
        if items.is_empty() { return y; }

        for (i, item) in items.iter().enumerate() {
            let icon = get_icon_image(item, &cuts_map, &img015_base, &multihit_base, &kamikaze_base, &bosswave_base, export_icon_size as u32);
            image::imageops::overlay(img, &icon, padding as i64, y as i64);
            
            // NO dynamic wrapping. Split on explicit localization line breaks \n
            let lines: Vec<&str> = item.text.split('\n').collect();
            let line_height = 18 * scale;
            let total_text_h = lines.len() as i32 * line_height;
            
            let mut text_y = y;
            if total_text_h < export_icon_size as i32 {
                text_y += (export_icon_size as i32 - total_text_h) / 2 + list_text_y_offset; 
            }

            for line in lines {
                draw_text_mut(img, text_white, padding + export_icon_size as i32 + list_text_gap_x, text_y, PxScale::from(14.0 * scale_f), &font, line);
                text_y += line_height;
            }
            
            y += (export_icon_size as i32).max(total_text_h);
            if i < items.len() - 1 {
                y += icon_gap_y;
            }
        }
        y 
    };

    let mut last_was_trait = false;
    let mut previously_drew = false;
    let section_gap = 5 * scale;

    if !traits.is_empty() { 
        cursor_y = draw_icon_row(&mut img, &traits, cursor_y); 
        last_was_trait = true; 
        previously_drew = true;
    }
    
    if !h1.is_empty() { 
        if previously_drew { cursor_y += if last_was_trait { section_gap } else { icon_gap_y }; }
        cursor_y = draw_icon_row(&mut img, &h1, cursor_y); 
        last_was_trait = false;
        previously_drew = true;
    }
    
    if !h2.is_empty() { 
        if previously_drew { cursor_y += if last_was_trait { section_gap } else { icon_gap_y }; }
        cursor_y = draw_icon_row(&mut img, &h2, cursor_y); 
        last_was_trait = false;
        previously_drew = true;
    }

    if !b1.is_empty() || !b2.is_empty() {
        if previously_drew { cursor_y += if last_was_trait { section_gap } else { icon_gap_y }; }
        
        if !b1.is_empty() {
            cursor_y = draw_list(&mut img, &b1, cursor_y);
        }
        if !b1.is_empty() && !b2.is_empty() { 
            cursor_y += icon_gap_y; 
        }
        if !b2.is_empty() {
            cursor_y = draw_list(&mut img, &b2, cursor_y);
        }
        
        last_was_trait = false;
        previously_drew = true;
    }

    if !footer.is_empty() { 
        if previously_drew { cursor_y += icon_gap_y; } 
        cursor_y = draw_icon_row(&mut img, &footer, cursor_y); 
    }

    // Tightly crop the massive scaled canvas
    let final_height = cursor_y + padding;
    let final_cropped = image::imageops::crop_imm(&img, 0, 0, canvas_width as u32, final_height as u32).to_image();
    
    // Mathematically downsample using Lanczos3 for perfect Anti-Aliased edges
    image::imageops::resize(
        &final_cropped, 
        final_cropped.width() / scale as u32, 
        final_cropped.height() / scale as u32, 
        image::imageops::FilterType::Lanczos3
    )
}

pub fn generate_and_copy_statblock(
    ctx: egui::Context, 
    language: String,
    cat: CatEntry,
    stats: CatRaw,
    form: usize,
    level: i32,
    cuts_map: HashMap<usize, SpriteCut>,
    talent_levels: Option<HashMap<u8, u8>>
) {
    std::thread::spawn(move || {
        let img = build_statblock_image(&language, &cat, &stats, form, level, cuts_map, talent_levels);
        
        let (width, height) = img.dimensions();
        let raw_pixels = img.into_raw();
        let img_data = ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(raw_pixels),
        };

        let success = match Clipboard::new() {
            Ok(mut clipboard) => clipboard.set_image(img_data).is_ok(),
            Err(_) => false,
        };

        let current_time = ctx.input(|i| i.time);
        
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("export_copy_time"), current_time);
            d.insert_temp(egui::Id::new("export_copy_res"), success);
            d.insert_temp(egui::Id::new("is_copying"), false);
        });
        ctx.request_repaint();

        let ctx_clone = ctx.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs_f32(2.1));
            ctx_clone.request_repaint();
        });
    });
}

pub fn generate_and_save_statblock(
    ctx: egui::Context, 
    language: String,
    cat: CatEntry,
    stats: CatRaw,
    form: usize,
    level: i32,
    cuts_map: HashMap<usize, SpriteCut>,
    talent_levels: Option<HashMap<u8, u8>>
) {
    std::thread::spawn(move || {
        let img = build_statblock_image(&language, &cat, &stats, form, level, cuts_map, talent_levels);
        
        let export_dir = Path::new("exports");
        let mut success = true;

        if !export_dir.exists() {
            if fs::create_dir_all(export_dir).is_err() {
                success = false;
            }
        }

        if success {
            let filename = export_dir.join(format!("{:03}-{}.statblock.png", cat.id, form + 1));
            success = img.save(filename).is_ok();
        }
        
        let current_time = ctx.input(|i| i.time);
        
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("export_save_time"), current_time);
            d.insert_temp(egui::Id::new("export_save_res"), success);
            d.insert_temp(egui::Id::new("is_exporting"), false);
        });
        ctx.request_repaint();

        let ctx_clone = ctx.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs_f32(2.1));
            ctx_clone.request_repaint();
        });
    });
}