use std::borrow::Cow;
use std::path::Path;
use std::fs;
use image::{RgbaImage, Rgba};
use ab_glyph::{FontRef, PxScale};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use arboard::{Clipboard, ImageData};
use eframe::egui;

fn build_statblock_image(language: &str) -> RgbaImage {
    let mut img = RgbaImage::new(600, 400);
    let bg_color = Rgba([30, 30, 30, 255]);
    draw_filled_rect_mut(&mut img, Rect::at(0, 0).of_size(600, 400), bg_color);

    let font_data: &[u8] = match language {
        "kr" => include_bytes!("../../../assets/NotoSansKR-Regular.ttf"),
        "tw" => include_bytes!("../../../assets/NotoSansTC-Regular.ttf"),
        "th" => include_bytes!("../../../assets/NotoSansThai-Regular.ttf"),
        _ => include_bytes!("../../../assets/NotoSansJP-Regular.ttf"), 
    };

    let font = FontRef::try_from_slice(font_data).expect("Failed to load font");
    
    let text_color = Rgba([255, 255, 255, 255]);
    let scale = PxScale::from(40.0);
    draw_text_mut(&mut img, text_color, 20, 20, scale, &font, "Statblock Exporter is Working!");
    
    let scale_small = PxScale::from(24.0);
    draw_text_mut(&mut img, text_color, 20, 80, scale_small, &font, "Paste this in Discord or Paint!");

    img
}

pub fn generate_and_copy_statblock(ctx: egui::Context, language: String) {
    std::thread::spawn(move || {
        let img = build_statblock_image(&language);
        
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

        // We use an f64 directly synced with egui's UI input time!
        let current_time = ctx.input(|i| i.time);
        
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("export_copy_time"), current_time);
            d.insert_temp(egui::Id::new("export_copy_res"), success);
        });
        ctx.request_repaint();

        let ctx_clone = ctx.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs_f32(2.1));
            ctx_clone.request_repaint();
        });
    });
}

pub fn generate_and_save_statblock(ctx: egui::Context, language: String, cat_id: u32, form: usize) {
    std::thread::spawn(move || {
        let img = build_statblock_image(&language);
        
        let export_dir = Path::new("exports");
        let mut success = true;

        if !export_dir.exists() {
            if fs::create_dir_all(export_dir).is_err() {
                success = false;
            }
        }

        if success {
            let filename = export_dir.join(format!("{:03}-{}.statblock.png", cat_id, form + 1));
            success = img.save(filename).is_ok();
        }
        
        let current_time = ctx.input(|i| i.time);
        
        ctx.data_mut(|d| {
            d.insert_temp(egui::Id::new("export_save_time"), current_time);
            d.insert_temp(egui::Id::new("export_save_res"), success);
        });
        ctx.request_repaint();

        let ctx_clone = ctx.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs_f32(2.1));
            ctx_clone.request_repaint();
        });
    });
}