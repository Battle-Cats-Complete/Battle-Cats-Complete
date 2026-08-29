use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use tracing::{debug, error, info, info_span};
use zip::ZipArchive;

use crate::common::architecture;
use crate::common::job::JobEvent;
use crate::systems::addons::apkeditor::xapk;

use super::modify::{self, Identity, ModIcons};
use super::sign;

const EXPORT_DIR: &str = "exports";
const SPLIT_EXTENSIONS: [&str; 3] = ["xapk", "apkm", "apks"];

pub struct CloneConfig {
    pub input: PathBuf,
    pub app_name: String,
    pub package: String,
    pub icon: Option<PathBuf>,
}

impl CloneConfig {
    pub fn is_actionable(&self) -> bool {
        !self.app_name.trim().is_empty() || !self.package.trim().is_empty() || self.icon.is_some()
    }
}

pub fn run(config: CloneConfig, emit: impl Fn(JobEvent) + Sync) -> Result<PathBuf, String> {
    let _span = info_span!("clone_worker", apk = %config.input.display()).entered();

    let log = |message: String| {
        info!("Clone UI Log: {}", message);
        emit(JobEvent::Log(message));
    };

    let base_dir = std::env::current_dir().map_err(|error| format!("No working directory: {}", error))?;
    let export_dir = base_dir.join(EXPORT_DIR);
    let _work = architecture::Scratch::claim();
    let app_dir = base_dir.join(architecture::WORK).join("clone");
    let bin_dir = app_dir.join("binaries");
    let assets_dir = app_dir.join("assets");

    let _ = fs::remove_dir_all(&app_dir);
    fs::create_dir_all(&export_dir).map_err(|error| format!("Could not create exports: {}", error))?;
    fs::create_dir_all(&bin_dir).map_err(|error| format!("Could not create workspace: {}", error))?;
    fs::create_dir_all(&assets_dir).map_err(|error| format!("Could not create workspace: {}", error))?;

    let mut working_apk = config.input.clone();
    let extension = config.input.extension().and_then(OsStr::to_str).unwrap_or_default();

    if SPLIT_EXTENSIONS.contains(&extension) {
        log("Merging split APKs...".to_string());
        let merged = app_dir.join("merged.apk");

        xapk::merge_xapk(&working_apk, &merged, &log).map_err(|error| error.to_string())?;
        working_apk = merged;
    }

    log("Reading APK identity...".to_string());
    let manifest_path = bin_dir.join("AndroidManifest.xml");
    let arsc_path = bin_dir.join("resources.arsc");
    let extracted_arsc = extract_binaries(&working_apk, &manifest_path, &arsc_path)?;

    let mut editor = modify::ApkEditor::from_paths(&manifest_path, extracted_arsc.then_some(arsc_path.as_path()))
        .map_err(|error| format!("Failed to parse APK binaries: {}", error))?;

    let package = config.package.trim();
    let identity = if package.is_empty() { Identity::Keep } else { Identity::Suffix(package) };

    let renamed = editor
        .apply_patches(identity, &config.app_name)
        .map_err(|error| format!("Patch Error: {}", error))?;

    log(format!("Cloned identity: {}", renamed));

    editor
        .save_to_paths(&manifest_path, extracted_arsc.then_some(arsc_path.as_path()))
        .map_err(|error| format!("Failed to save binaries: {}", error))?;

    let icons = ModIcons { icon: config.icon.clone(), ..ModIcons::default() };

    if icons.icon.is_some() {
        log("Replacing launcher icon...".to_string());
    }

    log("Rebuilding APK...".to_string());
    let unsigned = app_dir.join("unsigned.apk");

    let injected = modify::inject_and_build_apk(
        &working_apk,
        &unsigned,
        &assets_dir,
        &icons,
        &[],
        Some(manifest_path.as_path()),
        extracted_arsc.then_some(arsc_path.as_path()),
    )
    .map_err(|error| format!("Build Error: {}", error))?;

    debug!("Injected {} files during clone", injected);

    log("Normalizing binaries...".to_string());
    let normalized = app_dir.join("normalized.apk");
    modify::normalize_apk(&unsigned, &normalized, &working_apk)
        .map_err(|error| format!("Normalization Error: {}", error))?;

    log("Signing APK...".to_string());
    sign::sign(&normalized, None).map_err(|error| format!("Signing Error: {}", error))?;

    let stem = if config.app_name.trim().is_empty() { renamed } else { config.app_name.trim().to_string() };
    let destination = vacant_path(&export_dir, &stem);

    fs::copy(&normalized, &destination).map_err(|error| format!("Filesystem Error: {}", error))?;

    let name = destination.file_name().unwrap_or_default().to_string_lossy().to_string();
    log(format!("Successfully cloned {}!", name));

    Ok(destination)
}

fn extract_binaries(apk: &Path, manifest: &Path, arsc: &Path) -> Result<bool, String> {
    let file = fs::File::open(apk).map_err(|error| format!("Failed to open APK: {}", error))?;
    let mut archive = ZipArchive::new(file).map_err(|error| format!("Failed to read APK archive: {}", error))?;

    let mut found_arsc = false;
    let mut found_manifest = false;

    for index in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(index) else { continue };
        let name = entry.name().to_string();

        if name == "AndroidManifest.xml" {
            let mut output = fs::File::create(manifest).map_err(|error| error.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
            found_manifest = true;
        } else if name == "resources.arsc" {
            let mut output = fs::File::create(arsc).map_err(|error| error.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
            found_arsc = true;
        }
    }

    if !found_manifest {
        error!("APK carries no AndroidManifest.xml");
        return Err("That file carries no AndroidManifest.xml".to_string());
    }

    Ok(found_arsc)
}

fn vacant_path(dir: &Path, stem: &str) -> PathBuf {
    let mut counter = 0;

    loop {
        let name = if counter == 0 { format!("{}.apk", stem) } else { format!("{}{}.apk", stem, counter) };
        let candidate = dir.join(name);

        if !candidate.exists() {
            return candidate;
        }

        counter += 1;
    }
}
