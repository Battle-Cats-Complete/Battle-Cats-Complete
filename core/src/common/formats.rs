pub mod gatyaitembuy;
pub mod gatyaitemname;
pub mod imgcut;
pub mod mamodel;

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::sync::Mutex;

use image::RgbaImage;
use nyanko::graphics::rig::SpriteCut;
use nyanko::graphics::rig::SpriteSheet as NyankoSpriteSheet;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct GatyaItemName {
    pub name: String,
    pub description: Vec<String>,
}

#[derive(Default)]
pub struct SpriteSheet {
    pub image_data: Option<Arc<RgbaImage>>,
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
        imgcut::load_sheet(self, png_path, imgcut_path, id_str);
    }

    pub fn update(&mut self) {
        imgcut::update_sheet(self);
    }
}

#[derive(Clone, Debug)]
pub struct ModelPart {
    pub parent_id: i32,
    pub unit_id: i32,
    pub sprite_index: i32,
    pub drawing_layer: i32,
    pub position_x: f32,
    pub position_y: f32,
    pub pivot_x: f32,
    pub pivot_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation: f32,
    pub alpha: f32,
    pub glow_mode: i32,
    pub flip_x: bool,
    pub flip_y: bool,
    #[allow(dead_code)] pub name: String,
}

impl Default for ModelPart {
    fn default() -> Self {
        Self {
            parent_id: -1,
            unit_id: 0,
            sprite_index: 0,
            drawing_layer: 0,
            position_x: 0.0,
            position_y: 0.0,
            pivot_x: 0.0,
            pivot_y: 0.0,
            scale_x: 1000.0,
            scale_y: 1000.0,
            rotation: 0.0,
            alpha: 1000.0,
            glow_mode: 0,
            flip_x: false,
            flip_y: false,
            name: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Model {
    pub parts: Vec<ModelPart>,
    #[allow(dead_code)] pub version: u32,
    pub scale_unit: f32,
    pub angle_unit: f32,
    pub alpha_unit: f32,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            parts: Vec::new(),
            version: 0,
            scale_unit: 1000.0,
            angle_unit: 3600.0,
            alpha_unit: 1000.0,
        }
    }
}

impl Model {
    pub fn load(path: &Path) -> Option<Self> {
        mamodel::parse(path)
    }
}