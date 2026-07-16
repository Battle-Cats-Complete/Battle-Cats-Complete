use std::collections::HashSet;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use tracing::{debug, error, info, instrument, trace, warn};
use zip::{ZipArchive, ZipWriter};

use resand::{
    res_value::{ResValue, ResValueType},
    string_pool::StringPoolHandler,
    table::{ResTable, ResTableEntryValue},
    xmltree::{XMLTree, XMLTreeNode},
};

#[derive(Debug, thiserror::Error)]
pub enum ResError {
    #[error("File operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Manifest parse error: {0}")]
    Manifest(String),
    #[error("Missing required element: {0}")]
    MissingElement(&'static str),
}

pub struct ApkEditor {
    pub manifest: XMLTree,
    pub res_table: Option<ResTable>,
}

impl ApkEditor {
    #[instrument(skip_all, fields(manifest = %manifest_path.display()))]
    pub fn from_paths(manifest_path: &Path, table_path: Option<&Path>) -> Result<Self, ResError> {
        debug!("Parsing Manifest from paths");

        let mut manifest_file = fs::File::open(manifest_path)?;
        let manifest = XMLTree::read(&mut manifest_file).map_err(|error| {
            error!("Failed to parse Manifest: {}", error);
            ResError::Manifest(error.to_string())
        })?;

        let res_table = match table_path {
            Some(path) if path.exists() => {
                debug!("Parsing resources.arsc from {:?}", path);
                let mut table_file = fs::File::open(path)?;
                Some(ResTable::read_all(&mut table_file).map_err(|error| {
                    error!("Failed to parse resources.arsc: {}", error);
                    ResError::Manifest(error.to_string())
                })?)
            }
            Some(path) => {
                warn!("resources.arsc path provided but file does not exist: {:?}", path);
                None
            }
            None => None,
        };

        Ok(Self { manifest, res_table })
    }

    #[instrument(skip_all)]
    pub fn get_version_info(&self) -> Option<(u32, String)> {
        trace!("Extracting version information from XML tree");
        let root_element = self.manifest.root.get_element(&["manifest"], &self.manifest.string_pool)?;

        let version_code_attribute = root_element.get_attribute("versionCode", &self.manifest.string_pool)?;
        let version_name_attribute = root_element.get_attribute("versionName", &self.manifest.string_pool)?;

        let extracted_version_code = match &version_code_attribute.typed_value.data {
            ResValueType::IntDec(decimal_value) => *decimal_value,
            ResValueType::IntHex(hex_value) => *hex_value,
            ResValueType::String(string_reference) => {
                string_reference.resolve(&self.manifest.string_pool)?.parse::<u32>().ok()?
            }
            fallback_data => {
                let data_string = format!("{:?}", fallback_data);
                data_string.chars().filter(|character| character.is_ascii_digit()).collect::<String>().parse::<u32>().ok()?
            }
        };

        let extracted_version_name = match &version_name_attribute.typed_value.data {
            ResValueType::String(string_reference) => {
                string_reference.resolve(&self.manifest.string_pool)?.to_string()
            }
            _ => return None,
        };

        Some((extracted_version_code, extracted_version_name))
    }

    #[instrument(skip_all)]
    pub fn save_to_paths(self, manifest_path: &Path, table_path: Option<&Path>) -> Result<(), ResError> {
        debug!("Saving patched Manifest to {:?}", manifest_path);
        let mut manifest_out = fs::File::create(manifest_path)?;

        self.manifest.write(&mut manifest_out).map_err(|error| {
            error!("Failed to write Manifest: {}", error);
            ResError::Manifest(error.to_string())
        })?;

        let Some(path) = table_path else { return Ok(()) };
        let Some(table) = self.res_table else { return Ok(()) };

        debug!("Saving patched resources.arsc to {:?}", path);
        let mut table_out = fs::File::create(path)?;
        table.write_all(&mut table_out).map_err(|error| {
            error!("Failed to write resources.arsc: {}", error);
            ResError::Manifest(error.to_string())
        })?;

        Ok(())
    }

