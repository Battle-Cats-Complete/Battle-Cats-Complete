use std::path::Path;

use iced::Task;
use tracing::{debug, trace};

use core::common::formats::SpriteSheet as CoreSpriteSheet;
use core::common::io;
use core::common::resolver;
use core::modules::settings::Settings;

use super::SpriteSheet;

pub fn ensure_loaded(sheets: &mut Vec<SpriteSheet>, settings: &Settings) -> Task<(usize, Option<CoreSpriteSheet>)> {
    let base_dir = io::img022_folder(Path::new(""));

    let png_paths = resolver::get(&base_dir, ["img022.png"], &settings.general.language_priority);
    let cut_paths = resolver::get(&base_dir, ["img022.imgcut"], &settings.general.language_priority);

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
