use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;

use nyanko::graphics::rig::SpriteSheet as NyankoSpriteSheet;

use super::SpriteSheet;

pub fn load_sheet(sheet: &mut SpriteSheet, png_path: &Path, imgcut_path: &Path, id_str: String) {
    if sheet.is_loading_active { return; }

    sheet.is_loading_active = true;
    let png_path_buf = png_path.to_path_buf();
    let cut_path_buf = imgcut_path.to_path_buf();

    let (sender, receiver) = mpsc::channel();
    sheet.data_receiver = Some(Mutex::new(receiver));

    thread::spawn(move || {
        let png_data = fs::read(&png_path_buf).unwrap_or_default();
        let cut_data = fs::read(&cut_path_buf).unwrap_or_default();

        if let Some(parsed_sheet) = NyankoSpriteSheet::parse(&png_data, &cut_data) {
            let _ = sender.send((id_str, parsed_sheet));
        }
    });
}

pub fn update_sheet(sheet: &mut SpriteSheet) {
    if let Some(mutex) = &sheet.data_receiver
        && let Ok(receiver) = mutex.try_lock()
        && let Ok((name, parsed_sheet)) = receiver.try_recv() {
        sheet.sheet_name = name;
        sheet.image_data = parsed_sheet.image_data;
        sheet.cuts_map = parsed_sheet.cuts_map;
        sheet.is_loading_active = false;
    }

    if !sheet.is_loading_active && sheet.data_receiver.is_some() {
        sheet.data_receiver = None;
    }
}