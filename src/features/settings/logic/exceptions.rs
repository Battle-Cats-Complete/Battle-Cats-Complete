use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::fs;
use directories::ProjectDirs;

pub fn get_config_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("", "", "battle_cats_complete") {
        let data_dir = proj_dirs.data_dir();
        if !data_dir.exists() {
            let _ = fs::create_dir_all(data_dir);
        }
        data_dir.join("exceptions.json")
    } else {
        PathBuf::from("data/exceptions.json")
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum NameLogic {
    Contains,
    Only,
}

impl Default for NameLogic {
    fn default() -> Self { Self::Contains }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum LangLogic {
    Append,
    Only,
}

impl Default for LangLogic {
    fn default() -> Self { Self::Append }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionRule {
    pub prefix: String,
    pub suffix: String,
    pub extension: String,
    pub name_logic: NameLogic,
    pub languages: BTreeMap<String, bool>,
    pub lang_logic: LangLogic,
}

impl Default for ExceptionRule {
    fn default() -> Self {
        let mut languages = BTreeMap::new();
        for lang in ["en", "ja", "tw", "ko", "es", "de", "fr", "it", "th"] {
            languages.insert(lang.to_string(), false);
        }
        Self {
            prefix: String::new(),
            suffix: String::new(),
            extension: String::new(),
            name_logic: NameLogic::Contains,
            languages,
            lang_logic: LangLogic::Append,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionList {
    pub rules: Vec<ExceptionRule>,
}

impl Default for ExceptionList {
    fn default() -> Self {
        let default_json = include_str!("exceptions.json");
        serde_json::from_str(default_json).unwrap_or_else(|_| ExceptionList { 
            rules: vec![ExceptionRule::default()] 
        })
    }
}

impl ExceptionList {
    pub fn save_to_file(&self, path: &Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let list: ExceptionList = serde_json::from_str(&data).map_err(|e| e.to_string())?;
        Ok(list)
    }

    pub fn load_or_default(path: &Path) -> Self {
        if path.exists() {
            if let Ok(list) = Self::load_from_file(path) {
                return list;
            }
        }
        
        let default_list = Self::default();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = default_list.save_to_file(path);
        
        default_list
    }
}