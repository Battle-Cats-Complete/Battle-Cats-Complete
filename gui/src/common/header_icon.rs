use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use iced::widget::image::Handle;

use kore::common::gfx::autocrop;

#[derive(Clone)]
pub(crate) struct HeaderIcon {
    pub(crate) handle: Handle,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl HeaderIcon {
    pub(crate) fn dummy() -> Self {
        Self { handle: Handle::from_rgba(1, 1, vec![80, 80, 80, 255]), width: 1.0, height: 1.0 }
    }

    pub(crate) fn scale(&self, box_width: f32, box_height: f32) -> (f32, f32) {
        let scale = (box_width / self.width).min(box_height / self.height);

        (self.width * scale, self.height * scale)
    }
}

pub(crate) type Cache = RefCell<HashMap<PathBuf, HeaderIcon>>;

pub(crate) fn load(cache: &Cache, path: &PathBuf) -> Option<HeaderIcon> {
    if let Some(cached) = cache.borrow().get(path) {
        return Some(cached.clone());
    }

    if !path.exists() {
        return None;
    }

    let img = image::open(path).ok()?;
    let rgba = autocrop(img.to_rgba8());
    let (width, height) = rgba.dimensions();

    if width == 0 || height == 0 {
        return None;
    }

    let icon = HeaderIcon {
        handle: Handle::from_rgba(width, height, rgba.into_raw()),
        width: width as f32,
        height: height as f32,
    };

    cache.borrow_mut().insert(path.clone(), icon.clone());

    Some(icon)
}
