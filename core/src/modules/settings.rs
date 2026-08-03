pub mod desktop;
pub mod lang;
pub mod nightly;
pub mod pem;

use std::fs;
use std::path::Path;

use indexmap::IndexMap;
use md5;
use nyanko::common::tools::variant::Region;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::common::io::json;

const EXPECTED_HASHES: [(&str, &str); 4] = [
    ("bac299d3cf278544782427ff7c71ef58", "6910fae125547fd957a505c67e1c72bd"),
    ("b9e48b02312e5b3dd60194a03157d70c", "45cad482726268e341f5759230ce8cff"),
    ("264a0ffd5f69d257284b93ae881ce2b6", "213cecb58af008964303ecb2cf0f5373"),
    ("3d22eafdcc4fc2a1379b103970b36217", "4cacdb0839634116caaf0b966638865b"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UpdateMode {
    #[default]
    Prompt,
    Ignore,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default, Debug)]
pub enum SidebarBehavior {
    #[default]
    Cover,
    Push,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct StageDataSettings {
    pub sidebar_behavior: SidebarBehavior,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default, Debug)]
pub enum ExportBehavior {
    #[default]
    Automatic,
    Create,
    Update,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    pub general: GeneralSettings,
    pub cat_data: CatDataSettings,
    pub enemy_data: EnemyDataSettings,
    pub game_data: GameDataSettings,
    pub animation: AnimSettings,
    pub mods: ModsSettings,
    pub stages: StageDataSettings,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ModsSettings {
    pub export_behavior: ExportBehavior,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GeneralSettings {
    #[serde(default = "lang::default_priority")]
    pub language_priority: Vec<String>,
    pub update_mode: UpdateMode,
    pub enable_nightly: bool,
    pub enable_logging: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language_priority: lang::default_priority(),
            update_mode: UpdateMode::default(),
            enable_nightly: false,
            enable_logging: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct CatDataSettings {
    pub preferred_banner_form: usize,
    pub show_invalid_cats: bool,
    pub expand_spirit_details: bool,
    pub default_level: i32,
    pub auto_level_calculations: bool,
    pub bump_ultra_60: bool,
}

impl Default for CatDataSettings {
    fn default() -> Self {
        Self {
            preferred_banner_form: 3,
            show_invalid_cats: false,
            expand_spirit_details: false,
            default_level: 50,
            auto_level_calculations: true,
            bump_ultra_60: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct EnemyDataSettings {
    pub show_invalid_enemies: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GameDataSettings {
    pub app_folder_persistence: bool,
    pub enforce_key_validation: bool,
}

impl Default for GameDataSettings {
    fn default() -> Self {
        Self {
            app_folder_persistence: false,
            enforce_key_validation: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AnimSettings {
    pub debug_view: bool,
    pub use_tight_bounds: bool,
    pub auto_set_camera_region: bool,
    pub default_showcase_walk: i32,
    pub default_showcase_idle: i32,
    pub default_showcase_kb: i32,
}

impl Default for AnimSettings {
    fn default() -> Self {
        Self {
            debug_view: false,
            use_tight_bounds: true,
            auto_set_camera_region: false,
            default_showcase_walk: 90,
            default_showcase_idle: 90,
            default_showcase_kb: 60,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScannerConfig {
    pub language_priority: Vec<String>,
    pub preferred_form: usize,
    pub show_invalid_cats: bool,
    pub show_invalid_enemies: bool,
}

#[derive(Clone, Debug)]
pub struct EmulatorConfig {
    pub keep_app_folder: bool,
}

impl Settings {
    pub fn scanner_config(&self) -> ScannerConfig {
        ScannerConfig {
            language_priority: self.general.language_priority.clone(),
            preferred_form: self.cat_data.preferred_banner_form,
            show_invalid_cats: self.cat_data.show_invalid_cats,
            show_invalid_enemies: self.enemy_data.show_invalid_enemies,
        }
    }

    pub fn emulator_config(&self) -> EmulatorConfig {
        EmulatorConfig {
            keep_app_folder: self.game_data.app_folder_persistence,
        }
    }
}

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

}

impl std::fmt::Display for RuleHandling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Include => "Include",
            Self::Only => "Only",
            Self::Ignore => "Ignore",
        };
        write!(f, "{}", label)
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
        let default_json = include_str!("settings/exceptions.json");
        serde_json::from_str(default_json).unwrap_or_else(|_| ExceptionList {
            source: RuleSource::Default,
            rules: vec![ExceptionRule::default()],
        })
    }
}

impl ExceptionList {
    pub fn save(&mut self) {
        self.source = RuleSource::Custom;
        if let Err(err) = json::save("exceptions.json", self) {
            warn!("Failed to save exceptions.json: {}", err);
        }
    }

    pub fn load_or_default() -> Self {
        json::load("exceptions.json").unwrap_or_default()
    }

    pub fn save_to_file(&mut self, path: &Path) -> Result<(), std::io::Error> {
        self.source = RuleSource::Custom;
        let json_string = serde_json::to_string_pretty(self)?;
        fs::write(path, json_string)
    }

    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&data).map_err(|e| e.to_string())
    }

    pub fn sync_on_boot() {
        let disk_list = json::load::<ExceptionList>("exceptions.json");

        let needs_overwrite = match disk_list {
            Some(list) => list.source == RuleSource::Default,
            None => true,
        };

        if needs_overwrite {
            info!("Syncing default exceptions.json to disk...");
            let default_list = Self::default();
            if let Err(err) = json::save("exceptions.json", &default_list) {
                warn!("Failed to save exceptions.json: {}", err);
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Debug)]
pub struct RegionKey {
    pub key: String,
    pub iv: String,
}

#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Debug)]
pub struct UserKeys {
    pub ja: RegionKey,
    pub en: RegionKey,
    pub tw: RegionKey,
    pub ko: RegionKey,
}

impl UserKeys {
    pub fn load() -> Self {
        json::load("keys.json").unwrap_or_default()
    }

    pub fn save(&self) {
        if let Err(err) = json::save("keys.json", self) {
            warn!("Failed to save keys.json: {}", err);
        }
    }

    pub fn is_empty(&self) -> bool {
        ![Region::Ja, Region::En, Region::Tw, Region::Ko]
            .iter()
            .any(|&region| self.has_key_for(region))
    }

    fn has_key_for(&self, region: Region) -> bool {
        match region {
            Region::Ja => !self.ja.key.is_empty() && !self.ja.iv.is_empty(),
            Region::En => !self.en.key.is_empty() && !self.en.iv.is_empty(),
            Region::Tw => !self.tw.key.is_empty() && !self.tw.iv.is_empty(),
            Region::Ko => !self.ko.key.is_empty() && !self.ko.iv.is_empty(),
        }
    }

    pub(crate) fn as_tuples(&self) -> Vec<(String, String, Region)> {
        let mut key_tuples = Vec::new();
        if self.has_key_for(Region::Ja) { key_tuples.push((self.ja.key.clone(), self.ja.iv.clone(), Region::Ja)); }
        if self.has_key_for(Region::En) { key_tuples.push((self.en.key.clone(), self.en.iv.clone(), Region::En)); }
        if self.has_key_for(Region::Tw) { key_tuples.push((self.tw.key.clone(), self.tw.iv.clone(), Region::Tw)); }
        if self.has_key_for(Region::Ko) { key_tuples.push((self.ko.key.clone(), self.ko.iv.clone(), Region::Ko)); }
        key_tuples
    }

    pub fn validate(&self) -> [(bool, bool); 4] {
        let check_hash = |input_value: &str, expected_hash: &str| -> bool {
            if expected_hash.is_empty() { return true; }
            let clean_value = input_value.trim();
            if clean_value.is_empty() { return false; }

            let hash_result = format!("{:x}", md5::compute(clean_value.as_bytes()));
            hash_result == expected_hash
        };

        [
            (check_hash(&self.ja.key, EXPECTED_HASHES[0].0), check_hash(&self.ja.iv, EXPECTED_HASHES[0].1)),
            (check_hash(&self.en.key, EXPECTED_HASHES[1].0), check_hash(&self.en.iv, EXPECTED_HASHES[1].1)),
            (check_hash(&self.tw.key, EXPECTED_HASHES[2].0), check_hash(&self.tw.iv, EXPECTED_HASHES[2].1)),
            (check_hash(&self.ko.key, EXPECTED_HASHES[3].0), check_hash(&self.ko.iv, EXPECTED_HASHES[3].1)),
        ]
    }
}