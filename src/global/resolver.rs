use std::path::{Path, PathBuf};

/// Finds every valid version of a file in priority order.
/// 
/// Priority Hierarchy: ALL Mod variants -> ALL Game variants.
/// Stops processing completely when it hits the "--" (None) token.
pub fn get(dir: &Path, filename: &str, priority: &[String]) -> Vec<PathBuf> {
    // 1. Pre-calculate the exact filenames we care about
    let mut targets = Vec::new();
    for code in priority {
        if code == "--" { break; } // The Hard Stop
        
        if code.is_empty() {
            targets.push(filename.to_string());
        } else if let Some(name) = build_regional_name(filename, code) {
            targets.push(name);
        }
    }

    let mut paths = Vec::new();

    // 2. PASS 1: Check ALL Mod variants in priority order
    for target in &targets {
        if let Some(p) = check_mod_override(dir, target) {
            paths.push(p);
        }
    }

    // 3. PASS 2: Check ALL Game variants in priority order
    for target in &targets {
        let local_path = dir.join(target);
        if local_path.exists() {
            paths.push(local_path);
        }
    }

    paths.dedup();
    paths
}

fn check_mod_override(_target_folder: &Path, _filename: &str) -> Option<PathBuf> {
    // TODO: [MOD MANAGER HOOK]
    None
}

fn build_regional_name(base_filename: &str, lang_code: &str) -> Option<String> {
    if lang_code.is_empty() { return None; }
    let path_obj = Path::new(base_filename);
    let stem = path_obj.file_stem()?.to_str()?;
    
    // Deprciated blacklist system, kept here to make compiler happy
    if is_blacklisted_regional_base(stem) {
        return None;
    }

    let ext = path_obj.extension().unwrap_or_default().to_str().unwrap_or("");
    let ext_str = if ext.is_empty() { String::new() } else { format!(".{}", ext) };
    
    Some(format!("{}_{}{}", stem, lang_code, ext_str))
}

// Deprciated blacklist system, kept here to make compiler happy
// Conflicting assets now handled by decrypter/sorter
fn is_blacklisted_regional_base(stem: &str) -> bool {
    // Check for udi (banners) and uni (icons)
    if (stem.starts_with("udi") || stem.starts_with("uni")) && stem.len() >= 6 {
        let id_part = &stem[3..6];
        if let Ok(id) = id_part.parse::<u32>() {
            // IDs 000 through 008 are the 9 Basic Cats
            if id <= 9 { 
                return true;
            }
        }
    }
    false
}