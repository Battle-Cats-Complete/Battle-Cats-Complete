use iced::Task;

use core::common::formats::SpriteSheet as CoreSpriteSheet;
use core::Vfs;

use super::SpriteSheet;

pub fn ensure_loaded(sheets: &mut Vec<SpriteSheet>, vfs: &Vfs) -> Task<(usize, Option<CoreSpriteSheet>)> {
    super::ensure_sheet_loaded(sheets, vfs, "img015")
}
