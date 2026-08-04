use std::path::Path;

use iced::widget::image::Handle;
use image::imageops;

use core::common::gfx::autocrop;

pub fn load_scaled(path: &Path, max_size: u32) -> Option<Handle> {
    let raw = image::open(path).ok()?;
    let cropped = autocrop(raw.to_rgba8());
    let (width, height) = cropped.dimensions();
    if width == 0 || height == 0 {
        return None;
    }

    let max_dim = width.max(height) as f32;
    let scale = max_size as f32 / max_dim;
    let target_width = ((width as f32 * scale).round() as u32).max(1);
    let target_height = ((height as f32 * scale).round() as u32).max(1);

    let resized = imageops::resize(&cropped, target_width, target_height, imageops::FilterType::Triangle);
    Some(Handle::from_rgba(resized.width(), resized.height(), resized.into_raw()))
}

pub fn load_boxed(path: &Path, canvas: u32) -> Option<Handle> {
    let raw = image::open(path).ok()?;
    let cropped = autocrop(raw.to_rgba8());
    let (width, height) = cropped.dimensions();
    if width == 0 || height == 0 {
        return None;
    }

    let fitted = if width > canvas || height > canvas {
        let scale = canvas as f32 / width.max(height) as f32;
        let target_width = ((width as f32 * scale).round() as u32).max(1).min(canvas);
        let target_height = ((height as f32 * scale).round() as u32).max(1).min(canvas);
        imageops::resize(&cropped, target_width, target_height, imageops::FilterType::Lanczos3)
    } else {
        cropped
    };

    let mut boxed = image::RgbaImage::new(canvas, canvas);
    let x = ((canvas - fitted.width()) / 2) as i64;
    let y = ((canvas - fitted.height()) / 2) as i64;
    imageops::overlay(&mut boxed, &fitted, x, y);

    Some(Handle::from_rgba(canvas, canvas, boxed.into_raw()))
}

pub fn load_cropped(path: &Path) -> Option<(Handle, u32, u32)> {
    let raw = image::open(path).ok()?;
    if raw.width() <= 1 && raw.height() <= 1 {
        return None;
    }

    let cropped = autocrop(raw.to_rgba8());
    let (width, height) = cropped.dimensions();
    if width <= 1 && height <= 1 {
        return None;
    }

    let handle = Handle::from_rgba(width, height, cropped.into_raw());
    Some((handle, width, height))
}
