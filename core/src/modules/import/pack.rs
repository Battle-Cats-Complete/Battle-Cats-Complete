use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use crate::common::job::{JobEvent, ProgressCounter};
use crate::modules::settings::ImportConfig;

use super::engine;
use super::engine::keys;
use super::ImportMode;

pub fn run(
    source_path_string: &str,
    import_mode: ImportMode,
    import_config: ImportConfig,
    emit: impl Fn(JobEvent) + Sync,
    abort_flag: &AtomicBool,
    progress: &ProgressCounter,
) -> Result<(), String> {
    let emit_log = |line: String| emit(JobEvent::Log(line));

    if keys::verify(import_config.enforce_validation, &emit_log).is_err() {
        return Err("Decryption blocked: Invalid signature keys detected.".to_string());
    }

    let source_directory = match import_mode {
        ImportMode::Folder => PathBuf::from(source_path_string),
        ImportMode::Zip => {
            emit(JobEvent::Log("Extracting archive to temporary workspace...".to_string()));
            PathBuf::from("temp_workspace")
        }
    };

    let directories_to_process = vec![source_directory.clone()];

    let engine_result = engine::run_universal_import(&directories_to_process, import_config.structure, &emit, abort_flag, progress);

    if import_mode == ImportMode::Zip {
        let _ = fs::remove_dir_all(source_directory);
    }

    engine_result
}
