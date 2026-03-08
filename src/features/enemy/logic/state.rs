use eframe::egui;
use serde::{Deserialize, Serialize};
use crate::features::enemy::logic::scanner::{self, EnemyEntry};
use crate::features::settings::logic::Settings;
use crate::features::settings::logic::handle::ScannerConfig;
use crate::features::enemy::ui::list::EnemyList;
use crate::features::enemy::ui::master;

pub const TOP_PANEL_PADDING: f32 = 2.5;
pub const SEARCH_FILTER_GAP: f32 = 5.0;
pub const SPACE_BEFORE_SEPARATOR: f32 = 2.0;
pub const SPACE_AFTER_SEPARATOR: f32 = 2.0;

#[derive(Deserialize, Serialize, PartialEq, Clone, Copy)]
pub enum EnemyDetailTab {
    Stats,
    Description,
    Animation,
}

impl Default for EnemyDetailTab {
    fn default() -> Self { Self::Stats }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct EnemyListState {
    #[serde(skip)] pub entries: Vec<EnemyEntry>,
    pub selected_enemy: Option<u32>,
    pub search_query: String,
    pub selected_tab: EnemyDetailTab,
    #[serde(skip)] pub list_ui: EnemyList,
    #[serde(skip)] pub initialized: bool,
}

impl Default for EnemyListState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            selected_enemy: None,
            search_query: String::new(),
            selected_tab: EnemyDetailTab::default(),
            list_ui: EnemyList::default(),
            initialized: false,
        }
    }
}

impl EnemyListState {
    pub fn load_enemies(&mut self, config: &ScannerConfig) {
        self.entries = scanner::scan_all(config);
    }
}

pub fn show(ctx: &egui::Context, state: &mut EnemyListState, settings: &mut Settings) {
    if !state.initialized {
        state.initialized = true;
        if !settings.unit_persistence {
            state.selected_enemy = None;
            state.list_ui.reset_scroll();
        }
    }

    egui::SidePanel::left("enemy_list_panel")
        .resizable(false)
        .default_width(160.0)
        .show(ctx, |ui| {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(TOP_PANEL_PADDING);
                
                ui.vertical_centered(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text(egui::RichText::new("Search Enemy...").color(egui::Color32::GRAY))
                        .desired_width(140.0));
                });
                
                ui.add_space(SPACE_BEFORE_SEPARATOR + SEARCH_FILTER_GAP);
                ui.separator();
                ui.add_space(SPACE_AFTER_SEPARATOR);
            });

            if !state.entries.is_empty() {
                state.list_ui.show(ctx, ui, &state.entries, &mut state.selected_enemy, &state.search_query);
            } else {
                ui.centered_and_justified(|ui| { ui.spinner(); });
            }
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        if state.entries.is_empty() {
            ui.centered_and_justified(|ui| { ui.heading("No Enemy Data Found"); });
            return;
        }

        let Some(selected_id) = state.selected_enemy else {
            ui.centered_and_justified(|ui| { ui.label("Select an Enemy"); });
            return;
        };

        let Some(enemy_entry) = state.entries.iter().find(|e| e.id == selected_id) else {
            return;
        };

        master::show(ctx, ui, enemy_entry, &mut state.selected_tab, settings);
    });
}