    #[instrument(skip_all, fields(suffix = %suffix, title = %app_title))]
    pub fn apply_patches(&mut self, suffix: &str, app_title: &str) -> Result<String, ResError> {
        info!("Applying Manifest patches");

        let root = self.manifest.root.get_element_mut(&["manifest"], &self.manifest.string_pool)
            .ok_or_else(|| {
                error!("Could not find root <manifest> element.");
                ResError::MissingElement("manifest root")
            })?;

        let initial_children = root.children.len();
        root.children.retain(|child| {
            child.element.name.resolve(&self.manifest.string_pool).unwrap_or_default() != "split"
        });

        if root.children.len() < initial_children {
            trace!("Removed ghost <split> tags from root");
        }

        root.element.attributes.retain(|attribute| {
            let Some(name) = attribute.name.resolve(&self.manifest.string_pool) else { return true; };
            name != "split" && name != "isFeatureSplit"
        });

        trace!("Injecting isFeatureSplit=false into manifest root");
        root.insert_attribute(
            "isFeatureSplit".into(),
            ResValue::new_bool(false),
            &mut self.manifest.string_pool,
            self.manifest.resource_map.as_mut(),
            Some(0x0101055b.into()),
        );

        let package_attribute = root.get_attribute_mut("package", &self.manifest.string_pool)
            .ok_or_else(|| {
                error!("Missing 'package' attribute on root manifest.");
                ResError::MissingElement("package attribute")
            })?;

        let original_package = match &package_attribute.typed_value.data {
            ResValueType::String(string_value) => string_value
                .resolve(&self.manifest.string_pool)
                .unwrap_or_default()
                .to_string(),
            _ => {
                error!("Invalid package string format found in manifest.");
                return Err(ResError::MissingElement("Invalid package string format"));
            }
        };

        let mut parts: Vec<&str> = original_package.split('.').collect();
        if !parts.is_empty() {
            parts.pop();
        }

        let new_package_name = format!("{}.battlecats{}", parts.join("."), suffix.trim());
        debug!("Changing package name from {} to {}", original_package, new_package_name);

        package_attribute.write_string(new_package_name.as_str().into(), &mut self.manifest.string_pool);

        trace!("Initiating deep recursive package reference scrubbing...");
        replace_package_refs(&mut self.manifest.root, &mut self.manifest.string_pool, self.res_table.as_ref(), &original_package, &new_package_name);

        self.apply_application_patches(app_title);

        let Some(table) = self.res_table.as_mut() else {
            info!("Patching complete. New identity: {}", new_package_name);
            return Ok(new_package_name);
        };

        let Some(package) = table.packages.first_mut() else {
            info!("Patching complete. New identity: {}", new_package_name);
            return Ok(new_package_name);
        };

        debug!("Updating resources.arsc package name to {}", new_package_name);
        package.name = new_package_name.clone();

        info!("Patching complete. New identity: {}", new_package_name);
        Ok(new_package_name)
    }

