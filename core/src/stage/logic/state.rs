use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::Receiver;

use nyanko::cat::unit::UnitBuy;
use nyanko::chapter::map::LockSkipDataEntry;
use serde::{Deserialize, Serialize};

use crate::cat::waiter::unitbuy;
use crate::enemy::logic::scanner::EnemyEntry;
use crate::enemy::waiter::enemyname;
use crate::global::formats::gatyaitembuy::{self, GatyaItemBuy};
use crate::global::formats::gatyaitemname::{self, GatyaItemName};
use crate::settings::logic::ScannerConfig;
use crate::stage::data::{scatcpusetting};
use crate::stage::registry::StageRegistry;
use crate::stage::waiter::{drop_chara, lockskipdata};

use super::loader;

#[derive(Deserialize, Serialize)]
#[serde(default)]
#[derive(Default)]
pub struct StageDataState {
    #[serde(skip)] pub registry: StageRegistry,
    pub search_query: String,
    pub selected_category: Option<String>,
    pub selected_map: Option<String>,
    pub selected_stage: Option<String>,

    #[serde(skip)] pub scan_receiver: Option<Receiver<StageRegistry>>,
    #[serde(skip)] pub enemy_registry: HashMap<u32, EnemyEntry>,
    #[serde(skip)] pub enemy_name_registry: Vec<String>,
    #[serde(skip)] pub item_buy_registry: HashMap<u32, GatyaItemBuy>,
    #[serde(skip)] pub item_name_registry: HashMap<usize, GatyaItemName>,
    #[serde(skip)] pub drop_chara_registry: HashMap<u32, u32>,
    #[serde(skip)] pub unit_buy_registry: HashMap<u32, UnitBuy>,
    #[serde(skip)] pub lock_skip_registry: HashMap<u32, LockSkipDataEntry>,
    #[serde(skip)] pub scat_cpu_setting: scatcpusetting::ScatCpuSetting,
    #[serde(skip)] pub active_language_priority: Vec<String>,
}


impl StageDataState {
    pub fn restart_scan(&mut self, scanner_configuration: ScannerConfig) {
        self.active_language_priority = scanner_configuration.language_priority.clone();
        let lang_priority = &scanner_configuration.language_priority;

        let enemies_directory_path = Path::new("game/enemies");
        self.enemy_name_registry = enemyname(
            enemies_directory_path,
            lang_priority
        );

        let tables_directory_path = Path::new("game/tables");
        self.item_buy_registry = gatyaitembuy::load(
            tables_directory_path,
            "Gatyaitembuy.csv",
            lang_priority
        );

        let names_directory_path = tables_directory_path.join("GatyaitemName");
        self.item_name_registry = gatyaitemname::load(
            &names_directory_path,
            "GatyaitemName.csv",
            lang_priority
        );

        let stages_directory_path = Path::new("game/stages");

        macro_rules! load_stage_file {
            ($module:ident, $filename:expr) => {
                $module::load(stages_directory_path, $filename, lang_priority)
            };
        }

        self.drop_chara_registry = drop_chara(stages_directory_path, "drop_chara.csv", lang_priority);
        self.lock_skip_registry = lockskipdata(stages_directory_path, "LockSkipData.csv", lang_priority);

        self.scat_cpu_setting = load_stage_file!(scatcpusetting, "ScatCPUsetting.csv");

        let cats_directory_path = Path::new("game/cats");
        self.unit_buy_registry = unitbuy(
            cats_directory_path,
            lang_priority
        );

        loader::restart_scan(self, scanner_configuration);
    }

    pub fn update_data(&mut self) {
        loader::update_data(self);
    }

    pub fn sync_enemies(&mut self, extracted_enemies_array: &[EnemyEntry]) {
        self.enemy_registry = extracted_enemies_array.iter().map(|enemy_entry| (enemy_entry.id, enemy_entry.clone())).collect();
    }
}