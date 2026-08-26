pub mod imgcut;

use std::collections::HashMap;
use std::sync::Arc;

use image::RgbaImage;
use nyanko::graphics::rig::SpriteCut;

#[derive(Default, Clone)]
pub struct SpriteSheet {
    pub image_data: Option<Arc<RgbaImage>>,
    pub cuts_map: HashMap<usize, SpriteCut>,
    pub sheet_name: String,
}
