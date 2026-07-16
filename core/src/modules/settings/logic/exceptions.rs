use std::fs;
use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub enum RuleHandling {
    Include,
    Only,
    Ignore,
}

impl RuleHandling {
    pub fn all() -> [Self; 3] {
        [Self::Include, Self::Only, Self::Ignore]
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Include => "Include".to_string(),
            Self::Only => "Only".to_string(),
            Self::Ignore => "Ignore".to_string(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuleSource {
    #[default]
    Default,
    Custom,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionRule {
    pub pattern: String,
    pub extension: String,
    pub handling: RuleHandling,
    pub languages: IndexMap<String, bool>,
}

impl Default for ExceptionRule {
    fn default() -> Self {
        let mut languages = IndexMap::new();
        for lang in ["en", "ja", "tw", "ko", "es", "de", "fr", "it", "th"] {
            languages.insert(lang.to_string(), false);
        }
        Self {
            pattern: String::new(),
            extension: String::new(),
            handling: RuleHandling::Include,
            languages,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ExceptionList {
    #[serde(default)]
    pub source: RuleSource,
    pub rules: Vec<ExceptionRule>,
}

impl Default for ExceptionList {
    fn default() -> Self {
        let default_json = include_str!("exceptions.json");
        serde_json::from_str(default_json).unwrap_or_else(|_| ExceptionList {
            source: RuleSource::Default,
            rules: vec![ExceptionRule::default()]
        })
    }
}

impl ExceptionList {
    pub fn save(&mut self) {
        self.source = RuleSource::Custom;
        crate::common::io::json::save("exceptions.json", self);
    }

    pub fn load_or_default() -> Self {
        crate::common::io::json::load("exceptions.json").unwrap_or_default()
    }

    pub fn save_to_file(&mut self, path: &Path) -> Result<(), std::io::Error> {
        self.source = RuleSource::Custom;
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    pub fn sync_on_boot() {
        let disk_list = crate::common::io::json::load::<ExceptionList>("exceptions.json");

        let needs_overwrite = match disk_list {
            Some(list) => list.source == RuleSource::Default,
            None => true,
        };

        if needs_overwrite {
            tracing::info!("Syncing default exceptions.json to disk...");
            let default_list = Self::default();
            crate::common::io::json::save("exceptions.json", &default_list);
        }
    }
}