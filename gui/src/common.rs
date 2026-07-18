pub(crate) mod img015;
pub(crate) mod img022;
pub(crate) mod name_box;
pub(crate) mod shared;
pub(crate) mod stat_grid;
pub(crate) mod text;
pub(crate) mod watcher;

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use eframe::egui;
use image::load_from_memory;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{debug, error, info, trace, warn};

use core::common::assets::{
    BOSS_WAVE, BURROW, DEATH_TIMER, DOJO, GOD, KAMIKAZE, MULTIHIT, REVIVE, STARRED_ALIEN, STOP,
    UDI_F, UNKNOWN,
};
use core::common::formats::SpriteSheet as CoreSpriteSheet;
use core::common::game::CustomIcon;

#[derive(Clone)]
pub(crate) struct CustomAssets {
    pub multihit: egui::TextureHandle,
    pub kamikaze: egui::TextureHandle,
    pub boss_wave: egui::TextureHandle,
    pub dojo: egui::TextureHandle,
    pub starred_alien: egui::TextureHandle,
    pub burrow: egui::TextureHandle,
    pub revive: egui::TextureHandle,
    pub stop: egui::TextureHandle,
    pub death_timer: egui::TextureHandle,
    pub god: egui::TextureHandle,
    pub unknown: egui::TextureHandle,
    #[allow(dead_code)]
    pub udi_f: egui::TextureHandle,
}

impl CustomAssets {
    pub(crate) fn new(ctx: &egui::Context) -> Self {
        info!("Initializing CustomAssets module");

        let load = |name: &str, bytes: &[u8]| -> egui::TextureHandle {
            match load_from_memory(bytes) {
                Ok(img) => {
                    trace!("Successfully loaded embedded asset: {}", name);
                    let rgba = img.to_rgba8();
                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                        [rgba.width() as usize, rgba.height() as usize],
                        rgba.as_flat_samples().as_slice(),
                    );
                    ctx.load_texture(name, color_img, egui::TextureOptions::LINEAR)
                }
                Err(e) => {
                    error!("Failed to load embedded asset '{name}': {e}");
                    let fallback_img = egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255, 0, 255, 255]);
                    ctx.load_texture(name, fallback_img, egui::TextureOptions::LINEAR)
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
            udi_f: load("udi_f", UDI_F),
        }
    }

    pub(crate) fn get_icon_texture(&self, icon: CustomIcon) -> Option<&egui::TextureHandle> {
        match icon {
            CustomIcon::Multihit => Some(&self.multihit),
            CustomIcon::Kamikaze => Some(&self.kamikaze),
            CustomIcon::BossWave => Some(&self.boss_wave),
            CustomIcon::Dojo => Some(&self.dojo),
            CustomIcon::StarredAlien => Some(&self.starred_alien),
            CustomIcon::Burrow => Some(&self.burrow),
            CustomIcon::Revive => Some(&self.revive),
            CustomIcon::Stop => Some(&self.stop),
            CustomIcon::DeathTimer => Some(&self.death_timer),
            CustomIcon::God => Some(&self.god),
            CustomIcon::Unknown => Some(&self.unknown),
            CustomIcon::None => None,
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct SpriteSheet {
    pub core: CoreSpriteSheet,
    pub texture_handle: Option<egui::TextureHandle>,
}

impl SpriteSheet {
    pub(crate) fn load(&mut self, png_path: &Path, imgcut_path: &Path, id_str: String) {
        debug!("Loading SpriteSheet identifier: {}", id_str);
        self.core.load(png_path, imgcut_path, id_str);
    }

    pub(crate) fn update(&mut self, ctx: &egui::Context) {
        self.core.update();

        if self.texture_handle.is_none() && !self.core.is_loading_active {
            if let Some(image_data) = &self.core.image_data {
                debug!("Generating texture handle for sheet: {}", self.core.sheet_name);
                let size = [image_data.width() as usize, image_data.height() as usize];
                let pixels = image_data.as_flat_samples();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

                self.texture_handle = Some(ctx.load_texture(
                    &self.core.sheet_name,
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct DragGuard {
    broken: bool,
}

impl DragGuard {
    pub(crate) fn update(&mut self, ctx: &egui::Context) -> bool {
        let screen_rect = ctx.screen_rect();
        let (pointer_pos, mouse_down) = ctx.input(|i| {
            (i.pointer.interact_pos(), i.pointer.primary_down())
        });

        let in_window = pointer_pos.is_some_and(|p| screen_rect.contains(p));

        if !mouse_down {
            self.broken = false;
        } else if !in_window {
            self.broken = true;
        }

        in_window && !self.broken
    }

    pub(crate) fn assign_bounds(&mut self, ctx: &egui::Context, window_id: egui::Id) -> (bool, Option<egui::Pos2>) {
        (self.update(ctx), shared::clamp_window_to_screen(ctx, window_id))
    }
}

pub(crate) struct GuiWatcher {
    _watcher: RecommendedWatcher,
    pub rx: Receiver<PathBuf>,
}

impl GuiWatcher {
    pub(crate) fn new(ctx: egui::Context) -> Option<Self> {
        info!("Initializing GuiWatcher");

        let (internal_tx, internal_rx) = std::sync::mpsc::channel();
        let (final_tx, final_rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            watcher::debounce_loop(internal_rx, final_tx, ctx);
        });

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            match res {
                Ok(event) => {
                    if !matches!(event.kind, EventKind::Access(_)) {
                        for path in event.paths {
                            let path_str = path.to_string_lossy().to_lowercase();
                            if path_str.contains("raw") {
                                continue;
                            }

                            if !path_str.contains("game") && !path_str.contains("mods") {
                                continue;
                            }

                            let components: Vec<_> = path
                                .components()
                                .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
                                .collect();

                            if let Some(mods_idx) = components.iter().position(|c| c == "mods") {
                                if let Some(sub_folder) = components.get(mods_idx + 2) {
                                    if sub_folder != "patch" && sub_folder != "icons" && sub_folder != "loose" {
                                        continue;
                                    }
                                }
                            }

                            trace!("GuiWatcher detected file change: {:?}", path);
                            let _ = internal_tx.send(path);
                        }
                    }
                }
                Err(e) => warn!("GuiWatcher received an event error: {}", e),
            }
        })
            .ok()?;

        let current_dir = Path::new(".");
        if let Err(e) = watcher.watch(current_dir, RecursiveMode::NonRecursive) {
            warn!("Failed to watch current directory: {e}");
        }

        let game_path = Path::new("game");
        if game_path.exists() {
            if let Err(e) = watcher.watch(game_path, RecursiveMode::Recursive) {
                warn!("Failed to watch game directory: {e}");
            }
        }

        let mods_path = Path::new("mods");
        if mods_path.exists() {
            if let Err(e) = watcher.watch(mods_path, RecursiveMode::Recursive) {
                warn!("Failed to watch mods directory: {e}");
            }
        }

        Some(Self {
            _watcher: watcher,
            rx: final_rx,
        })
    }
}