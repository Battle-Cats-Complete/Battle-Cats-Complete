pub mod encoding;
pub mod find_loop;
pub mod leader;
pub mod process;

use std::path::PathBuf;

use crate::modules::settings::Settings;

const DEFAULT_WALK_LEN: i32 = 90;
const DEFAULT_IDLE_LEN: i32 = 90;
const DEFAULT_KB_LEN: i32 = 60;

#[derive(Clone, PartialEq, Debug)]
pub enum ExportMode {
    Manual,
    Loop,
    Showcase,
}

#[derive(Clone, Debug)]
pub enum LoopStatus {
    Searching(usize),
    Found(i32, i32),
    Error(String),
}

#[derive(Clone, Debug)]
pub struct ExportConfig {
    pub width: u32,
    pub height: u32,
    pub format: ExportFormat,
    pub quality_percent: u32,
    pub compression_percent: u32,
    pub fps: u32,
    pub start_frame: i32,
    pub end_frame: i32,
    pub output_path: PathBuf,
    pub base_name: String,
    pub background: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExportFormat {
    Gif,
    WebP,
    Avif,
    Png,
    Mp4,
    Mkv,
    Webm,
    Zip,
}

pub enum EncoderMessage {
    Frame(Vec<u8>, u32, u32, u32),
    Finish,
}

#[derive(Debug, Clone)]
pub enum EncoderStatus {
    Progress(u32),
    Finished,
}

pub struct ExporterState {
    pub frame_start: i32,
    pub frame_end: i32,
    pub max_frame: i32,
    pub frame_start_str: String,
    pub frame_end_str: String,
    pub export_mode: ExportMode,
    pub loop_supported: bool,
    pub loop_tolerance: i32,
    pub loop_tolerance_str: String,
    pub loop_min: i32,
    pub loop_min_str: String,
    pub loop_max: Option<i32>,
    pub loop_max_str: String,
    pub showcase_walk_str: String,
    pub showcase_idle_str: String,
    pub showcase_attack_str: String,
    pub showcase_kb_str: String,
    pub showcase_walk_len: i32,
    pub showcase_idle_len: i32,
    pub detected_attack_len: i32,
    pub showcase_attack_len: i32,
    pub showcase_kb_len: i32,
    pub detected_walk_len: i32,
    pub detected_idle_len: i32,
    pub last_known_walk_default: i32,
    pub last_known_idle_default: i32,
    pub last_known_kb_default: i32,
    pub fps: i32,
    pub zoom: f32,
    pub region_x: f32,
    pub region_y: f32,
    pub region_w: f32,
    pub region_h: f32,
    pub file_name: String,
    pub name_prefix: String,
    pub format: ExportFormat,
    pub quality_percent: i32,
    pub quality_percent_str: String,
    pub compression_percent: i32,
    pub compression_percent_str: String,
    pub background: bool,
    pub user_bg_preference: bool,
}

impl Default for ExporterState {
    fn default() -> Self {
        Self {
            frame_start: 0,
            frame_end: 0,
            max_frame: 100,
            frame_start_str: String::new(),
            frame_end_str: String::new(),
            export_mode: ExportMode::Manual,
            loop_supported: false,
            loop_tolerance: 30,
            loop_tolerance_str: String::new(),
            loop_min: 15,
            loop_min_str: String::new(),
            loop_max: None,
            loop_max_str: String::new(),
            showcase_walk_str: String::new(),
            showcase_idle_str: String::new(),
            showcase_attack_str: String::new(),
            showcase_kb_str: String::new(),
            showcase_walk_len: DEFAULT_WALK_LEN,
            showcase_idle_len: DEFAULT_IDLE_LEN,
            detected_attack_len: 0,
            showcase_attack_len: 0,
            showcase_kb_len: DEFAULT_KB_LEN,
            detected_walk_len: DEFAULT_WALK_LEN,
            detected_idle_len: DEFAULT_IDLE_LEN,
            last_known_walk_default: DEFAULT_WALK_LEN,
            last_known_idle_default: DEFAULT_IDLE_LEN,
            last_known_kb_default: DEFAULT_KB_LEN,
            fps: 30,
            zoom: 1.0,
            region_x: 0.0,
            region_y: 0.0,
            region_w: 0.0,
            region_h: 0.0,
            file_name: String::new(),
            name_prefix: String::new(),
            format: ExportFormat::Gif,
            quality_percent: 100,
            quality_percent_str: String::new(),
            compression_percent: 0,
            compression_percent_str: String::new(),
            background: false,
            user_bg_preference: false,
        }
    }
}

impl ExporterState {
    pub fn with_settings(settings: &Settings) -> Self {
        let format = match settings.animation.last_export_format {
            1 => ExportFormat::WebP,
            2 => ExportFormat::Avif,
            3 => ExportFormat::Png,
            4 => ExportFormat::Mp4,
            5 => ExportFormat::Mkv,
            6 => ExportFormat::Webm,
            7 => ExportFormat::Zip,
            _ => ExportFormat::Gif,
        };

        Self {
            format,
            quality_percent: settings.animation.last_export_quality.unwrap_or(100),
            quality_percent_str: settings.animation.last_export_quality.map_or_else(String::new, |v| v.to_string()),
            compression_percent: settings.animation.last_export_compression.unwrap_or(0),
            compression_percent_str: settings.animation.last_export_compression.map_or_else(String::new, |v| v.to_string()),
            showcase_walk_len: settings.animation.default_showcase_walk,
            showcase_idle_len: settings.animation.default_showcase_idle,
            showcase_kb_len: settings.animation.default_showcase_kb,
            last_known_walk_default: settings.animation.default_showcase_walk,
            last_known_idle_default: settings.animation.default_showcase_idle,
            last_known_kb_default: settings.animation.default_showcase_kb,
            ..Self::default()
        }
    }
}