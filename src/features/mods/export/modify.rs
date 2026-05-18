use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use regex::Regex;
use zip::{ZipArchive, ZipWriter};

pub fn patch_identity(decode_dir: &Path, new_suffix: &str, app_title: &str, _log_cb: &impl Fn(String)) -> Result<String, String> {
    let suffix = new_suffix.trim();
    if suffix.is_empty() {
        return Ok("jp.co.ponos.battlecats".to_string());
    }

    let manifest_path = decode_dir.join("AndroidManifest.xml");
    let strings_path = decode_dir.join("res").join("values").join("strings.xml");

    if !manifest_path.exists() {
        return Err("Decoded AndroidManifest.xml not found. Apktool decode may have failed.".into());
    }

    let mut manifest_text = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;

    let targets = [
        "battlecatsen",
        "battlecatsko",
        "battlecatstw",
        "battlecats",
    ];

    let mut active_token = "";
    for target in targets.iter() {
        if manifest_text.contains(&format!("package=\"jp.co.ponos.{}\"", target)) {
            active_token = target;
            break;
        }
    }

    if active_token.is_empty() {
        for target in targets.iter() {
            if manifest_text.contains(target) {
                active_token = target;
                break;
            }
        }
    }

    if active_token.is_empty() {
        return Err("Could not find a recognized Battle Cats package token to patch.".into());
    }

    let new_token = format!("battlecats{}", suffix);
    let final_package_id = format!("jp.co.ponos.{}", new_token);

    manifest_text = manifest_text.replace(active_token, &new_token);

    manifest_text = manifest_text.replace("android:isSplitRequired=\"true\"", "android:isSplitRequired=\"false\"");
    manifest_text = manifest_text.replace("android:extractNativeLibs=\"false\"", "android:extractNativeLibs=\"true\"");

    let split_re = Regex::new(r#"split="[^"]*""#).expect("Failed to compile split regex");
    let is_feature_split_re = Regex::new(r#"android:isFeatureSplit="true""#).expect("Failed to compile feature split regex");
    manifest_text = split_re.replace_all(&manifest_text, "").to_string();
    manifest_text = is_feature_split_re.replace_all(&manifest_text, "").to_string();

    if let Some(start) = manifest_text.find("<split") {
        if let Some(end) = manifest_text[start..].find("/>") {
            manifest_text.replace_range(start..start+end+2, "");
        }
    }

    fs::write(&manifest_path, manifest_text).map_err(|e| e.to_string())?;

    if strings_path.exists() {
        let mut strings_text = fs::read_to_string(&strings_path).map_err(|e| e.to_string())?;
        strings_text = strings_text.replace(active_token, &new_token);

        if !app_title.trim().is_empty() {
            let app_name_re = Regex::new(r#"<string name="app_name">[^<]*</string>"#).unwrap();
            let new_title_element = format!("<string name=\"app_name\">{}</string>", app_title.trim());
            strings_text = app_name_re.replace_all(&strings_text, new_title_element.as_str()).to_string();
        }

        fs::write(&strings_path, strings_text).map_err(|e| e.to_string())?;
    }

    Ok(final_package_id)
}

pub fn replace_icons(mod_dir: &Path, decode_dir: &Path, _log_cb: &impl Fn(String)) -> Result<(), String> {
    let icons_dir = mod_dir.join("icons");
    let targets = [
        (icons_dir.join("icon.png"), "icon.png"),
        (icons_dir.join("icon_foreground.png"), "icon_foreground.png"),
        (icons_dir.join("push_icon.png"), "push_icon.png"),
    ];

    if targets.iter().all(|(p, _)| !p.exists()) { return Ok(()); }

    let res_dir = decode_dir.join("res");
    if !res_dir.exists() { return Ok(()); }

    let target_dirs = [
        "drawable-xhdpi",
        "drawable-xxhdpi",
        "drawable-xxxhdpi"
    ];

    for dir_name in target_dirs {
        let target_dir = res_dir.join(dir_name);
        if !target_dir.exists() { continue; }

        for (src_path, target_name) in &targets {
            if src_path.exists() {
                let dest_path = target_dir.join(target_name);
                if dest_path.exists() {
                    let _ = fs::copy(src_path, &dest_path);
                }
            }
        }
    }

    Ok(())
}

pub fn inject_loose_assets(mod_dir: &Path, decode_dir: &Path) -> Result<usize, String> {
    let loose_dir = mod_dir.join("loose");
    if !loose_dir.exists() { return Ok(0); }

    let assets_dir = decode_dir.join("assets");
    let _ = fs::create_dir_all(&assets_dir);

    let mut copied_count = 0;

    if let Ok(entries) = fs::read_dir(&loose_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().unwrap_or_default();
                let dest_path = assets_dir.join(filename);

                let mut should_copy = true;

                if dest_path.exists() {
                    let src_meta = fs::metadata(&path).ok();
                    let dest_meta = fs::metadata(&dest_path).ok();

                    if let (Some(sm), Some(dm)) = (src_meta, dest_meta) {
                        if sm.len() == dm.len() {
                            if let (Ok(src_data), Ok(dest_data)) = (fs::read(&path), fs::read(&dest_path)) {
                                if src_data == dest_data {
                                    should_copy = false;
                                }
                            }
                        }
                    }
                }

                if should_copy {
                    if fs::copy(&path, &dest_path).is_ok() {
                        copied_count += 1;
                    }
                }
            }
        }
    }

    Ok(copied_count)
}

pub fn normalize_apk(input_apk: &Path, output_apk: &Path) -> Result<(), String> {
    let source_file = fs::File::open(input_apk).map_err(|e| format!("Failed to open APK: {}", e))?;
    let mut archive = ZipArchive::new(source_file).map_err(|e| format!("Failed to read APK archive: {}", e))?;

    let dest_file = fs::File::create(output_apk).map_err(|e| format!("Failed to create normalized APK: {}", e))?;
    let mut zip_writer = ZipWriter::new(dest_file);

    let uncompressed_exts = ["dex", "arsc", "so", "pack", "list", "ogg"];

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_string();
        let ext = Path::new(&name).extension().and_then(|e| e.to_str()).unwrap_or("");

        let needs_store = uncompressed_exts.contains(&ext);

        if needs_store && file.compression() != zip::CompressionMethod::Stored {
            let mut data = Vec::new();
            file.read_to_end(&mut data).map_err(|e| format!("Failed reading {}: {}", name, e))?;

            let alignment = if ext == "so" { 4096 } else { 4 };

            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)
                .with_alignment(alignment);

            zip_writer.start_file(&name, options).map_err(|e| e.to_string())?;
            zip_writer.write_all(&data).map_err(|e| e.to_string())?;
        } else {
            zip_writer.raw_copy_file(file).map_err(|e| e.to_string())?;
        }
    }

    zip_writer.finish().map_err(|e| e.to_string())?;
    Ok(())
}