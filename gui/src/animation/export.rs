use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Instant;

use iced::futures::channel::mpsc::unbounded;
use iced::widget::{button, column, container, pick_list, progress_bar, row, rule, scrollable, text, text_input, toggler, tooltip};
use iced::{task, Alignment, Element, Length, Size, Task, Theme};
use tracing::trace;

use nyanko::graphics::rig::Animation;

use core::addons::paths::{self, Presence};
use core::animation::export::{find_loop, leader, process, EncoderStatus, ExportFormat, ExportMode, ExportRequest, FrameTiming, LoopStatus, ShowcaseLengths};
use core::animation::{IDX_ATTACK, IDX_BURROW, IDX_IDLE, IDX_KB, IDX_MODEL, IDX_SPIRIT, IDX_SURFACE, IDX_WALK};
use core::modules::settings::Settings;

use crate::app::state::AnimState;
use crate::app::theme;
use crate::widget::{popup, section, smooth_scroll};

use super::data;
use super::offscreen::{self, Camera};
use super::overlay::Region;

const POPUP_SIZE: Size = Size::new(320.0, 500.0);
const MODE_OPTIONS: [&str; 3] = ["Manual", "Loop", "Showcase"];
const FORMAT_OPTIONS: [&str; 8] = ["GIF", "WebP", "AVIF", "PNG", "MP4", "MKV", "WebM", "ZIP"];

const CONTENT_PADDING: f32 = 20.0;
const SECTION_SPACING: f32 = 14.0;
const ROW_SPACING: f32 = 6.0;
const FIELD_SPACING: f32 = 8.0;
const FIELD_LABEL_WIDTH: f32 = 82.0;
const NAME_INPUT_WIDTH: f32 = 170.0;
const SMALL_INPUT_WIDTH: f32 = 55.0;
const AXIS_INPUT_WIDTH: f32 = 100.0;
const AXIS_LABEL_WIDTH: f32 = 18.0;
const COMBO_WIDTH: f32 = 130.0;
const BUTTON_STATUS_GAP: f32 = 8.0;
const RULE_HEIGHT: f32 = 1.0;
const SCROLLBAR_GAP: f32 = 2.0;
const CONTROL_TEXT_SIZE: f32 = 13.0;
const POPUP_BODY_ALPHA: f32 = 0.95;

const DEFAULT_WALK_LEN: i32 = 90;
const DEFAULT_IDLE_LEN: i32 = 90;
const DEFAULT_KB_LEN: i32 = 60;

type JobKey = (String, usize);

enum JobResult {
    Completed,
    Terminated,
}

enum JobPhase {
    Running,
    Aborting,
    Done { result: JobResult, shown_at: Option<Instant> },
}

struct JobState {
    phase: JobPhase,
    abort: Arc<AtomicBool>,
    render_progress: Arc<AtomicI32>,
    rendered_frames: i32,
    encoded_frames: i32,
    total_frames: i32,
    encoder_handle: task::Handle,
}

enum LoopResult {
    Found,
    Terminated,
    Error(String),
}

enum LoopPhase {
    Running,
    Aborting,
    Done { result: LoopResult, shown_at: Option<Instant> },
}

struct LoopJob {
    phase: LoopPhase,
    abort: Arc<AtomicBool>,
    frames_searched: usize,
    start_time: Instant,
    task_handle: task::Handle,
}

struct ExportForm {
    frame_start: i32,
    frame_end: i32,
    max_frame: i32,
    frame_start_str: String,
    frame_end_str: String,
    export_mode: ExportMode,
    loop_supported: bool,
    loop_tolerance: i32,
    loop_tolerance_str: String,
    loop_min: i32,
    loop_min_str: String,
    loop_max: Option<i32>,
    loop_max_str: String,
    showcase_walk_str: String,
    showcase_idle_str: String,
    showcase_attack_str: String,
    showcase_kb_str: String,
    showcase_walk_len: i32,
    showcase_idle_len: i32,
    detected_attack_len: i32,
    showcase_attack_len: i32,
    showcase_kb_len: i32,
    detected_walk_len: i32,
    detected_idle_len: i32,
    last_known_walk_default: i32,
    last_known_idle_default: i32,
    last_known_kb_default: i32,
    fps: i32,
    zoom: f32,
    region_x: f32,
    region_y: f32,
    region_w: f32,
    region_h: f32,
    file_name: String,
    name_prefix: String,
    format: ExportFormat,
    quality_percent: i32,
    quality_percent_str: String,
    compression_percent: i32,
    compression_percent_str: String,
    background: bool,
    user_bg_preference: bool,
}

impl Default for ExportForm {
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
            quality_percent: 80,
            quality_percent_str: String::new(),
            compression_percent: 30,
            compression_percent_str: String::new(),
            background: false,
            user_bg_preference: false,
        }
    }
}

impl ExportForm {
    fn with_settings(settings: &Settings) -> Self {
        Self {
            showcase_walk_len: settings.animation.default_showcase_walk,
            showcase_idle_len: settings.animation.default_showcase_idle,
            showcase_kb_len: settings.animation.default_showcase_kb,
            last_known_walk_default: settings.animation.default_showcase_walk,
            last_known_idle_default: settings.animation.default_showcase_idle,
            last_known_kb_default: settings.animation.default_showcase_kb,
            ..Self::default()
        }
    }