    fn apply_application_patches(&mut self, app_title: &str) {
        let Some(app_elem) = self.manifest.root.get_element_mut(&["manifest", "application"], &self.manifest.string_pool) else {
            warn!("Could not find <application> element in Manifest!");
            return;
        };

        app_elem.element.attributes.retain(|attribute| {
            let Some(name) = attribute.name.resolve(&self.manifest.string_pool) else { return true; };
            name != "extractNativeLibs" && name != "isSplitRequired"
        });

        let pre_vending_count = app_elem.children.len();
        app_elem.children.retain(|child| {
            let is_metadata = child.element.name.resolve(&self.manifest.string_pool) == Some("meta-data");
            if !is_metadata { return true; }

            let Some(name_attribute) = child.get_attribute("name", &self.manifest.string_pool) else { return true; };
            let ResValueType::String(ref string_value) = name_attribute.typed_value.data else { return true; };
            let Some(resolved_val) = string_value.resolve(&self.manifest.string_pool) else { return true; };

            !(resolved_val.contains("vending.splits") || resolved_val.contains("vending.derived.apk.id"))
        });

        if app_elem.children.len() < pre_vending_count {
            trace!("Stripped vending split metadata tags");
        }

        app_elem.insert_attribute(
            "extractNativeLibs".into(),
            ResValue::new_bool(true),
            &mut self.manifest.string_pool,
            self.manifest.resource_map.as_mut(),
            Some(0x010104ea.into()),
        );

        app_elem.insert_attribute(
            "isSplitRequired".into(),
            ResValue::new_bool(false),
            &mut self.manifest.string_pool,
            self.manifest.resource_map.as_mut(),
            Some(0x01010591.into()),
        );

        if app_title.trim().is_empty() { return; }

        if let Some(label_attribute) = app_elem.get_attribute_mut("label", &self.manifest.string_pool) {
            debug!("Overwriting app label with '{}'", app_title.trim());
            label_attribute.write_string(app_title.trim().into(), &mut self.manifest.string_pool);
        } else {
            debug!("Inserting new app label '{}'", app_title.trim());
            app_elem.insert_attribute(
                "label".into(),
                ResValue::new_str(app_title.trim().into(), &mut self.manifest.string_pool),
                &mut self.manifest.string_pool,
                self.manifest.resource_map.as_mut(),
                Some(0x01010001.into()),
            );
        }

        trace!("Scrubbing original labels from all activities to force application label inheritance...");
        strip_activity_labels(app_elem, &mut self.manifest.string_pool);
    }
}

fn strip_activity_labels(node: &mut XMLTreeNode, pool: &mut StringPoolHandler) {
    let is_activity = node.element.name.resolve(pool).is_some_and(|name| name == "activity" || name == "activity-alias");

    if is_activity {
        node.element.attributes.retain(|attribute| {
            attribute.name.resolve(pool).is_none_or(|attr_name| attr_name != "label")
        });
    }

    for child in &mut node.children {
        strip_activity_labels(child, pool);
    }
}

fn replace_package_refs(
    elem: &mut XMLTreeNode,
    pool: &mut StringPoolHandler,
    res_table: Option<&ResTable>,
    old_pkg: &str,
    new_pkg: &str,
) {
    const ATTRS_TO_CHECK: &[&str] = &["name", "authorities", "taskAffinity", "sharedUserId", "value", "scheme", "host"];

    let tag_name = elem.element.name.resolve(pool).unwrap_or_default().to_string();
    let is_component = matches!(tag_name.as_str(), "application" | "activity" | "activity-alias" | "service" | "receiver" | "provider");

    for attr_name in ATTRS_TO_CHECK {
        let Some(attribute) = elem.get_attribute_mut(attr_name, pool) else { continue; };

        let resolved_string_value = match &attribute.typed_value.data {
            ResValueType::String(string_value) => string_value.resolve(pool).map(|s| s.to_string()),
            ResValueType::Reference(table_reference) => {
                (|| -> Option<String> {
                    let table = res_table?;
                    let package = table.packages.first()?;
                    let resource_value = package.resolve_ref(*table_reference)?;
                    let ResTableEntryValue::ResValue(ref val) = resource_value.data else { return None; };
                    let ResValueType::String(ref string_reference) = val.data.data else { return None; };
                    let resolved_string = string_reference.resolve(&table.string_pool)?;
                    Some(resolved_string.to_string())
                })()
            },
            _ => None,
        };

        let Some(resolved_string) = resolved_string_value else { continue; };

        if *attr_name == "name" && is_component {
            if resolved_string.starts_with('.') {
                let new_val = format!("{}{}", old_pkg, resolved_string);
                attribute.write_string(new_val.into(), pool);
            } else if !resolved_string.contains('.') {
                let new_val = format!("{}.{}", old_pkg, resolved_string);
                attribute.write_string(new_val.into(), pool);
            }
            continue;
        }

        if !resolved_string.contains(old_pkg) { continue; }

        let new_val = resolved_string.replace(old_pkg, new_pkg);
        attribute.write_string(new_val.into(), pool);
    }

    for child in &mut elem.children {
        replace_package_refs(child, pool, res_table, old_pkg, new_pkg);
    }
}

fn collect_directory_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() { return Ok(Vec::new()); }

