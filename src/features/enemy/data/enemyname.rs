use std::fs;
use std::path::Path;

pub fn load(lang_dir: &Path, lang_code: &str) -> Vec<String> {
    // 1. Try target language (e.g., Enemyname_de.tsv)
    let mut target_path = lang_dir.join("Enemyname").join(format!("Enemyname_{}.tsv", lang_code));
    
    // 2. Fallback directly to base (Enemyname.tsv)
    if !target_path.exists() {
        target_path = lang_dir.join("Enemyname").join("Enemyname.tsv");
    }

    let mut names = Vec::new();
    
    if let Ok(content) = fs::read_to_string(&target_path) {
        for line in content.lines() {
            names.push(line.trim().to_string());
        }
    }
    
    names
}