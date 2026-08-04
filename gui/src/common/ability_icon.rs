use std::cell::RefCell;
use std::collections::HashMap;

use iced::widget::image::Handle;
use image::imageops;

use crate::common::SpriteSheet;

#[derive(Default)]
pub struct Cache {
    cache: RefCell<HashMap<usize, Handle>>,
}

impl Cache {
    pub fn handle(&self, icon_id: usize, sheets: &[SpriteSheet]) -> Option<Handle> {
        if let Some(cached) = self.cache.borrow().get(&icon_id) {
            return Some(cached.clone());
        }

        for sheet in sheets {
            let Some(cut) = sheet.core.cuts_map.get(&icon_id) else { continue; };
            let Some(image_data) = &sheet.core.image_data else { continue; };

            let width = image_data.width();
            let height = image_data.height();

            let px = (cut.uv_coordinates.min.x * width as f32).round() as u32;
            let py = (cut.uv_coordinates.min.y * height as f32).round() as u32;
            let pw = cut.original_size.x.round() as u32;
            let ph = cut.original_size.y.round() as u32;

            if pw == 0 || ph == 0 || px + pw > width || py + ph > height {
                continue;
            }

            let cropped = imageops::crop_imm(image_data.as_ref(), px, py, pw, ph).to_image();
            let handle = Handle::from_rgba(pw, ph, cropped.into_raw());
            self.cache.borrow_mut().insert(icon_id, handle.clone());
            return Some(handle);
        }

        None
    }
}