    let mut files = Vec::new();
    let entries = fs::read_dir(dir).map_err(|error| error.to_string())?;

    for entry in entries.flatten() {
        if entry.path().is_file() {
            files.push(entry.path());
        }
    }

    Ok(files)
}

#[instrument(skip_all)]
pub fn inject_and_build_apk(
    source_apk: &Path,
    output_apk: &Path,
    assets_dir: &Path,
    icons_dir: &Path,
    loose_dir: &Path,
    patched_manifest: Option<&Path>,
    patched_arsc: Option<&Path>,
) -> Result<usize, String> {
    info!("Starting APK build & injection from {:?} to {:?}", source_apk, output_apk);

    let source_file = fs::File::open(source_apk).map_err(|error| {
        error!("Failed to open source APK: {}", error);
        error.to_string()
    })?;

    let mut archive = ZipArchive::new(source_file).map_err(|error| {
        error!("Failed to read source APK archive: {}", error);
        error.to_string()
    })?;

    let destination_file = fs::File::create(output_apk).map_err(|error| {
        error!("Failed to create output APK: {}", error);
        error.to_string()
    })?;

    let mut zip_writer = ZipWriter::new(destination_file);
    let mut injected_count = 0;
    let mut files_to_inject = HashSet::new();

    if patched_manifest.is_some() { files_to_inject.insert("AndroidManifest.xml".to_string()); }
    if patched_arsc.is_some() { files_to_inject.insert("resources.arsc".to_string()); }

    let asset_files = collect_directory_files(assets_dir)?;
    for path in &asset_files {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        files_to_inject.insert(format!("assets/{}", name));
    }

    let loose_files = collect_directory_files(loose_dir)?;
    for path in &loose_files {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        files_to_inject.insert(format!("assets/{}", name));
    }

    debug!("Identified {} files to inject or replace.", files_to_inject.len());

    let has_custom_icon = icons_dir.join("icon.png").exists();
    let has_custom_foreground = icons_dir.join("icon_foreground.png").exists();
    let has_custom_push = icons_dir.join("push_icon.png").exists();
    let fallback_foreground = has_custom_icon && !has_custom_foreground;

    let mut existing_res_folders = HashSet::new();

    for index in 0..archive.len() {
        let archive_file = archive.by_index(index).map_err(|error| error.to_string())?;
        let file_name = archive_file.name().to_string();
        let upper_name = file_name.to_ascii_uppercase();

        if upper_name.starts_with("META-INF/") || upper_name.starts_with("META-INF\\") {
            if upper_name.ends_with(".SF")
                || upper_name.ends_with(".RSA")
                || upper_name.ends_with(".DSA")
                || upper_name.ends_with(".EC")
                || upper_name.ends_with("MANIFEST.MF")
                || upper_name.contains("STAMP-CERT")
            {
                trace!("Skipping original signature file: {}", file_name);
                continue;
            }
        }

        if file_name.starts_with("res/") {
            if let Some(parent) = Path::new(&file_name).parent() {
                existing_res_folders.insert(parent.to_string_lossy().replace('\\', "/"));
            }
        }

        if files_to_inject.contains(&file_name) { continue; }

        let short_name = Path::new(&file_name).file_name().unwrap_or_default().to_string_lossy();

        if file_name.starts_with("res/") {
            if short_name == "icon.png" && has_custom_icon {
                trace!("Intercepted original icon.png");
                continue;
            }
            if short_name == "icon_foreground.png" && (has_custom_foreground || fallback_foreground) {
                trace!("Intercepted and dropped original icon_foreground.png");
                continue;
            }
            if short_name == "push_icon.png" && has_custom_push {
                trace!("Intercepted original push_icon.png");
                continue;
            }
        }

        zip_writer.raw_copy_file(archive_file).map_err(|error| {
            error!("Failed to copy file {:?} to new archive: {}", file_name, error);
            error.to_string()
        })?;
    }

    let mut inject_file = |local_path: &Path, zip_path: &str, store: bool| -> Result<(), String> {
        if !local_path.exists() { return Ok(()); }
        trace!("Injecting file: {} (Store mode: {})", zip_path, store);

        let file_data = fs::read(local_path).map_err(|error| error.to_string())?;
        let compression = if store { zip::CompressionMethod::Stored } else { zip::CompressionMethod::Deflated };
        let options = zip::write::SimpleFileOptions::default().compression_method(compression);

        zip_writer.start_file(zip_path, options).map_err(|error| error.to_string())?;
        zip_writer.write_all(&file_data).map_err(|error| error.to_string())?;
        injected_count += 1;
        Ok(())
    };

    debug!("Beginning to inject files...");

    if let Some(manifest) = patched_manifest { inject_file(manifest, "AndroidManifest.xml", false)?; }
    if let Some(arsc) = patched_arsc { inject_file(arsc, "resources.arsc", true)?; }

    for path in asset_files {
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let store = name.ends_with(".pack") || name.ends_with(".list");
        inject_file(&path, &format!("assets/{}", name), store)?;
    }

    for path in loose_files {
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        inject_file(&path, &format!("assets/{}", name), true)?;
    }

    if icons_dir.exists() {
        injected_count += inject_scaled_icons(
            &mut zip_writer,
            icons_dir,
            &existing_res_folders,
            has_custom_icon,
            has_custom_foreground,
            has_custom_push,
            fallback_foreground
        )?;
    }

    zip_writer.finish().map_err(|error| {
        error!("Failed to finalize APK ZipWriter: {}", error);
        error.to_string()
    })?;

    info!("Successfully built APK. Total injected files: {}", injected_count);
    Ok(injected_count)
}

