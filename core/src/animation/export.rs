pub mod encoding;
pub mod find_loop;
pub mod leader;
pub mod process;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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

#[derive(Clone, Copy, Debug)]
pub struct ShowcaseLengths {
    pub walk: i32,
    pub idle: i32,
    pub attack: i32,
    pub kb: i32,
}

#[derive(Clone, Debug)]
pub struct FrameTiming {
    pub mode: ExportMode,
    pub frame_start: i32,
    pub frame_end: i32,
    pub loop_supported: bool,
}

#[derive(Clone, Debug)]
pub struct ExportRequest {
    pub timing: FrameTiming,
    pub showcase: ShowcaseLengths,
    pub file_name: Option<String>,
    pub name_prefix: String,
    pub format: ExportFormat,
    pub quality_percent: u32,
    pub compression_percent: u32,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub background: bool,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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