#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod modules;
mod common;

use std::panic;
use std::fs;

use eframe::egui;

fn main() -> eframe::Result<()> {
    panic::set_hook(Box::new(|panic_info| {
        let msg = format!("Battle Cats Complete crashed!\n{}\n", panic_info);
        let _ = fs::write("crash.txt", msg);
    }));

    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Battle Cats Complete")
            .with_drag_and_drop(true)
            .with_icon(icon)
            .with_app_id("battle_cats_complete"),
        multisampling: 0,
        ..Default::default()
    };

    eframe::run_native(
        "Battle Cats Complete",
        options,
        Box::new(|cc| Ok(Box::new(app::BattleCatsApp::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(core::common::assets::ICON)
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