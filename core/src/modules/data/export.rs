use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::common::job::JobEvent;

pub fn create_game_archive(
    emit: impl Fn(JobEvent) + Sync,
    abort_flag: &AtomicBool,
    compression_level: i32,
    filename: String,
    include_raw: bool,
) -> Result<(), String> {
    let game_root_path = Path::new("game");
    let export_directory = Path::new("exports");

    let final_filename = if filename.ends_with(".tar.zst") {
        filename
    } else {
        let clean_name = filename
            .trim_end_matches(".zip")
            .trim_end_matches(".game")
            .trim_end_matches(".tar.zst");

        format!("{}.tar.zst", clean_name)
    };

    let archive_path = export_directory.join(final_filename);

    if !game_root_path.exists() {
        return Err("No 'game' folder found to export.".to_string());
    }

    if !export_directory.exists() {
        fs::create_dir_all(export_directory).map_err(|e| e.to_string())?;
    }

    emit(JobEvent::Log("Estimating bundle size...".to_string()));
    let mut total_files: usize = 0;
    let mut directory_stack = vec![game_root_path.to_path_buf()];

    while let Some(current_directory) = directory_stack.pop() {
        if abort_flag.load(Ordering::Relaxed) {
            return Err("Job Aborted".to_string());
        }

        if let Ok(directory_entries) = fs::read_dir(&current_directory) {
            for entry_result in directory_entries.flatten() {
                let path = entry_result.path();
                let path_string = path.to_string_lossy();

                if !include_raw && (path_string.contains("game/raw") || path_string.contains("game\\raw")) {
                    continue;
                }

                if path.is_dir() {
                    directory_stack.push(path);
                } else {
                    total_files += 1;
                }
            }
        }
    }

    let update_interval = (total_files / 100).max(10);
    let progress_step = (total_files / 100).max(1);
    emit(JobEvent::Progress { current: 0, total: total_files });

    let thread_count = match thread::available_parallelism() {
        Ok(n) => n.get() as u32,
        Err(_) => 4,
    };

    emit(JobEvent::Log(format!(
        "Starting Multi-Threaded Compression ({} threads)...",
        thread_count
    )));

    let file_handle = fs::File::create(&archive_path).map_err(|e| e.to_string())?;
    let mut zstd_encoder = zstd::stream::write::Encoder::new(file_handle, compression_level)
        .map_err(|e| e.to_string())?;

    zstd_encoder.multithread(thread_count).map_err(|e| e.to_string())?;
    zstd_encoder.include_checksum(true).map_err(|e| e.to_string())?;

    let mut tar_builder = tar::Builder::new(zstd_encoder);
    let mut processed_file_count: usize = 0;

    let mut export_stack = vec![game_root_path.to_path_buf()];

    while let Some(current_directory) = export_stack.pop() {
        if abort_flag.load(Ordering::Relaxed) {
            return Err("Job Aborted".to_string());
        }

        let directory_entries = match fs::read_dir(&current_directory) {
            Ok(iter) => iter,
            Err(_) => continue,
        };

        for entry_result in directory_entries.flatten() {
            if abort_flag.load(Ordering::Relaxed) {
                return Err("Job Aborted".to_string());
            }

            let file_path = entry_result.path();
            let path_string = file_path.to_string_lossy();

            if !include_raw && (path_string.contains("game/raw") || path_string.contains("game\\raw")) {
                continue;
            }

            if file_path.is_dir() {
                export_stack.push(file_path.clone());

                let Ok(relative_directory_name) = file_path.strip_prefix(game_root_path) else {
                    continue;
                };

                let _ = tar_builder.append_dir(relative_directory_name, &file_path);
                continue;
            }

            let Ok(relative_file_name) = file_path.strip_prefix(game_root_path) else {
                continue;
            };

            let mut active_file_handle = match fs::File::open(&file_path) {
                Ok(f) => f,
                Err(_) => continue,
            };

            if tar_builder.append_file(relative_file_name, &mut active_file_handle).is_ok() {
                processed_file_count += 1;

                if processed_file_count.is_multiple_of(progress_step) || processed_file_count == total_files {
                    emit(JobEvent::Progress { current: processed_file_count, total: total_files });
                }

                if processed_file_count.is_multiple_of(update_interval) {
                    let simple_filename = file_path.file_name().unwrap_or_default().to_string_lossy();
                    emit(JobEvent::Log(format!(
                        "Packed {} files | Current: {}",
                        processed_file_count, simple_filename
                    )));
                }
            }
        }
    }

    let final_encoder = tar_builder.into_inner().map_err(|e| e.to_string())?;
    let _ = final_encoder.finish().map_err(|e| e.to_string())?;

    emit(JobEvent::Log(format!(
        "Success! Exported {} files to {:?}",
        processed_file_count, archive_path
    )));

    Ok(())
}
