mod canvas;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use eframe::egui;
use nyanko::graphics::rig::{Animation, Unit};
use tracing::{error, info, trace, warn};

use core::modules::animation::export::{ExportMode, ExporterState};
use core::modules::animation::GlowRenderer;
use core::modules::settings::Settings;

const IDX_WALK: usize = 0;
const IDX_IDLE: usize = 1;
const IDX_ATTACK: usize = 2;
const IDX_KB: usize = 3;
const IDX_SPIRIT: usize = 4;
const IDX_BURROW: usize = 5;
const IDX_SURFACE: usize = 6;
const IDX_MODEL: usize = 99;
const IDX_NONE: usize = 999;

pub(crate) struct AnimViewer {
    pub zoom_level: f32,
    pub target_zoom_level: f32,
    pub pan_offset: egui::Vec2,
    pub current_anim: Option<Arc<Animation>>,
    pub current_frame: f32,
    pub is_playing: bool,
    pub playback_speed: f32,
    pub loop_range: (Option<i32>, Option<i32>),
    pub range_str_cache: (String, String),
    pub single_frame_str: String,
    pub speed_str: String,
    pub hold_timer: f32,
    pub hold_dir: i8,
    pub loaded_anim_index: usize,
    pub last_anim_index: usize,
    pub loaded_id: String,
    pub failed_load_id: String,
    pub summoner_id: String,
    pub last_loaded_id: String,
    pub pending_initial_center: bool,
    pub held_unit: Option<Arc<Unit>>,
    pub renderer: Arc<Mutex<Option<GlowRenderer>>>,
    pub cached_controls_width: f32,
    pub cached_grid_height: f32,
    pub is_expanded: bool,
    pub texture_version: u64,
    pub is_pointer_over_controls: bool,
    pub is_viewport_dragging: bool,
    pub is_selecting_export_region: bool,
    pub export_selection_start: Option<egui::Pos2>,
    pub export_state: ExporterState,
    pub has_scanned_showcase: bool,
    pub was_export_popup_open: bool,
}

impl Default for AnimViewer {
    fn default() -> Self {
        Self {
            zoom_level: 1.0,
            target_zoom_level: 1.0,
            pan_offset: egui::vec2(0.0, 0.0),
            current_anim: None,
            current_frame: 0.0,
            is_playing: true,
            playback_speed: 1.0,
            loop_range: (None, None),
            range_str_cache: (String::new(), String::new()),
            single_frame_str: String::new(),
            speed_str: String::new(),
            hold_timer: 0.0,
            hold_dir: 0,
            loaded_anim_index: 0,
            last_anim_index: usize::MAX,
            loaded_id: String::new(),
            failed_load_id: String::new(),
            summoner_id: String::new(),
            last_loaded_id: "FORCE_INIT".to_string(),
            pending_initial_center: false,
            held_unit: None,
            renderer: Arc::new(Mutex::new(None)),
            cached_controls_width: 0.0,
            cached_grid_height: 55.0,
            is_expanded: false,
            texture_version: 0,
            is_pointer_over_controls: false,
            is_viewport_dragging: false,
            is_selecting_export_region: false,
            export_selection_start: None,
            export_state: ExporterState::default(),
            has_scanned_showcase: false,
            was_export_popup_open: false,
        }
    }
}

impl AnimViewer {
    pub fn update_export_state(&mut self, _settings: &Settings) {
        trace!("Updating animation export state settings");
        self.export_state.loop_supported = self.loaded_anim_index == IDX_WALK || self.loaded_anim_index == IDX_IDLE;

        if self.export_state.export_mode != ExportMode::Showcase {
            if let Some(anim) = &self.current_anim {
                let true_end = anim.calculate_true_loop().unwrap_or(anim.max_frame);

                self.export_state.max_frame = true_end;
                self.export_state.frame_start = 0;
                self.export_state.frame_end = true_end;
            } else {
                self.export_state.max_frame = 0;
                self.export_state.frame_start = 0;
                self.export_state.frame_end = 0;
            }
            self.export_state.frame_start_str.clear();
            self.export_state.frame_end_str.clear();
        }

        let type_string = match self.loaded_anim_index {
            IDX_WALK => "walk",
            IDX_IDLE => "idle",
            IDX_ATTACK => "attack",
            IDX_KB => "kb",
            IDX_BURROW => "burrow",
            IDX_SURFACE => "surface",
            IDX_SPIRIT => "spirit",
            IDX_MODEL => "model",
            _ => "anim",
        };

        let raw_id = if self.loaded_anim_index == IDX_SPIRIT {
            if self.summoner_id.is_empty() { &self.loaded_id } else { &self.summoner_id }
        } else { &self.loaded_id };

        let id_parts: Vec<&str> = raw_id.split('_').collect();
        let mut clean_id = id_parts.first().unwrap_or(&"unit").to_string();

        if id_parts.len() >= 2
            && id_parts[0].chars().all(char::is_numeric) {
            let form_number = match id_parts[1].chars().next() {
                Some('f') => 1, Some('c') => 2, Some('s') => 3, Some('u') => 4, _ => 0
            };
            if form_number > 0 { clean_id = format!("{}-{}", id_parts[0], form_number); }
        }

        self.export_state.name_prefix = format!("{}.{}", clean_id, type_string);
    }

    pub fn load_anim(&mut self, path: &Path, settings: &Settings) {
        info!("Loading animation from path: {}", path.display());
        if let Ok(anim_bytes) = fs::read(path) {
            if let Some(anim) = Animation::parse(&anim_bytes) {
                trace!("Successfully parsed animation file.");
                self.current_frame = 0.0;
                self.loop_range = (None, None);
                self.range_str_cache = (String::new(), String::new());
                self.single_frame_str = "0".to_string();

                self.current_anim = Some(Arc::new(anim));
                self.update_export_state(settings);
                return;
            } else {
                warn!("Animation binary parsing failed for path: {}", path.display());
            }
        } else {
            error!("Failed to read animation file from disk.");
        }

        self.current_anim = None;
        self.current_frame = 0.0;
        self.loop_range = (None, None);
        self.range_str_cache = (String::new(), String::new());
        self.single_frame_str = "0".to_string();
    }

    pub fn resolve_paths<'a>(
        target_index: usize,
        primary_assets: &'a Option<(PathBuf, PathBuf, PathBuf)>,
        secondary_assets: &'a Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
        available_anims: &'a [(usize, PathBuf)]
    ) -> (Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>) {
        if target_index == IDX_SPIRIT {
            if let Some((secondary_png, secondary_cut, secondary_model, secondary_anim)) = secondary_assets {
                return (Some(secondary_png), Some(secondary_cut), Some(secondary_model), Some(secondary_anim));
            }
        } else {
            let animation_path = available_anims.iter().find(|(index, _)| *index == target_index).map(|(_, path)| path);
            if let Some((primary_png, primary_cut, primary_model)) = primary_assets {
                return (Some(primary_png), Some(primary_cut), Some(primary_model), animation_path);
            }
        }
        (None, None, None, None)
    }
}