    fn to_request(&self) -> ExportRequest {
        ExportRequest {
            timing: FrameTiming {
                mode: self.export_mode.clone(),
                frame_start: self.frame_start,
                frame_end: self.frame_end,
                loop_supported: self.loop_supported,
            },
            showcase: ShowcaseLengths {
                walk: self.showcase_walk_len,
                idle: self.showcase_idle_len,
                attack: self.showcase_attack_len,
                kb: self.showcase_kb_len,
            },
            file_name: (!self.file_name.trim().is_empty()).then(|| self.file_name.clone()),
            name_prefix: self.name_prefix.clone(),
            format: self.format.clone(),
            quality_percent: self.quality_percent as u32,
            compression_percent: self.compression_percent as u32,
            fps: self.fps as u32,
            width: self.region_w.round() as u32,
            height: self.region_h.round() as u32,
            background: self.background,
        }
    }
}

#[derive(Default)]
pub struct State {
    popup: popup::State,
    exporter: ExportForm,
    jobs: HashMap<JobKey, JobState>,
    loop_job: Option<(JobKey, LoopJob)>,
    synced_key: Option<JobKey>,
    scanned_showcase: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    SetMode(ExportMode),
    SetFormat(ExportFormat),
    SetFileName(String),
    SetStartFrame(String),
    SetEndFrame(String),
    SetLoopTolerance(String),
    SetLoopMin(String),
    SetLoopMax(String),
    SetShowcaseWalk(String),
    SetShowcaseIdle(String),
    SetShowcaseAttack(String),
    SetShowcaseKb(String),
    SetRegionX(String),
    SetRegionY(String),
    SetRegionW(String),
    SetRegionH(String),
    SetQuality(String),
    SetCompression(String),
    ToggleBackground(bool),
    BeginExport,
    AbortExport,
    Encoder(JobKey, EncoderStatus),
    SetCamera,
    UseBounds,
    FindLoop,
    AbortLoopSearch,
    LoopSearch(JobKey, LoopStatus),
}

impl State {
    pub fn sync(&mut self, data: &data::State, settings: &Settings, anim_state: &AnimState) {
        self.check_settings_defaults(settings);

        let key = (data.loaded_id().to_string(), data.loaded_anim_index);

        if self.synced_key.as_ref() != Some(&key) {
            let unit_changed = self.synced_key.as_ref().is_none_or(|(id, _)| *id != key.0);
            self.synced_key = Some(key);

            if unit_changed {
                self.reset(settings, anim_state);
            }

            self.exporter.loop_supported = matches!(data.loaded_anim_index, IDX_WALK | IDX_IDLE);

            if self.exporter.export_mode == ExportMode::Loop && !self.exporter.loop_supported {
                self.exporter.export_mode = ExportMode::Manual;
                self.exporter.frame_start = 0;
                self.exporter.frame_end = 0;
                self.exporter.frame_start_str.clear();
                self.exporter.frame_end_str.clear();
            }

            if self.exporter.export_mode != ExportMode::Showcase {
                match &data.current_anim {
                    Some(anim) => {
                        self.exporter.max_frame = anim.max_frame;
                        self.exporter.frame_start = 0;
                        self.exporter.frame_end = anim.max_frame;
                    }
                    None => {
                        self.exporter.max_frame = 0;
                        self.exporter.frame_start = 0;
                        self.exporter.frame_end = 0;
                    }
                }
                self.exporter.frame_start_str.clear();
                self.exporter.frame_end_str.clear();
            }

            self.exporter.name_prefix = derive_name_prefix(data.primary_id(), data.loaded_anim_index);
        }

        self.maybe_scan_showcase(data, settings);
    }

    fn check_settings_defaults(&mut self, settings: &Settings) {
        let walk_mismatch = self.exporter.last_known_walk_default != settings.animation.default_showcase_walk;
        let idle_mismatch = self.exporter.last_known_idle_default != settings.animation.default_showcase_idle;
        let kb_mismatch = self.exporter.last_known_kb_default != settings.animation.default_showcase_kb;

        if !walk_mismatch && !idle_mismatch && !kb_mismatch {
            return;
        }

        self.exporter.last_known_walk_default = settings.animation.default_showcase_walk;
        self.exporter.last_known_idle_default = settings.animation.default_showcase_idle;
        self.exporter.last_known_kb_default = settings.animation.default_showcase_kb;

        if self.exporter.showcase_walk_str.is_empty() {
            self.exporter.showcase_walk_len = settings.animation.default_showcase_walk;
        }
        if self.exporter.showcase_idle_str.is_empty() {
            self.exporter.showcase_idle_len = settings.animation.default_showcase_idle;
        }
        if self.exporter.showcase_kb_str.is_empty() {
            self.exporter.showcase_kb_len = settings.animation.default_showcase_kb;
        }

        self.scanned_showcase = None;
    }

