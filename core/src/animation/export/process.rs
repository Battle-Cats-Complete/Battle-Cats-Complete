use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};

use nyanko::graphics::rig::Animation;

use super::encoding::{EncoderStatus, ExportConfig, ExportFormat};
use super::leader;
use super::state::{ExportMode, ExporterState};

pub static STATUS_RX: Mutex<Option<mpsc::Receiver<EncoderStatus>>> = Mutex::new(None);

pub fn start_export(state: &mut ExporterState) {
    if state.is_processing { return; }

    state.is_processing = true;
    state.current_progress = 0;
    state.encoded_frames = 0;
    state.completion_time = None;

    let abort_signal = Arc::new(AtomicBool::new(false));
    state.abort = Some(abort_signal.clone());

    if state.export_mode == ExportMode::Showcase {
        state.frame_start = 0;
        let total_frames = state.showcase_walk_len + state.showcase_idle_len + state.showcase_attack_len + state.showcase_kb_len;
        state.frame_end = if total_frames > 0 { total_frames - 1 } else { 0 };
    }

    let (base_name, file_name) = if state.file_name.trim().is_empty() {
        let (display_start, display_end) = if state.export_mode == ExportMode::Showcase {
            let total_frames = state.showcase_walk_len + state.showcase_idle_len + state.showcase_attack_len + state.showcase_kb_len;
            let end_display = if total_frames > 0 { total_frames - 1 } else { 0 };
            (0, end_display)
        } else {
            (state.frame_start, state.frame_end)
        };

        let range_part = if display_start == display_end {
            format!("{}f", display_start)
        } else {
            format!("{}f~{}f", display_start, display_end)
        };

        let clean_prefix = state.name_prefix.replace("_0", "").replace("_f", "-1").replace("_c", "-2").replace("_s", "-3");

        let prefix_display = if state.export_mode == ExportMode::Showcase {
            let prefix_parts: Vec<&str> = clean_prefix.split('.').collect();
            if !prefix_parts.is_empty() {
                format!("{}.showcase", prefix_parts[0])
            } else {
                "unit.showcase".to_string()
            }
        } else {
            clean_prefix.clone()
        };

        let base = if prefix_display.is_empty() { "animation".to_string() } else { prefix_display };
        let full = format!("{}.{}", base, range_part);
        (base, full)
    } else {
        let path_object = Path::new(&state.file_name);
        let base = path_object.file_stem().unwrap_or(path_object.as_os_str()).to_string_lossy().to_string();
        (base, state.file_name.clone())
    };

    let mut output_path = std::env::current_dir().unwrap_or(PathBuf::from("."));
    output_path.push("exports");

    let mut final_file_name = file_name;

    let target_extension = match state.format {
        ExportFormat::Gif => Some("gif"),
        ExportFormat::WebP => Some("webp"),
        ExportFormat::Avif => Some("avif"),
        ExportFormat::Png => Some("png"),
        ExportFormat::Mp4 => Some("mp4"),
        ExportFormat::Mkv => Some("mkv"),
        ExportFormat::Webm => Some("webm"),
        ExportFormat::Zip => Some("zip"),
    };

    if let Some(extension) = target_extension {
        let suffix = format!(".{}", extension);
        if !final_file_name.to_lowercase().ends_with(&suffix) {
            final_file_name = format!("{}{}", final_file_name, suffix);
        }
    }

    output_path.push(final_file_name);

    let config = ExportConfig {
        width: state.region_w as u32, height: state.region_h as u32,
        camera_x: state.region_x, camera_y: state.region_y, camera_zoom: state.zoom,
        format: state.format.clone(),
        quality_percent: state.quality_percent as u32,
        compression_percent: state.compression_percent as u32,
        fps: state.fps as u32,
        start_frame: state.frame_start, end_frame: state.frame_end, interpolation: state.interpolation,
        output_path,
        base_name,
        background: state.background,
    };

    let (sender, receiver) = mpsc::channel();
    let (status_sender, status_receiver) = mpsc::channel();

    if let Ok(mut lock) = STATUS_RX.lock() { *lock = Some(status_receiver); }

    state.tx = Some(sender);
    leader::start_encoding_thread(config, receiver, status_sender, abort_signal);
}

pub fn calculate_export_time(
    state: &ExporterState,
    animation_option: Option<&Animation>,
    current_time: f32,
) -> f32 {
    let Some(animation) = animation_option else {
        return 0.0;
    };

    let raw_frame = if state.export_mode == ExportMode::Showcase {
        current_time
    } else {
        let start = state.frame_start;
        let step = if state.frame_start < state.frame_end { 1 } else { -1 };
        (start + (state.current_progress * step)) as f32
    };

    if state.export_mode == ExportMode::Showcase {
        let natively_loops = animation.curves.iter().any(|c| c.loop_count != 1);

        if natively_loops {
            raw_frame
        } else {
            let effective_maximum = animation.curves.iter()
                .filter_map(|c| c.keyframes.last().map(|k| k.frame))
                .max()
                .unwrap_or(0);

            if effective_maximum > 0 {
                raw_frame.rem_euclid(effective_maximum as f32 + 1.0)
            } else {
                raw_frame
            }
        }
    } else if state.loop_supported {
        raw_frame
    } else if state.max_frame > 0 {
        raw_frame.rem_euclid(state.max_frame as f32 + 1.0)
    } else {
        raw_frame
    }
}