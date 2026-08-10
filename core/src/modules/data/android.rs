use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::addons::adb::bridge;
use crate::common::job::{JobEvent, ProgressCounter};
use crate::modules::settings::{EmulatorConfig, ImportConfig};

use super::architecture;
use super::engine;
use super::engine::keys;
use super::{AdbImportType, AdbTarget};

pub fn run(
    import_mode: AdbImportType,
    target_region: AdbTarget,
    emulator_config: EmulatorConfig,
    import_config: ImportConfig,
    emit: impl Fn(JobEvent) + Sync,
    abort_flag: &AtomicBool,
    progress: &ProgressCounter,
) -> Result<(), String> {
    let emit_log = |line: String| emit(JobEvent::Log(line));

    keys::verify(import_config.enforce_validation, &emit_log)?;

    let app_repository = PathBuf::from(architecture::APP);

    let pull_options = bridge::PullOptions {
        import_mode,
        ignore_modified_app: import_config.ignore_modified_app,
    };

    let pulled_directories = bridge::execute_pull(
        &app_repository,
        pull_options,
        target_region,
        &emit_log,
        abort_flag,
    )
    .map_err(|bridge_error| format!("ADB Pull Failed: {}", bridge_error))?;

    if abort_flag.load(Ordering::Relaxed) {
        return Err("Job Aborted".to_string());
    }

    emit(JobEvent::Log("Starting Processing Phase...".to_string()));

    engine::run_universal_import(&pulled_directories, import_config.structure, &emit, abort_flag, progress)
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