    fn maybe_scan_showcase(&mut self, data: &data::State, settings: &Settings) {
        if self.exporter.export_mode != ExportMode::Showcase {
            return;
        }

        let loaded_id = data.loaded_id();
        if self.scanned_showcase.as_deref() == Some(loaded_id) {
            return;
        }
        self.scanned_showcase = Some(loaded_id.to_string());

        let parse_anim = |target_idx: usize| -> Option<Animation> {
            let (_, path) = data.available_anims.iter().find(|(idx, _)| *idx == target_idx)?;
            let bytes = fs::read(path).ok()?;
            Animation::parse(&bytes)
        };

        if let Some(attack) = parse_anim(IDX_ATTACK) {
            let total_attack_frames = attack.max_frame + 1;
            self.exporter.detected_attack_len = total_attack_frames;
            if self.exporter.showcase_attack_str.is_empty() {
                self.exporter.showcase_attack_len = total_attack_frames;
            }
        }

        if let Some(walk) = parse_anim(IDX_WALK) {
            let walk_loop = walk.calculate_true_loop().unwrap_or(walk.max_frame);
            let new_walk_length = if walk_loop <= 1 { 0 } else { settings.animation.default_showcase_walk };
            self.exporter.detected_walk_len = new_walk_length;

            if self.exporter.showcase_walk_str.is_empty()
                || self.exporter.showcase_walk_len == settings.animation.default_showcase_walk {
                self.exporter.showcase_walk_len = new_walk_length;
            }
        }

        if let Some(idle) = parse_anim(IDX_IDLE) {
            let idle_loop = idle.calculate_true_loop().unwrap_or(idle.max_frame);
            let new_idle_length = if idle_loop <= 1 { 0 } else { settings.animation.default_showcase_idle };
            self.exporter.detected_idle_len = new_idle_length;

            if self.exporter.showcase_idle_str.is_empty()
                || self.exporter.showcase_idle_len == settings.animation.default_showcase_idle {
                self.exporter.showcase_idle_len = new_idle_length;
            }
        }
    }

    fn reset(&mut self, settings: &Settings, anim_state: &AnimState) {
        let previous_mode = self.exporter.export_mode.clone();
        self.exporter = ExportForm::with_settings(settings);
        self.exporter.export_mode = previous_mode;
        self.exporter.format = anim_state.last_export_format.clone();
        self.exporter.quality_percent = anim_state.last_export_quality.unwrap_or(80);
        self.exporter.quality_percent_str = anim_state.last_export_quality.map_or_else(String::new, |v| v.to_string());
        self.exporter.compression_percent = anim_state.last_export_compression.unwrap_or(30);
        self.exporter.compression_percent_str = anim_state.last_export_compression.map_or_else(String::new, |v| v.to_string());
    }

    pub fn set_region(&mut self, region: Region) {
        self.exporter.region_x = region.x;
        self.exporter.region_y = region.y;
        self.exporter.region_w = region.w;
        self.exporter.region_h = region.h;
        self.exporter.zoom = 1.0;
    }

    pub fn camera_region(&self, anim_state: &AnimState) -> Option<Region> {
        if anim_state.export_popup_open && self.exporter.region_w > 0.1 && self.exporter.region_h > 0.1 {
            Some(Region {
                x: self.exporter.region_x,
                y: self.exporter.region_y,
                w: self.exporter.region_w,
                h: self.exporter.region_h,
            })
        } else {
            None
        }
    }

    pub fn tick(&mut self) {
        if let Some(key) = &self.synced_key
            && let Some(job) = self.jobs.get_mut(key) {
            job.rendered_frames = job.render_progress.load(Ordering::Relaxed);

            if let JobPhase::Done { shown_at, .. } = &mut job.phase
                && shown_at.is_none() {
                *shown_at = Some(Instant::now());
            }
        }

        self.jobs.retain(|_, job| {
            if let JobPhase::Done { shown_at: Some(at), .. } = &job.phase
                && at.elapsed().as_secs_f32() > 3.0 {
                job.encoder_handle.abort();
                return false;
            }
            true
        });

        if let Some((_, job)) = &mut self.loop_job
            && matches!(job.phase, LoopPhase::Aborting) {
            job.phase = LoopPhase::Done { result: LoopResult::Terminated, shown_at: None };
        }

        if let Some(key) = &self.synced_key
            && let Some((job_key, job)) = &mut self.loop_job
            && job_key == key
            && let LoopPhase::Done { shown_at, .. } = &mut job.phase
            && shown_at.is_none() {
            *shown_at = Some(Instant::now());
        }

        let loop_expired = self.loop_job.as_ref().is_some_and(|(_, job)| {
            matches!(&job.phase, LoopPhase::Done { shown_at: Some(at), .. } if at.elapsed().as_secs_f32() > 3.0)
        });

        if loop_expired
            && let Some((_, job)) = self.loop_job.take() {
            job.task_handle.abort();
        }
    }

