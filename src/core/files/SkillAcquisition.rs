use std::fs;
use std::path::Path;
use std::collections::HashSet;

pub fn load(cats_directory: &Path) -> HashSet<i32> {
    let mut ids = HashSet::new();
    let file_path = cats_directory.join("SkillAcquisition.csv");
    
    if let Ok(file_content) = fs::read_to_string(&file_path) {
        for line in file_content.lines() {
            if let Some(first_part) = line.split(',').next() {
                if let Ok(id) = first_part.trim().parse::<i32>() {
                    ids.insert(id);
                }
            }
        }
    } 
    ids
}