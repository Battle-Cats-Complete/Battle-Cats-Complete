use std::path::Path;

use eframe::egui;

use core::common::formats::imgcut::SpriteSheet as CoreSpriteSheet;

#[derive(Default, Clone)]
pub(crate) struct SpriteSheet {
    pub core: CoreSpriteSheet,
    pub texture_handle: Option<egui::TextureHandle>,
}

impl SpriteSheet {
    pub(crate) fn load(&mut self, png_path: &Path, imgcut_path: &Path, id_str: String) {
        self.core.load(png_path, imgcut_path, id_str);
    }

    pub(crate) fn update(&mut self, ctx: &egui::Context) {
        self.core.update();
        if self.texture_handle.is_none() && !self.core.is_loading_active
            && let Some(image_data) = &self.core.image_data {
            let size = [image_data.width() as usize, image_data.height() as usize];
            let pixels = image_data.as_flat_samples();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

            self.texture_handle = Some(ctx.load_texture(
                &self.core.sheet_name,
                color_image,
                egui::TextureOptions::LINEAR
            ));
        }
    }
}