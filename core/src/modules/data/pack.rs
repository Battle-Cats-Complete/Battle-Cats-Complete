use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use crate::common::job::JobEvent;

use super::engine;
use super::engine::keys;
use super::{AdbTarget, ImportMode};

pub fn run(
    source_path_string: &str,
    import_mode: ImportMode,
    _target_region: AdbTarget,
    enforce_validation: bool,
    emit: impl Fn(JobEvent) + Sync,
    abort_flag: &AtomicBool,
) -> Result<(), String> {
    let emit_log = |line: String| emit(JobEvent::Log(line));

    if keys::verify(enforce_validation, &emit_log).is_err() {
        return Err("Decryption blocked: Invalid signature keys detected.".to_string());
    }

    let source_directory = match import_mode {
        ImportMode::Folder => PathBuf::from(source_path_string),
        ImportMode::Zip => {
            emit(JobEvent::Log("Extracting archive to temporary workspace...".to_string()));
            PathBuf::from("temp_workspace")
        }
        _ => return Err("Invalid Import Mode selected.".to_string()),
    };

    let directories_to_process = vec![source_directory.clone()];

    let engine_result = engine::run_universal_import(&directories_to_process, &emit, abort_flag);

    if import_mode == ImportMode::Zip {
        let _ = fs::remove_dir_all(source_directory);
    }

    engine_result
}
