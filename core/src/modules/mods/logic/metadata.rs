use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

fn default_source() -> String {
    "Battle Cats Complete".to_string()
}

fn default_package() -> String {
    "".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModMetadata {
    #[serde(default)] pub title: String,
    #[serde(default)] pub author: String,
    #[serde(default)] pub version: String,
    #[serde(default)] pub description: String,
    #[serde(default = "default_package")] pub package: String,
    #[serde(default = "default_source")] pub source: String,
}

impl Default for ModMetadata {
    fn default() -> Self {
        Self {
            title: String::new(),
            author: String::new(),
            version: String::new(),
            description: String::new(),
            package: default_package(),
            source: default_source(),
        }
    }
}

impl ModMetadata {
    pub fn load<P: AsRef<Path>>(mod_folder_path: P) -> Self {
        let meta_path = mod_folder_path.as_ref().join("patch").join("metadata.json");
        if let Ok(data) = fs::read_to_string(meta_path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save<P: AsRef<Path>>(&self, mod_folder_path: P) -> Result<(), std::io::Error> {
        let dl_dir = mod_folder_path.as_ref().join("patch");
        
        if !dl_dir.exists() {
            let _ = fs::create_dir_all(&dl_dir);
        }

        let meta_path = dl_dir.join("metadata.json");
        let data = serde_json::to_string_pretty(self)?;
        fs::write(meta_path, data)
    }
}