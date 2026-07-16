use std::fs;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use zip::ZipArchive;

pub fn find_files(
    search_directory: &Path,
    list_paths: &mut Vec<PathBuf>,
    apk_paths: &mut Vec<PathBuf>,
    loose_paths: &mut Vec<PathBuf>
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

pub fn extract_all(apk_paths: &[PathBuf]) -> (Vec<PathBuf>, Vec<PathBuf>, Vec<PathBuf>) {
    if apk_paths.is_empty() {
        return (Vec::new(), Vec::new(), Vec::new());
    }

    let parallel_results: Vec<(Vec<PathBuf>, PathBuf, Vec<PathBuf>)> = apk_paths.par_iter().filter_map(|apk_file_path| {
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
            let mut current_file = match archive_reader.by_index(index) {
                Ok(file) => file,
                Err(_) => continue,
            };

            if current_file.is_dir() {
                continue;
            }

            let file_name_string = current_file.name().to_string();
            let path_object = Path::new(&file_name_string);

            let is_list_or_pack = file_name_string.ends_with(".list") || file_name_string.ends_with(".pack");
            let is_shallow_directory = path_object.parent() == Some(Path::new("assets")) || path_object.parent() == Some(Path::new(""));

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
    }).collect();

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