use eframe::egui;
use std::sync::mpsc::{Receiver, TryRecvError};
use crate::functions::SoftReset;
use serde::{Deserialize, Serialize};

pub mod scanner;
pub mod list;
pub mod cat; 
pub mod sprites;
pub mod stats;
pub mod abilities; 

use scanner::CatEntry;
use list::CatList;
use sprites::SpriteSheet;

#[derive(Deserialize, Serialize)]
#[serde(default)] 
pub struct CatListState {
    #[serde(skip)] 
    pub cats: Vec<CatEntry>,
    
    #[serde(alias = "persistent_id")] 
    pub selected_cat: Option<u32>,
    
    pub search_query: String, 
    pub selected_form: usize, 
    pub level_input: String, 
    pub current_level: i32, 

    #[serde(skip)] 
    pub cat_list: CatList,
    
    #[serde(skip)]
    pub scan_receiver: Option<Receiver<CatEntry>>,
    
    #[serde(skip)]
    pub detail_texture: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub detail_key: String, 
    
    #[serde(skip)]
    pub sprite_sheet: SpriteSheet, 
    #[serde(skip)]
    pub multihit_texture: Option<egui::TextureHandle>,

    #[serde(skip)]
    pub initialized: bool,
}

impl Default for CatListState {
    fn default() -> Self {
        Self {
            cats: Vec::new(),
            selected_cat: None,
            cat_list: CatList::default(),
            scan_receiver: None,
            search_query: String::new(),
            selected_form: 0,
            
            level_input: "50".to_string(),
            current_level: 50,
            
            detail_texture: None,
            detail_key: String::new(),
            
            sprite_sheet: SpriteSheet::default(), 
            multihit_texture: None,
            
            initialized: false, 
        }
    }
}

impl SoftReset for CatListState {
    fn reset(&mut self) {
        self.cats.clear();
        self.cat_list.clear_cache();
        self.detail_texture = None;
        self.detail_key.clear();
        self.selected_cat = None;
        self.selected_form = 0; 
        self.search_query.clear(); 
        self.level_input = "50".to_string();
        self.current_level = 50;
        self.sprite_sheet = SpriteSheet::default(); 
        self.multihit_texture = None; 
        self.scan_receiver = None;
    }
}

impl CatListState {
    pub fn update_data(&mut self) {
        let Some(rx) = &self.scan_receiver else { return };

        let mut new_data = false;
        let mut scan_complete = false;

        loop {
            match rx.try_recv() {
                Ok(entry) => {
                    self.cats.push(entry);
                    new_data = true;
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    scan_complete = true;
                    break;
                }
            }
        }

        if new_data { 
            self.cats.sort_by_key(|c| c.id);
        }

        // Only default select 0 if we actually have cats
        if self.selected_cat.is_none() && !self.cats.is_empty() {
            self.selected_cat = Some(self.cats[0].id);
        }
        
        if scan_complete {
            // Validate selection
            if let Some(target) = self.selected_cat {
                if !self.cats.iter().any(|c| c.id == target) {
                    if let Some(first) = self.cats.first() {
                        self.selected_cat = Some(first.id);
                    } else {
                        self.selected_cat = None;
                    }
                }
            }
            self.scan_receiver = None;
        }
    }

    pub fn restart_scan(&mut self, language: &str) {
        let current_selection = self.selected_cat;
        self.reset();
        self.selected_cat = current_selection;
        self.scan_receiver = Some(scanner::start_scan(language.to_string()));
    }
}

pub fn show(ctx: &egui::Context, state: &mut CatListState, settings: &crate::settings::Settings) {
    if !state.initialized {
        state.initialized = true;
        if !settings.unit_persistence {
            state.selected_cat = None;
            state.cat_list.reset_scroll();
        }
    }

    egui::SidePanel::left("cat_list_panel")
        .resizable(false)
        .default_width(160.0)
        .show(ctx, |ui| {
            ui.add_space(12.0); 
            ui.vertical_centered(|ui| {
                ui.add(egui::TextEdit::singleline(&mut state.search_query)
                    .hint_text("Search Cat...")
                    .desired_width(140.0));
            });
            ui.add_space(6.0); 
            ui.separator();

            let old_selection = state.selected_cat;
            
            state.cat_list.show(ctx, ui, &state.cats, &mut state.selected_cat, &state.search_query, settings.high_banner_quality);
            
            if state.selected_cat != old_selection {
                state.selected_form = 0; 
                state.detail_texture = None; 
                state.detail_key.clear();
            }
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        // --- LOADING / EMPTY STATE HANDLING ---
        if state.cats.is_empty() {
            ui.centered_and_justified(|ui| {
                if state.scan_receiver.is_some() {
                    ui.vertical(|ui| {
                        ui.spinner();
                        ui.add_space(10.0);
                        ui.label("Loading Unit Data...");
                    });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.heading("No Data Found");
                        ui.label("Could not find any units in game/cats.");
                        ui.add_space(5.0);
                        ui.label("Check that 'unitbuy.csv' exists and");
                        ui.label("unit folders (000, 001...) are present.");
                        ui.add_space(15.0);
                        if ui.button("Retry Scan").clicked() {
                            state.restart_scan(&settings.game_language);
                        }
                    });
                }
            });
            return;
        }

        // --- NORMAL DISPLAY ---
        let Some(selected_id) = state.selected_cat else {
            // Should theoretically be handled by update_data, but safe fallback
            ui.centered_and_justified(|ui| { ui.label("Select a Unit"); });
            return;
        };

        let Some(cat) = state.cats.iter().find(|c| c.id == selected_id) else {
            ui.centered_and_justified(|ui| { ui.spinner(); }); // Transient state
            return;
        };
        
        cat::show(
            ctx, 
            ui, 
            cat, 
            &mut state.selected_form,
            &mut state.level_input,   
            &mut state.current_level, 
            &mut state.detail_texture, 
            &mut state.detail_key,
            &mut state.sprite_sheet,
            &mut state.multihit_texture,
            settings
        );
    });
}