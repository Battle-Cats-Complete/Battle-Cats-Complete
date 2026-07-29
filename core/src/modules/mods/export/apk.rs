use std::fs;
use std::path::PathBuf;

use resand::res_value::ResValueType;
use tracing::{debug, error, info, info_span};
use zip::ZipArchive;

use crate::common::job::JobEvent;
use crate::common::region::Region;
use crate::modules::addons::apkeditor::xapk;
use crate::modules::data::engine::keys;
use crate::modules::settings::ExportBehavior;

use super::{modify, pack, sign};

#[allow(clippy::too_many_arguments)]
pub fn run(
    mod_folder: String,
    input_apk_path: PathBuf,
    app_title: String,
    suffix: String,
    target_region: Region,
    export_behavior: ExportBehavior,
    enforce_keys: bool,
    emit: impl Fn(JobEvent) + Sync,
) -> Result<(), String> {
    let _span = info_span!("export_worker", apk = %input_apk_path.display()).entered();

    let log_callback = |message: String| {
        info!("Export UI Log: {}", message);
        emit(JobEvent::Log(message));
    };

    debug!("Verifying encryption keys. Enforce: {}", enforce_keys);
    let user_keys = match keys::verify(enforce_keys, &log_callback) {
        Ok(keys) => keys,
        Err(error) => {
            error!("Key verification failed: {}", error);
            return Err(error);
        }
    };

    let base_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("../../../../../../../../.."));
    let mod_dir = base_dir.join("mods").join(&mod_folder);
    let export_dir = base_dir.join("exports");
    let app_dir = mod_dir.join("app");
    let temp_bin_dir = app_dir.join("binaries");
    let assets_dir = app_dir.join("assets");
    let xapk_dir = app_dir.join("xapk");
    let icons_dir = mod_dir.join("icons");

    debug!("Preparing file structure in {}", app_dir.display());
    let _ = fs::remove_dir_all(&app_dir);
    let _ = fs::create_dir_all(&export_dir);
    let _ = fs::create_dir_all(&temp_bin_dir);
    let _ = fs::create_dir_all(&assets_dir);

    let mut working_apk = input_apk_path.clone();
    let extension = input_apk_path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    if extension == "xapk" || extension == "apkm" || extension == "apks" {
        log_callback("Merging split APKs...".to_string());
        let _ = fs::create_dir_all(&xapk_dir);
        let merged_temp_path = xapk_dir.join("merged_xapk.apk");

        if let Err(error) = xapk::merge_xapk(&working_apk, &merged_temp_path, &log_callback) {
            error!("XAPK Merge failed: {}", error);
            return Err(error.to_string());
        }
        working_apk = merged_temp_path;
    }

    log_callback("Analyzing APK identity...".to_string());
    let manifest_path = temp_bin_dir.join("AndroidManifest.xml");
    let arsc_path = temp_bin_dir.join("resources.arsc");
    let mut extracted_arsc = false;

    let source_file = match fs::File::open(&working_apk) {
        Ok(file) => file,
        Err(error) => {
            error!("Failed to open APK {:?}: {}", working_apk, error);
            return Err(format!("Failed to open APK: {}", error));
        }
    };

    let mut archive = match ZipArchive::new(source_file) {
        Ok(archive_instance) => archive_instance,
        Err(error) => {
            error!("Failed to read APK archive: {}", error);
            return Err(format!("Failed to read APK archive: {}", error));
        }
    };

    debug!("Extracting Manifest & ARSC for look-ahead check...");
    for index in 0..archive.len() {
        let Ok(mut archive_file) = archive.by_index(index) else { continue; };
        let file_name = archive_file.name();

        if file_name == "AndroidManifest.xml" {
            if let Ok(mut output_file) = fs::File::create(&manifest_path) {
                let _ = std::io::copy(&mut archive_file, &mut output_file);
            }
        } else if file_name == "resources.arsc"
            && let Ok(mut output_file) = fs::File::create(&arsc_path) {
            let _ = std::io::copy(&mut archive_file, &mut output_file);
            extracted_arsc = true;
        }
    }
    drop(archive);

    let mut apk_editor = match modify::ApkEditor::from_paths(&manifest_path, if extracted_arsc { Some(arsc_path.as_path()) } else { None }) {
        Ok(editor) => editor,
        Err(error) => {
            error!("APK Editor initialization failed: {}", error);
            return Err(format!("Failed to parse APK binaries: {}", error));
        }
    };

    let apk_version_info = apk_editor.get_version_info();

    let target_package = format!("jp.co.ponos.battlecats{}", suffix.trim());
    let root_elem = apk_editor.manifest.root.get_element(&["manifest"], &apk_editor.manifest.string_pool);
    let pkg_attr = root_elem.and_then(|root| root.get_attribute("package", &apk_editor.manifest.string_pool));

    let current_pkg = if let Some(attr) = pkg_attr {
        if let ResValueType::String(ref string_value) = attr.typed_value.data {
            string_value.resolve(&apk_editor.manifest.string_pool).unwrap_or_default().to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let is_update_patch = current_pkg == target_package;

    let final_id = if is_update_patch {
        log_callback("Package identity matches target APK.".to_string());
        log_callback("Updating target APK...".to_string());
        target_package
    } else {
        log_callback("New package identity found.".to_string());
        log_callback("Creating new APK...".to_string());

        match apk_editor.apply_patches(&suffix, &app_title) {
            Ok(new_package_id) => {
                if let Err(error) = apk_editor.save_to_paths(&manifest_path, if extracted_arsc { Some(arsc_path.as_path()) } else { None }) {
                    error!("Failed saving patched binaries: {}", error);
                    return Err(format!("Failed to save binaries: {}", error));
                }
                new_package_id
            },
            Err(error) => {
                error!("ApkEditor failed to apply patches: {}", error);
                return Err(format!("Patch Error: {}", error));
            }
        }
    };

    let region_key = match target_region {
        Region::En => &user_keys.en,
        Region::Ja => &user_keys.ja,
        Region::Ko => &user_keys.ko,
        Region::Tw => &user_keys.tw,
    };

    log_callback("Packing modded game data...".to_string());
    if let Err(error) = pack::stream_pack_and_list(&mod_dir.join("patch"), &assets_dir, "DownloadLocal", region_key, &log_callback) {
        error!("Data packing failed: {}", error);
        return Err(error);
    }

    log_callback("Rebuilding APK with patch...".to_string());
    let unsigned_apk_path = app_dir.join("unsigned_final.apk");

    match modify::inject_and_build_apk(
        &working_apk,
        &unsigned_apk_path,
        &assets_dir,
        &icons_dir,
        &mod_dir.join("loose"),
        if is_update_patch { None } else { Some(manifest_path.as_path()) },
        if is_update_patch || !extracted_arsc { None } else { Some(arsc_path.as_path()) }
    ) {
        Ok(count) => {
            debug!("Injection successful.");
            log_callback(format!("Injected {} files.", count));
        },
        Err(error) => {
            error!("Injection build failed: {}", error);
            return Err(format!("Build Error: {}", error));
        }
    }

    log_callback("Normalizing binaries...".to_string());
    let normalized_apk_path = app_dir.join("normalized_final.apk");
    if let Err(error) = modify::normalize_apk(&unsigned_apk_path, &normalized_apk_path, &working_apk) {
        error!("Normalization failed: {}", error);
        return Err(format!("Normalization Error: {}", error));
    }

    log_callback("Signing APK...".to_string());
    if let Err(error) = sign::sign(&normalized_apk_path, None) {
        error!("APK Signing failed: {}", error);
        return Err(format!("Native Signing Error: {}", error));
    }

    let output_name = if app_title.trim().is_empty() { final_id } else { app_title.trim().to_string() };

    let get_incremental_path = |dir: &PathBuf, base_name: &str| -> PathBuf {
        let mut counter = 0;
        loop {
            let name = if counter == 0 {
                format!("{}.apk", base_name)
            } else {
                format!("{}{}.apk", base_name, counter)
            };
            let candidate = dir.join(name);
            if !candidate.exists() {
                return candidate;
            }
            counter += 1;
        }
    };

    let final_apk_path = match export_behavior {
        ExportBehavior::Update => {
            input_apk_path.clone()
        },
        ExportBehavior::Create => {
            get_incremental_path(&export_dir, &output_name)
        },
        ExportBehavior::Automatic => {
            if is_update_patch {
                input_apk_path.clone()
            } else {
                get_incremental_path(&export_dir, &output_name)
            }
        }
    };

    debug!("Moving final APK to {:?}", final_apk_path);
    if let Err(error) = fs::copy(&normalized_apk_path, &final_apk_path) {
        error!("Failed copying final APK to destination: {}", error);
        return Err(format!("Filesystem Error: {}", error));
    }

    let _ = fs::remove_dir_all(&app_dir);

    let final_filename = final_apk_path.file_name().unwrap_or_default().to_string_lossy();
    let success_message = match export_behavior {
        ExportBehavior::Update => format!("Successfully Updated {}!", final_filename),
        ExportBehavior::Create => format!("Successfully Built {}!", final_filename),
        ExportBehavior::Automatic => {
            if is_update_patch {
                format!("Successfully Updated {}!", final_filename)
            } else {
                format!("Successfully Built {}!", final_filename)
            }
        }
    };

    info!("Export completed successfully: {}", success_message);
    emit(JobEvent::Log(success_message));

    if let Some((version_code, version_name)) = apk_version_info
        && version_code <= 1401010 {
        log_callback(String::new());
        log_callback(format!("Legacy game version {} detected", version_name));
        log_callback("Legacy versions are known to crash on load".to_string());
        log_callback("Please update to a more stable game version".to_string());
    }

    Ok(())
}
