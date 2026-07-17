pub mod filter;
pub mod fixedlineup;
pub mod materials;
pub mod navigate;
pub mod paths;
pub mod patterns;
pub mod restrictions;
pub mod rules;
pub mod scanner;
pub mod treasure;
pub mod waiter;

use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use nyanko::cat::unit::UnitBuy;
use nyanko::chapter::Category;
use nyanko::chapter::map::LockSkipDataEntry;
use nyanko::chapter::stage::ScatCpuSetting;
pub use nyanko::chapter::Map;
pub use nyanko::chapter::Stage;
use serde::{Deserialize, Serialize};

use crate::common::formats::GatyaItemBuy;
use crate::common::formats::GatyaItemName;
use crate::modules::cat::waiter::{unitbuy, unitexplanation};
use crate::modules::enemy::scanner::EnemyEntry;
use crate::modules::enemy::waiter::enemyname;
use crate::modules::settings::ScannerConfig;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub struct GlobalMapId {
    pub category: Category,
    pub map: u32,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub struct GlobalStageId {
    pub category: Category,
    pub map: u32,
    pub stage: u32,
}

#[derive(Default, Deserialize, Serialize)]
pub struct StageRegistry {
    pub maps: HashMap<GlobalMapId, Map>,
    pub stages: HashMap<GlobalStageId, Stage>,
}

impl StageRegistry {
    pub fn clear_cache(&mut self) {
        self.maps.clear();
        self.stages.clear();
    }
}

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

        let enemy_dir = std::path::Path::new(paths::DIR_ENEMIES);
        self.enemy_name_registry = enemyname(enemy_dir, langs);

        let tables_dir = std::path::Path::new(paths::DIR_TABLES);
        self.item_buy_registry = crate::common::formats::gatyaitembuy::load(tables_dir, "Gatyaitembuy.csv", langs);

        let names_dir = paths::gatya_name_dir();
        self.item_name_registry = crate::common::formats::gatyaitemname::load(&names_dir, "GatyaitemName.csv", langs);

        let stages_dir = std::path::Path::new(paths::DIR_STAGES);
        self.drop_chara_registry = waiter::drop_chara(stages_dir, "drop_chara.csv", langs);
        self.lock_skip_registry = waiter::lockskipdata(stages_dir, "LockSkipData.csv", langs);
        self.scat_cpu_setting = waiter::scatcpusetting(stages_dir, "ScatCPUsetting.csv", langs);

        let cats_dir = std::path::Path::new(paths::DIR_CATS);
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

    pub fn restart_scan(&mut self, config: ScannerConfig) {
        tracing::info!("Initializing stage data scan sequence");

        self.initialized = false;
        self.load_dictionaries(&config);

        tracing::debug!("Clearing stage cache and starting background scan");
        self.registry.clear_cache();
        self.scan_receiver = Some(scanner::start_scan(&config));
    }

    pub fn update_data(&mut self) {
        let Some(rx) = &self.scan_receiver else { return; };
        if let Ok(new_reg) = rx.try_recv() {
            tracing::info!("Stage scan complete. Updating memory registry.");
            self.registry = new_reg;
            self.scan_receiver = None;
            self.initialized = true;
        }
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