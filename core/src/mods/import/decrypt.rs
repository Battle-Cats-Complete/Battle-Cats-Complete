use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;

use tracing::{error, info, trace};
use nyanko::common::Region;
use nyanko::pack::cryptology;
use rayon::prelude::*;

use crate::settings::logic::keys::UserKeys;

struct PackEntry {
    name: String,
    offset: usize,
    size: usize,
}

fn format_log_name(name: &str, path: &Path) -> String {
    let cleaned = name.trim_start_matches("...\\").trim_start_matches(".../");
    if cleaned.len() > 35 {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        let trunc_stem: String = stem.chars().take(20).collect();
        if !ext.is_empty() && ext.len() <= 5 {
            format!("{}....{}", trunc_stem, ext)
        } else {
            format!("{}...", trunc_stem)
        }
    } else {
        cleaned.to_string()
    }
}

pub fn run(pack_file: &Path, workspace_dir: &Path, status_sender: Sender<String>, user_keys: &UserKeys) -> Result<(), String> {
    info!("Starting decryption for pack file: {:?}", pack_file);
    let list_path = pack_file.with_extension("list");

    if !list_path.exists() || !pack_file.exists() {
        error!("Missing required .pack or .list files");
        return Err("Missing required .pack or .list files.".to_string());
    }

    let patch_dir = workspace_dir.join("patch");
    fs::create_dir_all(&patch_dir).map_err(|error| {
        error!("Failed to create patch directory: {}", error);
        error.to_string()
    })?;

    let list_data = fs::read(&list_path).map_err(|error| error.to_string())?;
    let content = cryptology::decrypt_list(&list_data).map_err(|error| error.to_string())?;

    let mut entries = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 { continue; }

        entries.push(PackEntry {
            name: parts[0].to_string(),
            offset: parts[1].parse().unwrap_or(0),
            size: parts[2].parse().unwrap_or(0),
        });
    }

    let total_entries = entries.len();
    let log_interval = (total_entries / 10).max(1);

    let pack_data = fs::read(pack_file).map_err(|error| error.to_string())?;
    let owned_tuples = user_keys.as_tuples();
    let ref_tuples: Vec<(Region, &str, &str)> = owned_tuples.iter()
        .map(|(key_string, iv, region_enum)| (*region_enum, key_string.as_str(), iv.as_str()))
        .collect();

    let nyanko_keys = cryptology::Keys::parse(&ref_tuples).map_err(|error| error.to_string())?;

    let extracted_count = AtomicUsize::new(0);

    entries.into_par_iter().for_each(|entry| {
        let aligned_size = if entry.size % 16 == 0 { entry.size } else { ((entry.size / 16) + 1) * 16 };

        if entry.offset + aligned_size <= pack_data.len() {
            let chunk = &pack_data[entry.offset .. entry.offset + aligned_size];
            let (decrypted_bytes, _) = cryptology::decrypt_chunk(chunk, &entry.name, &nyanko_keys);
            let final_data = &decrypted_bytes[..std::cmp::min(entry.size, decrypted_bytes.len())];

            let out_file = patch_dir.join(&entry.name);
            if let Some(parent) = out_file.parent() { let _ = fs::create_dir_all(parent); }
            let _ = fs::write(out_file, final_data);

            let current = extracted_count.fetch_add(1, Ordering::Relaxed) + 1;

            if current % log_interval == 0 || current == total_entries {
                let path = Path::new(&entry.name);
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                let display_name = format_log_name(&file_name, path);
                trace!("Decrypted chunk stream: {}", display_name);
                let _ = status_sender.send(format!("Extracted {} Files | Streaming: {}", current, display_name));
            }
        }
    });

    Ok(())
}