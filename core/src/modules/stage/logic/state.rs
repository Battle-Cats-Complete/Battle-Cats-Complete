use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::Receiver;

use nyanko::cat::unit::UnitBuy;
use nyanko::chapter::Category;
use nyanko::chapter::map::LockSkipDataEntry;
use nyanko::chapter::stage::ScatCpuSetting;
use serde::{Deserialize, Serialize};

use crate::modules::cat::waiter::{unitbuy, unitexplanation};
use crate::modules::enemy::scanner::EnemyEntry;
use crate::modules::enemy::waiter::enemyname;
use crate::common::formats::gatyaitembuy::{self, GatyaItemBuy};
use crate::common::formats::gatyaitemname::{self, GatyaItemName};
use crate::modules::settings::ScannerConfig;
use crate::modules::stage::paths;
use crate::modules::stage::registry::{GlobalMapId, GlobalStageId, StageRegistry};
use crate::modules::stage::waiter::{drop_chara, lockskipdata, scatcpusetting};

use super::loader;

#[derive(Default, Deserialize, Serialize)]
pub struct StageDataState {
    #[serde(skip)] pub registry: StageRegistry,
    pub search_query: String,
    pub selected_category: Option<Category>,
    pub selected_map: Option<GlobalMapId>,
    pub selected_stage: Option<GlobalStageId>,

    #[serde(skip)] pub initialized: bool,
    #[serde(skip)] pub scan_receiver: Option<Receiver<StageRegistry>>,
    #[serde(skip)] pub enemy_registry: HashMap<u32, EnemyEntry>,
    #[serde(skip)] pub enemy_name_registry: Vec<String>,
    #[serde(skip)] pub item_buy_registry: HashMap<u32, GatyaItemBuy>,
    #[serde(skip)] pub item_name_registry: HashMap<usize, GatyaItemName>,
    #[serde(skip)] pub drop_chara_registry: HashMap<u32, u32>,
    #[serde(skip)] pub unit_buy_registry: HashMap<u32, UnitBuy>,
    #[serde(skip)] pub cat_name_registry: HashMap<u32, Vec<String>>,
    #[serde(skip)] pub lock_skip_registry: HashMap<u32, LockSkipDataEntry>,
    #[serde(skip)] pub scat_cpu_setting: ScatCpuSetting,
    #[serde(skip)] pub active_language_priority: Vec<String>,
}

impl StageDataState {
    #[tracing::instrument(level = "debug", skip(self, config))]
    pub fn load_dictionaries(&mut self, config: &ScannerConfig) {
        tracing::trace!("Loading auxiliary stage dictionaries");

        self.active_language_priority = config.language_priority.clone();
        let langs = &config.language_priority;

        let enemy_dir = Path::new(paths::DIR_ENEMIES);
        self.enemy_name_registry = enemyname(enemy_dir, langs);

        let tables_dir = Path::new(paths::DIR_TABLES);
        self.item_buy_registry = gatyaitembuy::load(tables_dir, "Gatyaitembuy.csv", langs);

        let names_dir = paths::gatya_name_dir();
        self.item_name_registry = gatyaitemname::load(&names_dir, "GatyaitemName.csv", langs);

        let stages_dir = Path::new(paths::DIR_STAGES);
        self.drop_chara_registry = drop_chara(stages_dir, "drop_chara.csv", langs);
        self.lock_skip_registry = lockskipdata(stages_dir, "LockSkipData.csv", langs);
        self.scat_cpu_setting = scatcpusetting(stages_dir, "ScatCPUsetting.csv", langs);

        let cats_dir = Path::new(paths::DIR_CATS);
        self.unit_buy_registry = unitbuy(cats_dir, langs);

        let mut cat_names = HashMap::new();
        for &unit_id in self.unit_buy_registry.keys() {
            let cat_folder = paths::cat_folder(unit_id);
            let expl = unitexplanation(unit_id, &cat_folder, langs);
            let names: Vec<String> = expl.names
                .into_iter()
                .flatten()
                .map(|n| n.to_lowercase())
                .collect();
            cat_names.insert(unit_id, names);
        }
        self.cat_name_registry = cat_names;
    }

    #[tracing::instrument(level = "debug", skip(self, config))]
    pub fn restart_scan(&mut self, config: ScannerConfig) {
        tracing::info!("Initializing stage data scan sequence");

        self.initialized = false;
        self.load_dictionaries(&config);

        tracing::debug!("Delegating thread scan to loader");
        loader::restart_scan(self, config);
    }

    pub fn update_data(&mut self) {
        loader::update_data(self);
    }

    #[tracing::instrument(level = "trace", skip(self, extracted))]
    pub fn sync_enemies(&mut self, extracted: &[EnemyEntry]) {
        tracing::trace!("Syncing {} enemies to stage registry", extracted.len());

        self.enemy_registry = extracted
            .iter()
            .map(|enemy| (enemy.id, enemy.clone()))
            .collect();
    }
}