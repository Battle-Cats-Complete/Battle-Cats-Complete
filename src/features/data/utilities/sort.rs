use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::features::settings::logic::exceptions::RuleHandling;
use crate::global::io::patterns;
use crate::features::data::utilities::rules;

pub struct SortedRawFile {
    pub original_path: PathBuf,
    pub resolved_name: String,
    pub region_code: String,
}

// Processes a list of raw files by applying regex exception rules
pub fn process_raw_files(
    files: Vec<PathBuf>,
    source_directory: &str,
    language_priority: &[String],
) -> Vec<SortedRawFile> {
    let (compiled_regex_set, compiled_exception_rules) = rules::compile();
    let mut file_groups: HashMap<String, Vec<SortedRawFile>> = HashMap::new();

    // Infer default region from the source directory name
    let source_path = Path::new(source_directory);
    let mut folder_region_name = source_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    if folder_region_name == "files" {
        if let Some(parent) = source_path.parent() {
            folder_region_name = parent.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        }
    }
    
    let inferred_region = match folder_region_name.as_str() {
        s if s.ends_with("tw") => "tw",
        s if s.ends_with("ko") => "ko",
        s if s.ends_with("en") => "en",
        s if s.ends_with("battlecats") => "ja",
        _ => "", // No matching criteria -> Treat as Base
    };

    // 2. First pass: check if there are any localized sibling folders (Global Version specific)
    let mut has_global_siblings = false;
    for path in &files {
        if let Some(parent) = path.parent() {
            if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                if parent_name.starts_with("resLocal_") {
                    has_global_siblings = true;
                    break;
                }
            }
        }
    }

    // Assign base pack region
    let base_pack_region = if has_global_siblings {
        "en".to_string()
    } else {
        inferred_region.to_string() // Will be "" if it's an unrecognized folder
    };

    for path in files {
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else { continue; };
        
        let mut region_code = base_pack_region.clone();
        
        // Check exact parent folder for sub-region hints
        if let Some(parent) = path.parent() {
            if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                if parent_name == "resLocal" {
                    region_code = "en".to_string(); // resLocal is explicitly English base
                } else if let Some(stripped) = parent_name.strip_prefix("resLocal_") {
                    region_code = stripped.to_string();
                }
            }
        }

        // Apply regex exceptions
        let matched_user_rule = compiled_regex_set.matches(file_name).into_iter().next().map(|index| &compiled_exception_rules[index]);
        
        if let Some(rule) = matched_user_rule {
            if rule.handling == RuleHandling::Ignore { continue; }
        }

        let mut final_resolved_filename = file_name.to_string();

        if let Some(rule) = matched_user_rule {
            // ONLY apply language renaming if we successfully identified a region.
            // If region_code is empty (""), it skips this block entirely and imports as base!
            if !region_code.is_empty() && rule.languages.values().any(|&is_active| is_active) {
                let asset_stem_string = path.file_stem().unwrap_or_default().to_string_lossy();
                let asset_extension_string = path.extension().unwrap_or_default().to_string_lossy();
                
                let mut cleaned_stem = asset_stem_string.to_string();
                for &(code, _) in patterns::APP_LANGUAGES {
                    let suffix = format!("_{}", code);
                    if cleaned_stem.ends_with(&suffix) { 
                        cleaned_stem = cleaned_stem.trim_end_matches(&suffix).to_string(); 
                        break; 
                    }
                }

                let is_region_enabled = rule.languages.get(&region_code).copied().unwrap_or(false);
                if rule.handling == RuleHandling::Only && !is_region_enabled { continue; }
                
                let is_single = rule.handling == RuleHandling::Only && rule.languages.values().filter(|&&is_active| is_active).count() == 1;

                if is_region_enabled {
                    if is_single {
                        final_resolved_filename = if asset_extension_string.is_empty() { cleaned_stem } else { format!("{}.{}", cleaned_stem, asset_extension_string) };
                    } else {
                        final_resolved_filename = if asset_extension_string.is_empty() { format!("{}_{}", cleaned_stem, region_code) } else { format!("{}_{}.{}", cleaned_stem, region_code, asset_extension_string) };
                    }
                }
            }
        }

        file_groups
            .entry(final_resolved_filename.clone())
            .or_default()
            .push(SortedRawFile {
                original_path: path.clone(),
                resolved_name: final_resolved_filename,
                region_code,
            });
    }

    // Resolve collisions based on language priority
    let mut final_files = Vec::new();
    for (_, mut versions) in file_groups {
        if versions.len() == 1 {
            if let Some(single) = versions.pop() {
                final_files.push(single);
            }
            continue;
        }

        let mut best_file = None;
        let mut best_rank = usize::MAX;

        for file in versions {
            let rank = language_priority
                .iter()
                .position(|l| l == &file.region_code)
                .unwrap_or(usize::MAX);
            if rank < best_rank {
                best_rank = rank;
                best_file = Some(file);
            }
        }

        if let Some(winner) = best_file {
            final_files.push(winner);
        }
    }

    final_files
}