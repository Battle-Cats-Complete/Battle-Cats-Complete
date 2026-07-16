use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{mpsc::{self, Receiver}, Arc, Mutex};
use std::thread;

use nyanko::graphics::rig::{SpriteCut, SpriteSheet as NyankoSpriteSheet};

#[derive(Default)]
pub struct SpriteSheet {
    pub image_data: Option<Arc<image::RgbaImage>>,
    pub cuts_map: HashMap<usize, SpriteCut>,
    pub is_loading_active: bool,
    pub data_receiver: Option<Mutex<Receiver<(String, NyankoSpriteSheet)>>>,
    pub sheet_name: String,
}

impl Clone for SpriteSheet {
    fn clone(&self) -> Self {
        Self {
            image_data: self.image_data.clone(),
            cuts_map: self.cuts_map.clone(),
            is_loading_active: self.is_loading_active,
            data_receiver: None,
            sheet_name: self.sheet_name.clone(),
        }
    }
}

impl SpriteSheet {
    #[allow(dead_code)]
    pub fn is_ready(&self) -> bool {
        self.image_data.is_some()
    }

    pub fn load(&mut self, png_path: &Path, imgcut_path: &Path, id_str: String) {
        if self.is_loading_active { return; }

        self.is_loading_active = true;
        let png_path_buf = png_path.to_path_buf();
        let cut_path_buf = imgcut_path.to_path_buf();

        let (sender, receiver) = mpsc::channel();
        self.data_receiver = Some(Mutex::new(receiver));

        thread::spawn(move || {
            let png_data = fs::read(&png_path_buf).unwrap_or_default();
            let cut_data = fs::read(&cut_path_buf).unwrap_or_default();

            if let Some(parsed_sheet) = NyankoSpriteSheet::parse(&png_data, &cut_data) {
                let _ = sender.send((id_str, parsed_sheet));
            }
        });
    }

    pub fn update(&mut self) {
        if let Some(mutex) = &self.data_receiver
            && let Ok(receiver) = mutex.try_lock()
            && let Ok((name, parsed_sheet)) = receiver.try_recv() {
            self.sheet_name = name;
            self.image_data = parsed_sheet.image_data;
            self.cuts_map = parsed_sheet.cuts_map;
            self.is_loading_active = false;
        }

        if !self.is_loading_active && self.data_receiver.is_some() {
            self.data_receiver = None;
        }
    }
}