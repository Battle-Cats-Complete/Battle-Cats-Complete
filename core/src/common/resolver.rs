use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

static ACTIVE_MOD: RwLock<Option<String>> = RwLock::new(None);
static OVERRIDE_CACHE: RwLock<Option<HashMap<String, Option<PathBuf>>>> = RwLock::new(None);

pub fn set_active_mod(mod_name: Option<String>) {
    let mut changed = false;

    if let Ok(mut active) = ACTIVE_MOD.write()
        && *active != mod_name {
            *active = mod_name;
            changed = true;
        }

    if changed {
        clear_override_cache();
    }
}

pub fn clear_override_cache() {
    if let Ok(mut cache) = OVERRIDE_CACHE.write() {
        *cache = Some(HashMap::new());
    }
}

pub fn get_active_mod() -> Option<String> {
    if let Ok(active) = ACTIVE_MOD.read() {
        active.clone()
    } else {
        None
    }
}

pub fn is_mod_active() -> bool {
    get_active_mod().is_some()
}

pub fn get<I, S>(dir: &Path, filenames: I, priority: &[String]) -> Vec<PathBuf>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let names: Vec<String> = filenames.into_iter().map(|s| s.as_ref().to_string()).collect();

    let mut targets = Vec::new();
    for code in priority {
        if code == "--" { break; }

        for filename in &names {
            if code.is_empty() {
                targets.push(filename.clone());
            } else if let Some(name) = build_regional_name(filename, code) {
                targets.push(name);
            }
        }
    }

    let mut paths = Vec::new();

    for target in &targets {
        if let Some(p) = check_mod_override(target) {
            paths.push(p);
        } else {
            let local_path = dir.join(target);
            if local_path.exists() {
                paths.push(local_path);
            }
        }
    }

    paths.dedup();
    paths
}

fn check_mod_override(filename: &str) -> Option<PathBuf> {
    if let Ok(cache) = OVERRIDE_CACHE.read()
        && let Some(map) = cache.as_ref()
            && let Some(cached_result) = map.get(filename) {
                return cached_result.clone();
            }

    let active_mod = {
        let guard = ACTIVE_MOD.read().ok()?;
        guard.as_ref().cloned()?
    };

    let mod_dir = Path::new("mods").join(active_mod);
    let mut found_path = None;

    if let Ok(entries) = std::fs::read_dir(&mod_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }

            let target = path.join(filename);
            if target.exists() {
                found_path = Some(target);
                break;
            }
        }
    }

    if let Ok(mut cache) = OVERRIDE_CACHE.write()
        && let Some(map) = cache.as_mut() {
            map.insert(filename.to_string(), found_path.clone());
        }

    found_path
}

fn build_regional_name(base_filename: &str, lang_code: &str) -> Option<String> {
    if lang_code.is_empty() { return None; }
    let path_obj = Path::new(base_filename);
    let stem = path_obj.file_stem()?.to_str()?;
    let ext = path_obj.extension().unwrap_or_default().to_str().unwrap_or("");
    let ext_str = if ext.is_empty() { String::new() } else { format!(".{}", ext) };

    Some(format!("{}_{}{}", stem, lang_code, ext_str))
}