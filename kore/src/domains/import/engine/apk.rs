use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use zip::ZipArchive;

use crate::domains::mining::Build;
use crate::systems::apk::modify::ApkEditor;

const MANIFEST: &str = "AndroidManifest.xml";

const BUNDLE_MANIFEST: &str = "manifest.json";

const JUNK_NAMES: [&str; 2] = ["save_data.old", "stamp-cert-sha256"];

const JUNK_EXTENSIONS: [&str; 3] = ["proto", "properties", "txt"];

const SESSION: &str = "session";

const SESSION_SUFFIXES: [&str; 2] = [".data", ".version"];

pub(crate) fn find_files(
    search_directory: &Path,
    list_paths: &mut Vec<PathBuf>,
    apk_paths: &mut Vec<PathBuf>,
    loose_paths: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    if !search_directory.is_dir() {
        return Ok(());
    }

    let directory_entries = fs::read_dir(search_directory)?;

    for entry_result in directory_entries.flatten() {
        let item_path = entry_result.path();

        if item_path.is_dir() {
            find_files(&item_path, list_paths, apk_paths, loose_paths)?;
            continue;
        }

        if junk(&item_path) {
            continue;
        }

        let Some(file_extension) = item_path.extension() else {
            continue;
        };

        let extension_string = file_extension.to_string_lossy().to_lowercase();

        match extension_string.as_str() {
            "list" => {
                list_paths.push(item_path);
            }
            "apk" | "xapk" => {
                apk_paths.push(item_path);
            }
            "pack" | "json" | "dat" | "lock" => {
                continue;
            }
            _ => {
                loose_paths.push(item_path);
            }
        }
    }

    Ok(())
}

fn junk(path: &Path) -> bool {
    let Some(name) = path.file_name().map(|held| held.to_string_lossy().to_lowercase()) else {
        return false;
    };

    if JUNK_NAMES.contains(&name.as_str()) {
        return true;
    }

    let extension = path.extension().map(|held| held.to_string_lossy().to_lowercase());

    if extension.is_some_and(|held| JUNK_EXTENSIONS.contains(&held.as_str())) {
        return true;
    }

    name.starts_with(SESSION) && SESSION_SUFFIXES.iter().any(|end| name.ends_with(end))
}

pub(crate) fn builds(apk_paths: &[PathBuf]) -> Vec<Build> {
    apk_paths.par_iter().filter_map(|path| read_build(path)).collect()
}

fn read_build(apk_path: &Path) -> Option<Build> {
    let archive_file = fs::File::open(apk_path).ok()?;
    let mut archive = ZipArchive::new(archive_file).ok()?;

    if let Some(build) = entry(&mut archive, MANIFEST).and_then(|bytes| binary_build(&bytes, apk_path)) {
        return Some(build);
    }

    entry(&mut archive, BUNDLE_MANIFEST).and_then(|bytes| bundle_build(&bytes, apk_path))
}

fn entry(archive: &mut ZipArchive<fs::File>, name: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    archive.by_name(name).ok()?.read_to_end(&mut bytes).ok()?;

    Some(bytes)
}

fn binary_build(bytes: &[u8], apk_path: &Path) -> Option<Build> {
    let editor = ApkEditor::from_manifest_bytes(bytes).ok()?;
    let (code, name) = editor.get_version_info()?;

    Some(Build { code, name, label: editor.get_package().unwrap_or_else(|| stem(apk_path)) })
}

fn bundle_build(bytes: &[u8], apk_path: &Path) -> Option<Build> {
    let manifest: serde_json::Value = serde_json::from_slice(bytes).ok()?;

    let name = manifest.get("version_name")?.as_str()?.trim().to_string();

    if name.is_empty() {
        return None;
    }

    let code = manifest.get("version_code").and_then(version_code).unwrap_or_default();
    let label = manifest
        .get("package_name")
        .and_then(serde_json::Value::as_str)
        .map_or_else(|| stem(apk_path), str::to_string);

    Some(Build { code, name, label })
}

fn version_code(value: &serde_json::Value) -> Option<u32> {
    value
        .as_u64()
        .and_then(|code| u32::try_from(code).ok())
        .or_else(|| value.as_str()?.trim().parse().ok())
}

fn stem(apk_path: &Path) -> String {
    apk_path.file_stem().unwrap_or_default().to_string_lossy().into_owned()
}

