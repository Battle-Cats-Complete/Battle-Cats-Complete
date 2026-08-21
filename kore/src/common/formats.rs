pub(crate) mod gatyaitembuy;
pub(crate) mod gatyaitemname;
pub mod imgcut;

use std::collections::HashMap;
use std::sync::Arc;

use image::RgbaImage;
use nyanko::graphics::rig::SpriteCut;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatyaItemBuy {
    pub rarity: i32,
    pub reflect_or_storage: i32,
    pub price: i32,
    pub stage_drop_item_id: u32,
    pub quantity: i32,
    pub server_id: i32,
    pub category: i32,
    pub index: i32,
    pub src_item_id: i32,
    pub main_menu_type: i32,
    pub gatya_ticket_id: i32,
    pub img_id: i32,
    pub comment: String,
    pub row_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatyaItemName {
    pub name: String,
    pub description: Vec<String>,
}

#[derive(Default, Clone)]
pub struct SpriteSheet {
    pub image_data: Option<Arc<RgbaImage>>,
    pub cuts_map: HashMap<usize, SpriteCut>,
    pub sheet_name: String,
}
