use std::fs;
use std::path::Path;

pub fn load(lang_dir: &Path, lang_code: &str) -> Vec<Vec<String>> {
    // 1. Try target language (e.g., EnemyPictureBook_de.csv)
    let mut target_path = lang_dir.join("EnemyPictureBook").join(format!("EnemyPictureBook_{}.csv", lang_code));
    
    // 2. Fallback directly to base (EnemyPictureBook.csv)
    if !target_path.exists() {
        target_path = lang_dir.join("EnemyPictureBook").join("EnemyPictureBook.csv");
    }

    let mut descriptions = Vec::new();
    
    if let Ok(content) = fs::read_to_string(&target_path) {
        for line in content.lines() {
            let cols: Vec<&str> = line.split('|').collect();
            let mut desc_lines = Vec::new();
            
            for col in cols.into_iter().skip(1) {
                let text = col.trim();
                if text.is_empty() || text.starts_with("仮") {
                    continue;
                }
                desc_lines.push(text.to_string());
            }
            
            descriptions.push(desc_lines);
        }
    }
    
    descriptions
}