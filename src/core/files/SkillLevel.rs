use std::fs;
use std::path::Path;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct SkillLevelRow {
    pub id: i32,
}

impl SkillLevelRow {
    pub fn from_csv_line(_csv_line: &str) -> Option<Self> {
        Some(Self { id: 0 })
    }
}

pub fn load(cats_directory: &Path) -> HashMap<i32, SkillLevelRow> {
    let mut map = HashMap::new();
    let file_path = cats_directory.join("SkillLevel.csv");
    
    if let Ok(file_content) = fs::read_to_string(&file_path) {
        for (i, line) in file_content.lines().enumerate() {
            if let Some(row) = SkillLevelRow::from_csv_line(line) {
                map.insert(i as i32, row);
            }
        }
    } 
    map
}