    pub fn update(&mut self, message: Message, data: &data::State, settings: &mut Settings, anim_state: &mut AnimState) -> Task<Message> {
        match message {
            Message::Popup(msg) => {
                if self.popup.update(msg, POPUP_SIZE) {
                    anim_state.export_popup_open = false;
                }
            }
            Message::SetMode(mode) => {
                if mode == ExportMode::Showcase {
                    self.exporter.showcase_walk_str.clear();
                    self.exporter.showcase_idle_str.clear();
                    self.exporter.showcase_attack_str.clear();
                    self.exporter.showcase_kb_str.clear();
                }
                self.exporter.export_mode = mode;
                self.maybe_scan_showcase(data, settings);
            }
            Message::SetFormat(format) => {
                self.exporter.format = format.clone();
                anim_state.last_export_format = format;
                if is_forced_opaque(&self.exporter.format) {
                    self.exporter.background = true;
                } else {
                    self.exporter.background = self.exporter.user_bg_preference;
                }
            }
            Message::SetFileName(name) => self.exporter.file_name = name,
            Message::SetStartFrame(value) => {
                self.exporter.frame_start_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.frame_start = 0;
                } else if let Ok(parsed) = value.trim().parse::<i32>() {
                    self.exporter.frame_start = parsed;
                }
            }
            Message::SetEndFrame(value) => {
                self.exporter.frame_end_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.frame_end = self.exporter.max_frame;
                } else if let Ok(parsed) = value.trim().parse::<i32>() {
                    self.exporter.frame_end = parsed;
                }
            }
            Message::SetLoopTolerance(value) => {
                self.exporter.loop_tolerance_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.loop_tolerance = parsed;
                }
            }
            Message::SetLoopMin(value) => {
                self.exporter.loop_min_str = value.clone();
                if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.loop_min = parsed;
                }
            }
            Message::SetLoopMax(value) => {
                self.exporter.loop_max = value.parse::<i32>().ok();
                self.exporter.loop_max_str = value;
            }
            Message::SetShowcaseWalk(value) => {
                self.exporter.showcase_walk_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.showcase_walk_len = self.exporter.detected_walk_len;
                } else if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.showcase_walk_len = parsed;
                }
            }
            Message::SetShowcaseIdle(value) => {
                self.exporter.showcase_idle_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.showcase_idle_len = self.exporter.detected_idle_len;
                } else if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.showcase_idle_len = parsed;
                }
            }
            Message::SetShowcaseAttack(value) => {
                self.exporter.showcase_attack_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.showcase_attack_len = self.exporter.detected_attack_len;
                } else if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.showcase_attack_len = parsed;
                }
            }
            Message::SetShowcaseKb(value) => {
                self.exporter.showcase_kb_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.showcase_kb_len = settings.animation.default_showcase_kb;
                } else if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.showcase_kb_len = parsed;
                }
            }
            Message::SetRegionX(value) => {
                if let Ok(parsed) = value.parse::<f32>() {
                    self.exporter.region_x = parsed;
                }
            }
            Message::SetRegionY(value) => {
                if let Ok(parsed) = value.parse::<f32>() {
                    self.exporter.region_y = parsed;
                }
            }
            Message::SetRegionW(value) => {
                if let Ok(parsed) = value.parse::<f32>() {
                    self.exporter.region_w = parsed;
                }
            }
            Message::SetRegionH(value) => {
                if let Ok(parsed) = value.parse::<f32>() {
                    self.exporter.region_h = parsed;
                }
            }
            Message::SetQuality(value) => {
                self.exporter.quality_percent_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.quality_percent = 100;
                    anim_state.last_export_quality = None;
                } else if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.quality_percent = parsed.clamp(0, 100);
                    anim_state.last_export_quality = Some(self.exporter.quality_percent);
                }
            }
            Message::SetCompression(value) => {
                self.exporter.compression_percent_str = value.clone();
                if value.trim().is_empty() {
                    self.exporter.compression_percent = 0;
                    anim_state.last_export_compression = None;
                } else if let Ok(parsed) = value.parse::<i32>() {
                    self.exporter.compression_percent = parsed.clamp(0, 100);
                    anim_state.last_export_compression = Some(self.exporter.compression_percent);
                }
            }
            Message::ToggleBackground(enabled) => {
                self.exporter.background = enabled;
                self.exporter.user_bg_preference = enabled;
            }
            Message::BeginExport => {
                let Some(key) = self.synced_key.clone() else {
                    return Task::none();
                };

                if matches!(
                    self.jobs.get(&key).map(|job| &job.phase),
                    Some(JobPhase::Running | JobPhase::Aborting)
                ) {
                    return Task::none();
                }

                let Some(unit) = data.held_unit.clone() else {
                    return Task::none();
                };

                if self.exporter.region_w <= 0.1 || self.exporter.region_h <= 0.1 {
                    return Task::none();
                }

                self.exporter.region_w = self.exporter.region_w.round();
                self.exporter.region_h = self.exporter.region_h.round();

                let request = self.exporter.to_request();
                let config = process::build_config(&request);
                let total_frames = (config.end_frame - config.start_frame).abs() + 1;

                let timing = FrameTiming {
                    frame_start: config.start_frame,
                    frame_end: config.end_frame,
                    ..request.timing.clone()
                };

                let abort = Arc::new(AtomicBool::new(false));
                let render_progress = Arc::new(AtomicI32::new(0));

                let (frame_tx, frame_rx) = mpsc::channel();
                let (tx, rx) = unbounded();

                let worker_abort = abort.clone();
                thread::spawn(move || {
                    leader::run(config, frame_rx, |status| {
                        let _ = tx.unbounded_send(status);
                    }, &worker_abort);
                });

                offscreen::spawn(offscreen::Job {
                    unit,
                    animation: data.current_anim.clone(),
                    available_anims: data.available_anims.clone(),
                    timing,
                    lengths: request.showcase,
                    camera: Camera {
                        region_x: self.exporter.region_x,
                        region_y: self.exporter.region_y,
                        zoom: self.exporter.zoom,
                    },
                    region_w: self.exporter.region_w,
                    region_h: self.exporter.region_h,
                    fps: self.exporter.fps,
                    background: self.exporter.background,
                    tx: frame_tx,
                    abort: abort.clone(),
                    progress: render_progress.clone(),
                });

                let map_key = key.clone();
                let (encoder_task, handle) = Task::stream(rx)
                    .map(move |status| Message::Encoder(map_key.clone(), status))
                    .abortable();

                if let Some(previous) = self.jobs.insert(key, JobState {
                    phase: JobPhase::Running,
                    abort,
                    render_progress,
                    rendered_frames: 0,
                    encoded_frames: 0,
                    total_frames,
                    encoder_handle: handle,
                }) {
                    previous.encoder_handle.abort();
                }

                return encoder_task;
            }
            Message::AbortExport => {
                if let Some(key) = &self.synced_key
                    && let Some(job) = self.jobs.get_mut(key)
                    && matches!(job.phase, JobPhase::Running) {
                    job.abort.store(true, Ordering::Relaxed);
                    job.phase = JobPhase::Aborting;
                }
            }
            Message::FindLoop => {
                let Some(key) = self.synced_key.clone() else {
                    return Task::none();
                };

                if matches!(
                    self.jobs.get(&key).map(|job| &job.phase),
                    Some(JobPhase::Running | JobPhase::Aborting)
                ) {
                    return Task::none();
                }

                if self.loop_job.as_ref().is_some_and(|(job_key, job)| {
                    *job_key == key && matches!(job.phase, LoopPhase::Running | LoopPhase::Aborting)
                }) {
                    return Task::none();
                }

                let Some(unit) = data.held_unit.clone() else {
                    return Task::none();
                };
                let Some(animation) = data.current_anim.clone() else {
                    return Task::none();
                };

                let tolerance = self.exporter.loop_tolerance as f32;
                let minimum = self.exporter.loop_min;
                let maximum = self.exporter.loop_max;

                let abort = Arc::new(AtomicBool::new(false));
                let (tx, rx) = unbounded();

                let worker_abort = abort.clone();
                thread::spawn(move || {
                    find_loop::search(&unit, &animation, tolerance, minimum, maximum, |status| {
                        let _ = tx.unbounded_send(status);
                    }, &worker_abort);
                });

                let map_key = key.clone();
                let (search_task, handle) = Task::stream(rx)
                    .map(move |status| Message::LoopSearch(map_key.clone(), status))
                    .abortable();

                if let Some((_, previous)) = self.loop_job.take() {
                    previous.abort.store(true, Ordering::Relaxed);
                    previous.task_handle.abort();
                }

                self.loop_job = Some((key, LoopJob {
                    phase: LoopPhase::Running,
                    abort,
                    frames_searched: 0,
                    start_time: Instant::now(),
                    task_handle: handle,
                }));

                return search_task;
            }
            Message::AbortLoopSearch => {
                if let Some(key) = &self.synced_key
                    && let Some((job_key, job)) = &mut self.loop_job
                    && job_key == key
                    && matches!(job.phase, LoopPhase::Running) {
                    job.abort.store(true, Ordering::Relaxed);
                    job.phase = LoopPhase::Aborting;
                }
            }
            Message::LoopSearch(key, status) => {
                let Some((job_key, job)) = &mut self.loop_job else {
                    trace!("Dropping loop search event for missing job {:?}", key);
                    return Task::none();
                };

                if *job_key != key {
                    trace!("Dropping stale loop search event for {:?}", key);
                    return Task::none();
                }

                match status {
                    LoopStatus::Searching(frames) => job.frames_searched = frames,
                    LoopStatus::Found(start, end) => {
                        let was_aborting = matches!(job.phase, LoopPhase::Aborting);
                        job.phase = LoopPhase::Done {
                            result: if was_aborting { LoopResult::Terminated } else { LoopResult::Found },
                            shown_at: None,
                        };

                        if !was_aborting && self.synced_key.as_ref() == Some(&key) {
                            self.exporter.frame_start = start;
                            self.exporter.frame_end = end;
                            self.exporter.frame_start_str = start.to_string();
                            self.exporter.frame_end_str = end.to_string();
                        }
                    }
                    LoopStatus::Error(message) => {
                        let was_aborting = matches!(job.phase, LoopPhase::Aborting);
                        job.phase = LoopPhase::Done {
                            result: if was_aborting { LoopResult::Terminated } else { LoopResult::Error(message) },
                            shown_at: None,
                        };
                    }
                }
            }
            Message::Encoder(key, status) => {
                let Some(job) = self.jobs.get_mut(&key) else {
                    trace!("Dropping encoder event for pruned export job {:?}", key);
                    return Task::none();
                };

                match status {
                    EncoderStatus::Progress(progress) => job.encoded_frames = progress as i32,
                    EncoderStatus::Finished => {
                        let result = if matches!(job.phase, JobPhase::Aborting) {
                            JobResult::Terminated
                        } else {
                            JobResult::Completed
                        };
                        job.phase = JobPhase::Done { result, shown_at: None };
                    }
                }
            }
            Message::SetCamera => {}
            Message::UseBounds => {
                let tolerance = if settings.animation.use_tight_bounds { 1.0 } else { 0.0 };
                let mut calculated = false;

                if let Some(unit) = &data.held_unit {
                    let mut showcase_clips = Vec::new();
                    let mut anim_refs: Vec<&Animation> = Vec::new();

                    if self.exporter.export_mode == ExportMode::Showcase {
                        for slot in [IDX_WALK, IDX_IDLE, IDX_ATTACK, IDX_KB] {
                            if let Some((_, path)) = data.available_anims.iter().find(|(idx, _)| *idx == slot)
                                && let Ok(bytes) = fs::read(path)
                                && let Some(anim) = Animation::parse(&bytes) {
                                showcase_clips.push(anim);
                            }
                        }
                        anim_refs.extend(showcase_clips.iter());
                    } else if let Some(anim) = &data.current_anim {
                        anim_refs.push(anim.as_ref());
                    }

                    if !anim_refs.is_empty()
                        && let Some((x, y, w, h)) = unit.calculate_bounds(&anim_refs, tolerance) {
                        self.exporter.region_x = x;
                        self.exporter.region_y = y;
                        self.exporter.region_w = w;
                        self.exporter.region_h = h;
                        self.exporter.zoom = 1.0;
                        calculated = true;
                    }
                }

                if !calculated {
                    self.exporter.region_x = 0.0;
                    self.exporter.region_y = 0.0;
                    self.exporter.region_w = 0.0;
                    self.exporter.region_h = 0.0;
                    self.exporter.zoom = 1.0;
                }
            }
        }

        Task::none()
    }

    pub fn view(&self, window: Size) -> Element<'_, Message> {
        self.popup.view("Export Animation", POPUP_SIZE, window, Message::Popup, move || self.content_view(), Some(POPUP_BODY_ALPHA))
    }

    fn content_view(&self) -> Element<'_, Message> {
        let is_avif_missing = paths::avifenc_status() != Presence::Installed;
        let is_ffmpeg_missing = paths::ffmpeg_status() != Presence::Installed;
        let current_job = self.synced_key.as_ref().and_then(|key| self.jobs.get(key));
        let job_active = matches!(
            current_job.map(|job| &job.phase),
            Some(JobPhase::Running | JobPhase::Aborting)
        );
        let is_locked = job_active;

        let current_loop = self.loop_job.as_ref()
            .filter(|(key, _)| Some(key) == self.synced_key.as_ref())
            .map(|(_, job)| job);
        let loop_active = matches!(
            current_loop.map(|job| &job.phase),
            Some(LoopPhase::Running | LoopPhase::Aborting)
        );

        let selected_mode = match self.exporter.export_mode {
            ExportMode::Manual => "Manual",
            ExportMode::Loop => "Loop",
            ExportMode::Showcase => "Showcase",
        };

        let mode_picker = field_row(
            "Mode",
            pick_list(&MODE_OPTIONS[..], Some(selected_mode), |selected: &str| {
                Message::SetMode(match selected {
                    "Loop" => ExportMode::Loop,
                    "Showcase" => ExportMode::Showcase,
                    _ => ExportMode::Manual,
                })
            })
                .width(Length::Fixed(COMBO_WIDTH))
                .style(theme::combo_box)
                .menu_style(theme::combo_box_menu),
        );

        let selected_format = match self.exporter.format {
            ExportFormat::Gif => "GIF",
            ExportFormat::WebP => "WebP",
            ExportFormat::Avif => "AVIF",
            ExportFormat::Png => "PNG",
            ExportFormat::Mp4 => "MP4",
            ExportFormat::Mkv => "MKV",
            ExportFormat::Webm => "WebM",
            ExportFormat::Zip => "ZIP",
        };

        let format_options: Vec<&str> = FORMAT_OPTIONS.iter().copied().filter(|format| {
            match *format {
                "AVIF" => !is_avif_missing,
                "MP4" | "MKV" | "WebM" | "PNG" => !is_ffmpeg_missing,
                _ => true,
            }
        }).collect();

        let format_picker = field_row(
            "Format",
            pick_list(format_options, Some(selected_format), |selected: &str| {
                Message::SetFormat(match selected {
                    "WebP" => ExportFormat::WebP,
                    "AVIF" => ExportFormat::Avif,
                    "PNG" => ExportFormat::Png,
                    "MP4" => ExportFormat::Mp4,
                    "MKV" => ExportFormat::Mkv,
                    "WebM" => ExportFormat::Webm,
                    "ZIP" => ExportFormat::Zip,
                    _ => ExportFormat::Gif,
                })
            })
                .width(Length::Fixed(COMBO_WIDTH))
                .style(theme::combo_box)
                .menu_style(theme::combo_box_menu),
        );

        let end_hint = self.exporter.max_frame.to_string();

        let input_section: Element<'_, Message> = match self.exporter.export_mode {
            ExportMode::Manual => field_row(
                "Frames",
                frame_range_row(
                    &self.exporter.frame_start_str,
                    &self.exporter.frame_end_str,
                    "0",
                    &end_hint,
                    Some(Message::SetStartFrame),
                    Some(Message::SetEndFrame),
                ),
            ),
            ExportMode::Loop => {
                let find_loop_button: Element<'_, Message> = match current_loop.map(|job| &job.phase) {
                    Some(LoopPhase::Running | LoopPhase::Aborting) => {
                        action_button("Abort Loop", theme::danger_button, Some(Message::AbortLoopSearch))
                    }
                    phase => {
                        let is_terminated = matches!(phase, Some(LoopPhase::Done { result: LoopResult::Terminated, .. }));
                        let label = if is_terminated { "Loop Terminated!" } else { "Find Loop" };
                        let style = if is_terminated { theme::danger_button } else { theme::neutral_button };
                        action_button(label, style, (!job_active).then_some(Message::FindLoop))
                    }
                };

                column![
                    field_row(
                        "Frames",
                        frame_range_row(
                            &self.exporter.frame_start_str,
                            &self.exporter.frame_end_str,
                            "0",
                            &end_hint,
                            Some(Message::SetStartFrame),
                            Some(Message::SetEndFrame),
                        ),
                    ),
                    field_row("Tolerance %", small_input("30", &self.exporter.loop_tolerance_str, Message::SetLoopTolerance)),
                    field_row("Min Frames", small_input("15", &self.exporter.loop_min_str, Message::SetLoopMin)),
                    field_row("Max Frames", small_input("None", &self.exporter.loop_max_str, Message::SetLoopMax)),
                    find_loop_button,
                ].spacing(ROW_SPACING).into()
            }
            ExportMode::Showcase => {
                let walk_hint = self.exporter.detected_walk_len.to_string();
                let idle_hint = self.exporter.detected_idle_len.to_string();
                let attack_hint = self.exporter.detected_attack_len.to_string();
                let kb_hint = self.exporter.last_known_kb_default.to_string();

                column![
                    field_row("Walk", small_input(&walk_hint, &self.exporter.showcase_walk_str, Message::SetShowcaseWalk)),
                    field_row("Idle", small_input(&idle_hint, &self.exporter.showcase_idle_str, Message::SetShowcaseIdle)),
                    field_row("Attack", small_input(&attack_hint, &self.exporter.showcase_attack_str, Message::SetShowcaseAttack)),
                    field_row("Knockback", small_input(&kb_hint, &self.exporter.showcase_kb_str, Message::SetShowcaseKb)),
                ].spacing(ROW_SPACING).into()
            }
        };

        let camera_buttons = row![
            action_button("Set Camera", theme::neutral_button, (!is_locked).then_some(Message::SetCamera)),
            action_button("Use Bounds", theme::neutral_button, (!is_locked).then_some(Message::UseBounds)),
        ].spacing(FIELD_SPACING);

        let camera_section = section(
            "Camera",
            Length::Fill,
            column![
                camera_buttons,
                row![
                    axis_input("X", self.exporter.region_x, Message::SetRegionX),
                    axis_input("Y", self.exporter.region_y, Message::SetRegionY),
                ].spacing(FIELD_SPACING),
                row![
                    axis_input("W", self.exporter.region_w, Message::SetRegionW),
                    axis_input("H", self.exporter.region_h, Message::SetRegionH),
                ].spacing(FIELD_SPACING),
            ].spacing(ROW_SPACING),
        );

        let (display_start, display_end) = if self.exporter.export_mode == ExportMode::Showcase {
            let total = self.exporter.showcase_walk_len
                + self.exporter.showcase_idle_len
                + self.exporter.showcase_attack_len
                + self.exporter.showcase_kb_len;
            (0, if total > 0 { total - 1 } else { 0 })
        } else {
            (self.exporter.frame_start, self.exporter.frame_end)
        };

        let range_part = if display_start == display_end {
            format!("{}f", display_start)
        } else {
            format!("{}f~{}f", display_start, display_end)
        };

        let clean_prefix = self.exporter.name_prefix
            .replace("_0", "")
            .replace("_f", "-1")
            .replace("_c", "-2")
            .replace("_s", "-3");

        let prefix_display = if self.exporter.export_mode == ExportMode::Showcase {
            clean_prefix.split('.').next()
                .map_or_else(|| "unit.showcase".to_string(), |first| format!("{}.showcase", first))
        } else {
            clean_prefix
        };

        let name_hint = if prefix_display.is_empty() {
            "animation".to_string()
        } else {
            format!("{}.{}", prefix_display, range_part)
        };

        let output_section = section(
            "Output",
            Length::Fill,
            column![
                field_row(
                    "Name",
                    text_input(&name_hint, &self.exporter.file_name)
                        .on_input(Message::SetFileName)
                        .width(Length::Fixed(NAME_INPUT_WIDTH))
                        .style(theme::rounded_input),
                ),
                format_picker,
                field_row("Quality %", small_input("80", &self.exporter.quality_percent_str, Message::SetQuality)),
                field_row("Compress %", small_input("30", &self.exporter.compression_percent_str, Message::SetCompression)),
                row![
                    background_toggle(&self.exporter),
                    text("Background").size(CONTROL_TEXT_SIZE),
                ].spacing(FIELD_SPACING).align_y(Alignment::Center),
            ].spacing(ROW_SPACING),
        );

        let addons_section = section(
            "Add-Ons",
            Length::Fill,
            column![
                text("Tools that enhance the Exporter\nManage through Settings > Add-Ons").size(CONTROL_TEXT_SIZE),
                addon_badge("FFMPEG", !is_ffmpeg_missing),
                addon_badge("AVIFENC", !is_avif_missing),
            ].spacing(FIELD_SPACING),
        );

        let (ratio, status_label) = if let Some(job) = current_loop.filter(|job| matches!(job.phase, LoopPhase::Running | LoopPhase::Aborting)) {
            let pulse = job.start_time.elapsed().as_secs_f32() % 1.0;
            (pulse, format!("Searching | {} frames", job.frames_searched))
        } else if let Some(job) = current_job {
            let progress_ratio = |value: i32| (value as f32 / job.total_frames.max(1) as f32).min(1.0);

            match &job.phase {
                JobPhase::Running => {
                    if job.rendered_frames < job.total_frames {
                        let ratio = progress_ratio(job.rendered_frames);
                        (ratio, format!("Rendering | {}f/{}f ({}%)", job.rendered_frames, job.total_frames, (ratio * 100.0) as i32))
                    } else {
                        let ratio = progress_ratio(job.encoded_frames);
                        (ratio, format!("Encoding | {}f/{}f ({}%)", job.encoded_frames, job.total_frames, (ratio * 100.0) as i32))
                    }
                }
                JobPhase::Aborting => (1.0, "Aborting...".to_string()),
                JobPhase::Done { result: JobResult::Completed, .. } => (1.0, "Done".to_string()),
                JobPhase::Done { result: JobResult::Terminated, .. } => (1.0, "Ready".to_string()),
            }
        } else if let Some(LoopPhase::Done { result, .. }) = current_loop.map(|job| &job.phase) {
            let label = match result {
                LoopResult::Found => "Done".to_string(),
                LoopResult::Terminated => "Loop Terminated!".to_string(),
                LoopResult::Error(message) => message.clone(),
            };
            (1.0, label)
        } else {
            (1.0, "Ready".to_string())
        };

        let export_button: Element<'_, Message> = match current_job.map(|job| &job.phase) {
            Some(JobPhase::Running) => action_button("Abort Export", theme::danger_button, Some(Message::AbortExport)),
            Some(JobPhase::Aborting) => action_button("Aborting...", theme::danger_button, None),
            phase => {
                let is_valid = self.exporter.region_w > 0.1 && self.exporter.region_h > 0.1;
                let is_terminated = matches!(phase, Some(JobPhase::Done { result: JobResult::Terminated, .. }));

                let label = if is_terminated {
                    "Export Terminated!"
                } else if is_valid {
                    "Begin Export"
                } else {
                    "No Camera Set"
                };

                let style = match (is_terminated, is_valid) {
                    (true, _) => theme::danger_button,
                    (false, true) => theme::primary_button,
                    (false, false) => theme::neutral_button,
                };
                let press = (is_valid && !is_locked && !loop_active).then_some(Message::BeginExport);

                action_button(label, style, press)
            }
        };

        let scroll_content = column![
            section("Input", Length::Fill, column![mode_picker, input_section].spacing(ROW_SPACING)),
            camera_section,
            output_section,
            addons_section,
        ].spacing(SECTION_SPACING);

        let bottom_bar = column![
            export_button,
            column![
                theme::centered_text(status_label).size(CONTROL_TEXT_SIZE).width(Length::Fill),
                progress_bar(0.0..=1.0, ratio),
            ].spacing(ROW_SPACING),
        ].spacing(BUTTON_STATUS_GAP);

        container(
            column![
                smooth_scroll(scrollable(scroll_content).height(Length::Fill).spacing(SCROLLBAR_GAP)),
                container(rule::horizontal(RULE_HEIGHT)).width(Length::Fill),
                bottom_bar,
            ].spacing(SECTION_SPACING)
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(CONTENT_PADDING)
            .into()
    }
}