pub(crate) fn extract_all(apk_paths: &[PathBuf]) -> (Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>) {
    if apk_paths.is_empty() {
        return (Vec::new(), Vec::new(), Vec::new());
    }

    let parallel_results: Vec<(Vec<PathBuf>, PathBuf, Vec<PathBuf>)> = apk_paths
        .par_iter()
        .filter_map(|apk_file_path| {
            let parent_directory = apk_file_path.parent().unwrap_or(Path::new(""));
            let apk_stem_name = apk_file_path.file_stem().unwrap_or_default().to_string_lossy();
            let extraction_directory = parent_directory.join(apk_stem_name.to_string());

            if !extraction_directory.exists() {
                let _ = fs::create_dir_all(&extraction_directory);
            }

            let mut extracted_lists = Vec::new();
            let mut extracted_loose = Vec::new();

            let input_zip_file = match fs::File::open(apk_file_path) {
                Ok(file) => file,
                Err(_) => return Some((extracted_lists, extraction_directory, extracted_loose)),
            };

            let mut archive_reader = match ZipArchive::new(input_zip_file) {
                Ok(archive) => archive,
                Err(_) => return Some((extracted_lists, extraction_directory, extracted_loose)),
            };

            for index in 0..archive_reader.len() {
                let Ok(mut current_file) = archive_reader.by_index(index) else {
                    continue;
                };

                if current_file.is_dir() {
                    continue;
                }

                let file_name_string = current_file.name().to_string();
                let path_object = Path::new(&file_name_string);

                let is_list_or_pack = file_name_string.ends_with(".list") || file_name_string.ends_with(".pack");
                let is_shallow_directory = path_object.parent() == Some(Path::new("assets"))
                    || path_object.parent() == Some(Path::new(""));

                if !is_list_or_pack && !is_shallow_directory {
                    continue;
                }

                let is_junk_metadata = file_name_string.starts_with("META-INF")
                    || file_name_string.ends_with(".dex")
                    || file_name_string.ends_with(".arsc")
                    || file_name_string.ends_with(".xml")
                    || file_name_string.ends_with(".dat")
                    || file_name_string.ends_with(".lock");

                if is_junk_metadata {
                    continue;
                }

                let Some(safe_file_name) = path_object.file_name() else {
                    continue;
                };

                let destination_path = extraction_directory.join(safe_file_name);

                if let Ok(mut output_file) = fs::File::create(&destination_path) {
                    let _ = std::io::copy(&mut current_file, &mut output_file);
                }

                if file_name_string.ends_with(".list") {
                    extracted_lists.push(destination_path);
                } else if !file_name_string.ends_with(".pack") {
                    extracted_loose.push(destination_path);
                }
            }

            Some((extracted_lists, extraction_directory, extracted_loose))
        })
        .collect();

    let mut final_list_paths = Vec::new();
    let mut final_temporary_directories = Vec::new();
    let mut final_loose_paths = Vec::new();

    for (lists, temporary_directory, loose) in parallel_results {
        final_list_paths.extend(lists);
        final_temporary_directories.push(temporary_directory);
        final_loose_paths.extend(loose);
    }

    (final_list_paths, final_temporary_directories, final_loose_paths)
}
#[cfg(test)]
mod tests {
    use super::*;

    // Save state and build leftovers the game ships alongside its data; importing
    // them caused bogus one-file imports. PONOS uses csv/tsv, never txt.
    #[test]
    fn junk_never_reaches_the_import() {
        assert!(junk(Path::new("/pull/SAVE_DATA.OLD")));
        assert!(junk(Path::new("/pull/session_1.data")));
        assert!(junk(Path::new("/pull/session.version")));
        assert!(junk(Path::new("/pull/stamp-cert-sha256")));
        assert!(junk(Path::new("/pull/layout.txt")));
        assert!(junk(Path::new("/pull/aapt2.proto")));
        assert!(junk(Path::new("/pull/build.PROPERTIES")));

        assert!(!junk(Path::new("/pull/SAVE_DATA")), "only the .OLD backup is dropped");
        assert!(!junk(Path::new("/pull/unit001.csv")));
        assert!(!junk(Path::new("/pull/sessionless.png")), "the suffixes still have to match");
    }

    // An .xapk is a wrapper: no binary AndroidManifest.xml at its root, just the
    // split APKs and a plain-JSON manifest carrying the version.
    #[test]
    fn a_bundle_manifest_yields_the_build_its_json_declares() {
        let raw = br#"{"xapk_version":2,"package_name":"jp.co.ponos.battlecatsen","version_name":"15.5.0","version_code":"1505000"}"#;
        let build = bundle_build(raw, Path::new("BCEN-15.5.xapk")).expect("a build");

        assert_eq!(build.label, "jp.co.ponos.battlecatsen");
        assert_eq!(build.name, "15.5.0");
        assert_eq!(build.code, 1_505_000);
    }

    #[test]
    fn a_numeric_version_code_reads_the_same_as_a_quoted_one() {
        let raw = br#"{"package_name":"jp.co.ponos.battlecats","version_name":"15.6.0","version_code":1506000}"#;

        assert_eq!(bundle_build(raw, Path::new("x.xapk")).map(|build| build.code), Some(1_506_000));
    }

    // Without a version there is nothing to report, and the file stem is not a version.
    #[test]
    fn a_bundle_without_a_version_name_reports_nothing() {
        assert!(bundle_build(br#"{"package_name":"jp.co.ponos.battlecats"}"#, Path::new("x.xapk")).is_none());
        assert!(bundle_build(b"not json", Path::new("x.xapk")).is_none());
    }
}
