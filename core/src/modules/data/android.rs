use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::common::job::JobEvent;
use crate::modules::addons::adb::bridge;
use crate::modules::settings::EmulatorConfig;

use super::engine;
use super::engine::keys;
use super::{AdbImportType, AdbTarget};

pub fn run(
    import_mode: AdbImportType,
    target_region: AdbTarget,
    emulator_config: EmulatorConfig,
    enforce_validation: bool,
    emit: impl Fn(JobEvent) + Sync,
    abort_flag: &AtomicBool,
) -> Result<(), String> {
    let emit_log = |line: String| emit(JobEvent::Log(line));

    keys::verify(enforce_validation, &emit_log)?;

    let app_repository = PathBuf::from("game/app");

    let pulled_directories = bridge::execute_pull(
        &app_repository,
        import_mode,
        target_region,
        &emulator_config,
        &emit_log,
        abort_flag,
    )
    .map_err(|bridge_error| format!("ADB Pull Failed: {}", bridge_error))?;

    if abort_flag.load(Ordering::Relaxed) {
        return Err("Job Aborted".to_string());
    }

    emit(JobEvent::Log("Starting Processing Phase...".to_string()));

    engine::run_universal_import(&pulled_directories, &emit, abort_flag)
        .map_err(|engine_error| format!("Universal Import Failed: {}", engine_error))?;

    if !emulator_config.keep_app_folder {
        emit(JobEvent::Log("Cleaning up app package files...".to_string()));
        for directory in pulled_directories {
            let _ = fs::remove_dir_all(directory);
        }
    }

    emit(JobEvent::Log("All Operations Complete!".to_string()));
    Ok(())
}