fn field_row<'a>(label: &'a str, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    row![
        text(label).size(CONTROL_TEXT_SIZE).width(Length::Fixed(FIELD_LABEL_WIDTH)),
        control.into(),
    ]
        .spacing(FIELD_SPACING)
        .align_y(Alignment::Center)
        .into()
}

fn small_input<'a>(hint: &str, value: &'a str, on_input: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    text_input(hint, value)
        .on_input(on_input)
        .width(Length::Fixed(SMALL_INPUT_WIDTH))
        .style(theme::rounded_input)
        .into()
}

fn axis_input<'a>(label: &'a str, value: f32, on_input: impl Fn(String) -> Message + 'a) -> Element<'a, Message> {
    row![
        text(label).size(CONTROL_TEXT_SIZE).width(Length::Fixed(AXIS_LABEL_WIDTH)),
        text_input("0", &value.to_string())
            .on_input(on_input)
            .width(Length::Fixed(AXIS_INPUT_WIDTH))
            .style(theme::rounded_input),
    ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into()
}

fn frame_range_row<'a, FMin, FMax>(
    min: &'a str,
    max: &'a str,
    min_hint: &str,
    max_hint: &str,
    on_min: Option<FMin>,
    on_max: Option<FMax>,
) -> Element<'a, Message>
where
    FMin: Fn(String) -> Message + 'a,
    FMax: Fn(String) -> Message + 'a,
{
    row![
        text_input(min_hint, min).on_input_maybe(on_min).width(Length::Fixed(SMALL_INPUT_WIDTH)).style(theme::rounded_input),
        text("~").size(CONTROL_TEXT_SIZE),
        text_input(max_hint, max).on_input_maybe(on_max).width(Length::Fixed(SMALL_INPUT_WIDTH)).style(theme::rounded_input),
    ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into()
}

fn action_button<'a>(
    label: &'a str,
    style: fn(&Theme, button::Status) -> button::Style,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    button(theme::button_label(label).size(CONTROL_TEXT_SIZE))
        .width(Length::Fill)
        .padding([6, 10])
        .style(style)
        .on_press_maybe(on_press)
        .into()
}