fn inject_scaled_icons<W: Write + std::io::Seek>(
    zip_writer: &mut ZipWriter<W>,
    icons_dir: &Path,
    existing_res_folders: &HashSet<String>,
    has_custom_icon: bool,
    has_custom_foreground: bool,
    has_custom_push: bool,
    fallback_foreground: bool,
) -> Result<usize, String> {
    info!("Scaling and injecting custom icons...");
    let mut injected = 0;
    let foreground_source = if fallback_foreground { "icon.png" } else { "icon_foreground.png" };

    let icon_blueprints = [
        ("icon.png", "icon.png", 192, 144, 96, has_custom_icon, false),
        ("icon_foreground.png", foreground_source, 432, 324, 216, has_custom_foreground || fallback_foreground, fallback_foreground),
        ("push_icon.png", "push_icon.png", 96, 72, 48, has_custom_push, false),
    ];

    let target_resolutions = [
        ("drawable-xxxhdpi", 0), ("drawable-xxhdpi", 1), ("drawable-xhdpi", 2),
        ("drawable-xxxhdpi-v4", 0), ("drawable-xxhdpi-v4", 1), ("drawable-xhdpi-v4", 2),
        ("mipmap-xxxhdpi", 0), ("mipmap-xxhdpi", 1), ("mipmap-xhdpi", 2),
        ("mipmap-xxxhdpi-v4", 0), ("mipmap-xxhdpi-v4", 1), ("mipmap-xhdpi-v4", 2),
    ];

    for (dest_name, source_name, xxxhdpi, xxhdpi, xhdpi, exists, is_fallback) in icon_blueprints {
        if !exists { continue; }

        let source_path = icons_dir.join(source_name);
        let Ok(source_image) = image::open(&source_path) else {
            warn!("Failed to open or decode custom icon: {}", source_name);
            continue;
        };

        for (folder, resolution_idx) in target_resolutions {
            let res_folder = format!("res/{}", folder);
            if !existing_res_folders.contains(&res_folder) { continue; }

            let canvas_size = match resolution_idx {
                0 => xxxhdpi,
                1 => xxhdpi,
                _ => xhdpi,
            };

            let zip_path = format!("{}/{}", res_folder, dest_name);
            let inner_scale_size = if is_fallback { (canvas_size as f32 * 0.67) as u32 } else { canvas_size };

            let scaled_image = source_image.resize_exact(inner_scale_size, inner_scale_size, image::imageops::FilterType::Lanczos3);

            let final_image = if is_fallback {
                let mut canvas = image::RgbaImage::new(canvas_size, canvas_size);
                let offset = ((canvas_size.saturating_sub(inner_scale_size)) / 2) as i64;
                image::imageops::overlay(&mut canvas, &scaled_image.to_rgba8(), offset, offset);
                image::DynamicImage::ImageRgba8(canvas)
            } else {
                scaled_image
            };

            let mut cursor = Cursor::new(Vec::new());
            if final_image.write_to(&mut cursor, image::ImageFormat::Png).is_err() { continue; };

            let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            if zip_writer.start_file(&zip_path, options).is_err() { continue; };

            let _ = zip_writer.write_all(&cursor.into_inner());
            trace!("Injected scaled icon at: {} (Canvas: {}, Image: {})", zip_path, canvas_size, inner_scale_size);
            injected += 1;
        }
    }

    Ok(injected)
}

