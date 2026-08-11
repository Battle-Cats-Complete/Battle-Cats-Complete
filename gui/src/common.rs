pub mod ability_icon;
pub mod feedback;
pub mod fonts;
pub mod img015;
pub mod img022;
pub mod item_icon;
pub mod row_window;
pub mod udi_loader;
pub mod watcher;

use std::path::Path;
use std::thread;

use iced::futures::channel::mpsc::unbounded;
use iced::widget::image::Handle;
use iced::Task;
use tracing::{debug, error, info, trace, warn};

use core::common::assets::{
    BOSS_WAVE, BURROW, DEATH_TIMER, DOJO, GOD, KAMIKAZE, MULTIHIT, REVIVE, STARRED_ALIEN, STOP,
    UNKNOWN,
};
use core::common::formats::{imgcut, SpriteSheet as CoreSpriteSheet};
use core::common::game::CustomIcon;
use core::Vfs;

#[derive(Clone)]
pub struct CustomAssets {
    pub multihit: Handle,
    pub kamikaze: Handle,
    pub boss_wave: Handle,
    pub dojo: Handle,
    pub starred_alien: Handle,
    pub burrow: Handle,
    pub revive: Handle,
    pub stop: Handle,
    pub death_timer: Handle,
    pub god: Handle,
    pub unknown: Handle,
}

impl Default for CustomAssets {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomAssets {
    pub fn new() -> Self {
        info!("Initializing CustomAssets module");

        let load = |name: &str, bytes: &[u8]| -> Handle {
            match image::load_from_memory(bytes) {
                Ok(img) => {
                    trace!("Successfully loaded embedded asset: {}", name);
                    let rgba = img.into_rgba8();
                    Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw())
                }
                Err(e) => {
                    error!("Failed to load embedded asset '{name}': {e}");
                    Handle::from_rgba(1, 1, vec![255, 0, 255, 255])
                }
            }
        };

        Self {
            multihit: load("multihit", MULTIHIT),
            kamikaze: load("kamikaze", KAMIKAZE),
            boss_wave: load("boss_wave", BOSS_WAVE),
            dojo: load("dojo", DOJO),
            starred_alien: load("starred_alien", STARRED_ALIEN),
            burrow: load("burrow", BURROW),
            revive: load("revive", REVIVE),
            stop: load("stop", STOP),
            death_timer: load("death_timer", DEATH_TIMER),
            god: load("god", GOD),
            unknown: load("unknown", UNKNOWN),
        }
    }

    pub fn get_icon_texture(&self, icon: CustomIcon) -> Option<Handle> {
        match icon {
            CustomIcon::Multihit => Some(self.multihit.clone()),
            CustomIcon::Kamikaze => Some(self.kamikaze.clone()),
            CustomIcon::BossWave => Some(self.boss_wave.clone()),
            CustomIcon::Dojo => Some(self.dojo.clone()),
            CustomIcon::StarredAlien => Some(self.starred_alien.clone()),
            CustomIcon::Burrow => Some(self.burrow.clone()),
            CustomIcon::Revive => Some(self.revive.clone()),
            CustomIcon::Stop => Some(self.stop.clone()),
            CustomIcon::DeathTimer => Some(self.death_timer.clone()),
            CustomIcon::God => Some(self.god.clone()),
            CustomIcon::Unknown => Some(self.unknown.clone()),
            CustomIcon::None => None,
        }
    }
}

#[derive(Default)]
pub struct SpriteSheet {
    pub core: CoreSpriteSheet,
    pub texture_handle: Option<Handle>,
    loading: bool,
    failed: bool,
}

impl SpriteSheet {
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn has_failed(&self) -> bool {
        self.failed
    }

    pub fn load(&mut self, png_path: &Path, imgcut_path: &Path, id_str: String) -> Task<Option<CoreSpriteSheet>> {
        debug!("Loading SpriteSheet identifier: {}", id_str);
        self.loading = true;

        let png_path = png_path.to_path_buf();
        let imgcut_path = imgcut_path.to_path_buf();
        let (tx, rx) = unbounded();

        thread::spawn(move || {
            let parsed = imgcut::parse(&png_path, &imgcut_path);
            if parsed.is_none() {
                warn!("Failed to parse sprite sheet '{}' from {:?}", id_str, png_path);
            }

            let result = parsed.map(|sheet| CoreSpriteSheet {
                image_data: sheet.image_data,
                cuts_map: sheet.cuts_map,
                sheet_name: id_str,
            });
            let _ = tx.unbounded_send(result);
        });

        Task::stream(rx)
    }

    pub fn apply(&mut self, result: Option<CoreSpriteSheet>) {
        self.loading = false;

        let Some(core) = result else {
            self.failed = true;
            return;
        };

        self.core = core;

        let Some(image_data) = &self.core.image_data else {
            self.failed = true;
            return;
        };

        debug!("Generating texture handle for sheet: {}", self.core.sheet_name);
        self.texture_handle = Some(Handle::from_rgba(
            image_data.width(),
            image_data.height(),
            image_data.as_raw().clone(),
        ));
    }
}
pub fn ensure_sheet_loaded(
    sheets: &mut Vec<SpriteSheet>,
    vfs: &Vfs,
    name: &str,
) -> Task<(usize, Option<CoreSpriteSheet>)> {
    let png_paths = vfs.list(&format!("{}.png", name));

    if sheets.len() != png_paths.len() {
        debug!("Resizing the {} sheet matrix to match resolved paths ({})", name, png_paths.len());
        sheets.resize_with(png_paths.len(), SpriteSheet::default);
    }

    let mut tasks = Vec::new();

    for (index, png_path) in png_paths.into_iter().enumerate() {
        if sheets[index].texture_handle.is_some() || sheets[index].is_loading() || sheets[index].has_failed() {
            continue;
        }

        let stem = png_path.file_stem().map_or_else(|| name.to_string(), |stem| stem.to_string_lossy().into_owned());

        let Some(imgcut_path) = vfs
            .locate(&format!("{}.imgcut", stem))
            .or_else(|| vfs.locate(&format!("{}.imgcut", name)))
        else {
            warn!(sheet = %stem, "No matching imgcut for the resolved sprite sheet");
            continue;
        };

        trace!("Loading sprite sheet {}", stem);
        tasks.push(sheets[index].load(&png_path, &imgcut_path, stem).map(move |result| (index, result)));
    }

    Task::batch(tasks)
}

impl SpriteSheet {
    pub fn is_settled(&self) -> bool {
        self.failed || self.core.image_data.is_some()
    }
}
