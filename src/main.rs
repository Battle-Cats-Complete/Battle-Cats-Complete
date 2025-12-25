#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod main_menu;
mod import_data;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Battle Cats Complete")
            .with_drag_and_drop(true)
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "Battle Cats Complete",
        options,
        Box::new(|_cc| Ok(Box::new(app::BattleCatsApp::default()))),
    )
}

fn load_icon() -> egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(include_bytes!("../icon.ico"))
            .expect("Failed to open icon path")
            .into_rgba8();
        
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}