fn is_forced_opaque(format: &ExportFormat) -> bool {
    matches!(format, ExportFormat::Mp4 | ExportFormat::Mkv | ExportFormat::Webm)
}

fn background_toggle(exporter: &ExportForm) -> Element<'_, Message> {
    if is_forced_opaque(&exporter.format) {
        tooltip(
            toggler(true).style(theme::ios_toggle),
            container(text("This video format requires a background")).padding(6).style(container::bordered_box),
            tooltip::Position::Top,
        )
        .into()
    } else {
        toggler(exporter.background).on_toggle(Message::ToggleBackground).style(theme::ios_toggle).into()
    }
}

fn addon_badge(label: &str, installed: bool) -> Element<'_, Message> {
    let status = if installed { format!("{label} Installed") } else { format!("{label} Missing") };

    container(theme::centered_text(status).size(CONTROL_TEXT_SIZE))
        .width(Length::Fill)
        .padding(6)
        .style(move |theme: &Theme| theme::status_badge(theme, installed))
        .into()
}

fn derive_name_prefix(raw_id: &str, anim_index: usize) -> String {
    let type_string = match anim_index {
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

    let id_parts: Vec<&str> = raw_id.split('_').collect();
    let mut clean_id = id_parts.first().copied().unwrap_or("").to_string();

    if id_parts.len() >= 2 && !clean_id.is_empty() && clean_id.chars().all(char::is_numeric) {
        let form_number = match id_parts[1].chars().next() {
            Some('f') => 1,
            Some('c') => 2,
            Some('s') => 3,
            Some('u') => 4,
            _ => 0,
        };
        if form_number > 0 {
            clean_id = format!("{}-{}", clean_id, form_number);
        }
    }

    format!("{}.{}", clean_id, type_string)
}