#[instrument(skip_all)]
pub fn normalize_apk(input_apk: &Path, output_apk: &Path, original_apk: &Path) -> Result<(), String> {
    info!("Normalizing APK binaries for signature verification...");
    let mut stored_files_map = HashSet::new();

    let original_file = fs::File::open(original_apk).map_err(|error| {
        error!("Failed to open original APK for normalization: {}", error);
        format!("Failed to open original APK: {}", error)
    })?;

    let mut original_archive = ZipArchive::new(original_file).map_err(|error| {
        error!("Failed to read original APK for normalization: {}", error);
        format!("Failed to read original APK: {}", error)
    })?;

    for index in 0..original_archive.len() {
        let archive_file = original_archive.by_index(index).map_err(|error| error.to_string())?;
        if archive_file.compression() == zip::CompressionMethod::Stored {
            stored_files_map.insert(archive_file.name().to_string());
        }
    }

    debug!("Identified {} stored files from original APK.", stored_files_map.len());

    let source_file = fs::File::open(input_apk).map_err(|error| format!("Failed to open APK: {}", error))?;
    let mut archive = ZipArchive::new(source_file).map_err(|error| format!("Failed to read APK archive: {}", error))?;

    let destination_file = fs::File::create(output_apk).map_err(|error| format!("Failed to create normalized APK: {}", error))?;
    let mut zip_writer = ZipWriter::new(destination_file);

    const UNCOMPRESSED_EXTENSIONS: &[&str] = &["dex", "arsc", "so", "pack", "list", "ogg"];

    for index in 0..archive.len() {
        let mut archive_file = archive.by_index(index).map_err(|error| error.to_string())?;
        let file_name = archive_file.name().to_string();
        let file_extension = Path::new(&file_name).extension().and_then(|ext| ext.to_str()).unwrap_or("");

        let force_store = UNCOMPRESSED_EXTENSIONS.contains(&file_extension);
        let is_already_stored = stored_files_map.contains(&file_name);

        if !force_store && !is_already_stored {
            zip_writer.raw_copy_file(archive_file).map_err(|error| error.to_string())?;
            continue;
        }

        let mut file_data = Vec::new();
        archive_file.read_to_end(&mut file_data).map_err(|error| {
            error!("Failed reading file for normalization: {}", file_name);
            format!("Failed reading {}: {}", file_name, error)
        })?;

        let byte_alignment = if file_extension == "so" { 4096 } else { 4 };
        trace!("Re-aligning {} to {} bytes (Stored).", file_name, byte_alignment);

        let write_options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .with_alignment(byte_alignment);

        zip_writer.start_file(&file_name, write_options).map_err(|error| error.to_string())?;
        zip_writer.write_all(&file_data).map_err(|error| error.to_string())?;
    }

    zip_writer.finish().map_err(|error| {
        error!("Failed to finish normalized APK ZipWriter: {}", error);
        error.to_string()
    })?;

    info!("APK normalization complete.");
    Ok(())
}