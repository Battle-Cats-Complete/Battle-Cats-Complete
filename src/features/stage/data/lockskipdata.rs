use std::fs;
use std::path::Path;
use std::collections::HashMap;
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

#[derive(Default, Debug, Clone)]
pub struct LockSkipEntry {
    #[allow(dead_code)] pub exclusion_message_type: u32,
    pub excluded_map_id: u32,
    #[allow(dead_code)] pub comment: String,
}

pub fn load(dir_path: &Path, filename: &str, lang_priority: &[String]) -> HashMap<u32, LockSkipEntry> {
    let mut registry = HashMap::new();
    let file_paths = resolver::get(dir_path, &[filename], lang_priority);
    let Some(first_path) = file_paths.first() else { return registry; };
    let Ok(file_content) = fs::read_to_string(first_path) else { return registry; };
    
    let csv_separator = detect_csv_separator(&file_content);
    
    for line in file_content.lines() {
        let parts: Vec<&str> = line.split("//").collect();
        let data_part = parts.first().unwrap_or(&"").trim();
        let comment_part = parts.get(1).unwrap_or(&"").trim();

        if data_part.is_empty() { continue; }
        
        let cols: Vec<&str> = data_part.split(csv_separator).collect();
        let message_type = cols.first().and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
        let stage_id = cols.get(1).and_then(|p| p.parse::<u32>().ok());

        if let Some(id) = stage_id {
            registry.insert(id, LockSkipEntry {
                exclusion_message_type: message_type,
                excluded_map_id: id,
                comment: comment_part.to_string(),
            });
        }
    }
    registry
}