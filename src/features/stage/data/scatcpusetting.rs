use std::fs;
use std::path::Path;
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

#[derive(Default, Debug, Clone)]
pub struct ScatCpuSetting {
    pub unknown_1: u32,
    pub super_cpu_daily_limit: u32,
    pub super_cpu_consume_amount: u32,
}

pub fn load(dir_path: &Path, filename: &str, lang_priority: &[String]) -> ScatCpuSetting {
    let mut setting = ScatCpuSetting::default();
    let file_paths = resolver::get(dir_path, &[filename], lang_priority);
    let Some(first_path) = file_paths.first() else { return setting; };
    let Ok(file_content) = fs::read_to_string(first_path) else { return setting; };
    
    let csv_separator = detect_csv_separator(&file_content);
    
    for line in file_content.lines() {
        let clean_line = line.split("//").next().unwrap_or("").trim();
        if clean_line.is_empty() { continue; }
        
        let parts: Vec<&str> = clean_line.split(csv_separator).collect();
        setting.unknown_1 = parts.first().and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
        setting.super_cpu_daily_limit = parts.get(1).and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
        setting.super_cpu_consume_amount = parts.get(2).and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
        break; 
    }
    setting
}