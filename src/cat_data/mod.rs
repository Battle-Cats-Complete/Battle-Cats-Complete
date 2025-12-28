use eframe::egui;
use std::sync::mpsc::Receiver;

mod scanner;
mod list; 
use scanner::CatEntry;
use list::CatList;

pub struct CatListState {
    pub cats: Vec<CatEntry>,
    pub selected_cat: Option<u32>,
    pub cat_list: CatList,
    pub scan_receiver: Option<Receiver<CatEntry>>,
}

impl Default for CatListState {
    fn default() -> Self {
        Self {
            cats: Vec::new(),
            selected_cat: None,
            cat_list: CatList::default(),
            scan_receiver: Some(scanner::start_scan()),
        }
    }
}

impl CatListState {
    pub fn update_data(&mut self) {
        if let Some(rx) = &self.scan_receiver {
            let mut found_new = false;
            let mut count = 0;
            while count < 5 { 
                if let Ok(cat) = rx.try_recv() {
                    self.cats.push(cat);
                    found_new = true;
                    count += 1;
                } else {
                    break;
                }
            }

            if found_new {
                self.cats.sort_by_key(|u| u.id);
            }
        }
    }
    pub fn refresh(&mut self) {
        // 1. Clear the data list
        self.cats.clear();
        
        // 2. Clear the texture cache 
        self.cat_list.clear_cache(); 
        
        // 3. Restart the background scanner
        self.scan_receiver = Some(scanner::start_scan());
    }
}

pub fn show(ctx: &egui::Context, state: &mut CatListState) {
    egui::SidePanel::left("cat_list_panel")
        .resizable(false)
        .default_width(160.0)
        .show(ctx, |ui| {
            ui.heading("Cat Data Repository");
            ui.separator();
            state.cat_list.show(ctx, ui, &state.cats, &mut state.selected_cat);
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical(|ui| {
            if let Some(id) = state.selected_cat {
                ui.heading(format!("Selected Cat: {:03}", id));
                ui.label("Selection logic to be implemented later.");
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a cat from the left list.");
                });
            }
        });
    });
}