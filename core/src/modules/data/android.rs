use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread;

use crate::modules::addons::adb::bridge;
use crate::modules::settings::logic::state::EmulatorConfig;

use super::engine;
use super::engine::keys;
use super::{AdbImportType, AdbTarget};

pub fn run(
    status_sender: Sender<String>,
    import_mode: AdbImportType,
    target_region: AdbTarget,
    emulator_config: EmulatorConfig,
    enforce_validation: bool,
    abort_flag: Arc<AtomicBool>,
    job_status: Arc<AtomicU8>,
    progress_current: Arc<AtomicUsize>,
    progress_maximum: Arc<AtomicUsize>,
) {
    let _ = thread::Builder::new()
        .name("android_import_worker".to_string())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let terminate = |status: Arc<AtomicU8>, is_err: bool| {
                status.store(if is_err { 3 } else { 2 }, Ordering::Relaxed);
            };

            if keys::verify(enforce_validation, &status_sender).is_err() {
                return terminate(job_status, true);
            }

            let app_repository = PathBuf::from("game/app");

            let pull_result = bridge::execute_pull(
                &app_repository,
                import_mode,
                target_region,
                &emulator_config,
                &status_sender,
                &abort_flag,
            );

            if abort_flag.load(Ordering::Relaxed) {
                return terminate(job_status, true);
            }

            let pulled_directories = match pull_result {
                Ok(dirs) => dirs,
                Err(bridge_error) => {
                    let _ = status_sender.send(format!("ADB Pull Failed: {}", bridge_error));
                    return terminate(job_status, true);
                }
            };

            let _ = status_sender.send("Starting Processing Phase...".to_string());

            let engine_result = engine::run_universal_import(
                &pulled_directories,
                &status_sender,
                &abort_flag,
                &progress_current,
                &progress_maximum,
            );

            if let Err(engine_error) = engine_result {
                let _ = status_sender.send(format!("Universal Import Failed: {}", engine_error));
                return terminate(job_status, true);
            }

            if !emulator_config.keep_app_folder {
                let _ = status_sender.send("Cleaning up app package files...".to_string());
                for directory in pulled_directories {
                    let _ = std::fs::remove_dir_all(directory);
                }
            }

            let _ = status_sender.send("All Operations Complete!".to_string());
            terminate(job_status, false);
        });
}