use iced::Task;
use tracing::{debug, trace};

use core::common::formats::SpriteSheet as CoreSpriteSheet;
use core::Vfs;

use super::SpriteSheet;

pub fn ensure_loaded(sheets: &mut Vec<SpriteSheet>, vfs: &Vfs) -> Task<(usize, Option<CoreSpriteSheet>)> {
    let png_paths = vfs.list("img022.png");
    let cut_paths = vfs.list("img022.imgcut");

    if sheets.len() != png_paths.len() {
        debug!("Resizing img022 sheets matrix to match resolved paths ({})", png_paths.len());
        sheets.resize_with(png_paths.len(), SpriteSheet::default);
    }

    let mut tasks = Vec::new();

    for (i, (png_path, imgcut_path)) in png_paths.into_iter().zip(cut_paths).enumerate() {
        if sheets[i].texture_handle.is_some() || sheets[i].is_loading() || sheets[i].has_failed() {
            continue;
        }

        let key = png_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown_sheet".to_string());

        trace!("Loading sprite sheet img022: {}", key);
        tasks.push(sheets[i].load(&png_path, &imgcut_path, key).map(move |result| (i, result)));
    }

    Task::batch(tasks)
}
