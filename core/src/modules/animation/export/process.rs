use std::env;
use std::path::{Path, PathBuf};

use nyanko::graphics::rig::Animation;
use tracing::info;

use super::{ExportConfig, ExportFormat, ExportMode, ExportRequest, FrameTiming};

pub fn build_config(request: &ExportRequest) -> ExportConfig {
    let is_showcase = request.timing.mode == ExportMode::Showcase;

    let (start_frame, end_frame) = if is_showcase {
        let total_frames = request.showcase.walk + request.showcase.idle + request.showcase.attack + request.showcase.kb;
        (0, if total_frames > 0 { total_frames - 1 } else { 0 })
    } else {
        (request.timing.frame_start, request.timing.frame_end)
    };

    let (base_name, file_name) = match &request.file_name {
        None => {
            let range_part = if start_frame == end_frame {
                format!("{}f", start_frame)
            } else {
                format!("{}f~{}f", start_frame, end_frame)
            };

            let clean_prefix = request.name_prefix.replace("_0", "").replace("_f", "-1").replace("_c", "-2").replace("_s", "-3");

            let prefix_display = if is_showcase {
                clean_prefix.split('.').next().map_or_else(|| "unit.showcase".to_string(), |first| format!("{}.showcase", first))
            } else {
                clean_prefix
            };

            let base = if prefix_display.is_empty() { "animation".to_string() } else { prefix_display };
            let full = format!("{}.{}", base, range_part);
            (base, full)
        }
        Some(name) => {
            let path_object = Path::new(name);
            let base = path_object.file_stem().unwrap_or(path_object.as_os_str()).to_string_lossy().to_string();
            (base, name.clone())
        }
    };

    let mut output_path = env::current_dir().unwrap_or_else(|_| PathBuf::from("../../../../.."));
    output_path.push("exports");

    let mut final_file_name = file_name;

    let target_extension = match request.format {
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

    output_path.push(&final_file_name);
    info!("Starting export process targeted to: {:?}", output_path);

    ExportConfig {
        width: request.width,
        height: request.height,
        format: request.format.clone(),
        quality_percent: request.quality_percent,
        compression_percent: request.compression_percent,
        fps: request.fps,
        start_frame,
        end_frame,
        output_path,
        base_name,
        background: request.background,
    }
}

pub fn calculate_export_time(
    timing: &FrameTiming,
    animation_option: Option<&Animation>,
    current_time: f32,
    current_progress: i32,
) -> f32 {
    let Some(animation) = animation_option else {
        return 0.0;
    };

    let is_showcase = timing.mode == ExportMode::Showcase;

    let raw_frame = if is_showcase {
        current_time
    } else {
        let start = timing.frame_start;
        let step = if timing.frame_start < timing.frame_end { 1 } else { -1 };
        (start + (current_progress * step)) as f32
    };

    let loop_boundary = animation.calculate_true_loop().unwrap_or(animation.max_frame);

    if is_showcase {
        let natively_loops = animation.curves.iter().any(|c| c.loop_count != 1);

        if natively_loops {
            raw_frame
        } else if loop_boundary > 0 {
            raw_frame.rem_euclid(loop_boundary as f32 + 1.0)
        } else {
            raw_frame
        }
    } else if timing.loop_supported {
        raw_frame
    } else if loop_boundary > 0 {
        raw_frame.rem_euclid(loop_boundary as f32 + 1.0)
    } else {
        raw_frame
    }
}