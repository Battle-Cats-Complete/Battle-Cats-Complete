use std::fs;
use std::path::Path;

use nyanko::graphics::rig::SpriteSheet as NyankoSpriteSheet;

pub fn parse(png_path: &Path, imgcut_path: &Path) -> Option<NyankoSpriteSheet> {
    let png_data = fs::read(png_path).unwrap_or_default();
    let cut_data = fs::read(imgcut_path).unwrap_or_default();
    NyankoSpriteSheet::parse(&png_data, &cut_data)
}
