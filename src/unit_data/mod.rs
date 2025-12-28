use eframe::egui;
use std::sync::mpsc::Receiver;

mod scanner;
mod list; 
use scanner::UnitEntry;
use list::UnitList;

// RENAMED: UnitDataState -> UnitListState
pub struct UnitListState {
    pub units: Vec<UnitEntry>,
    pub selected_unit: Option<u32>,
    pub unit_list: UnitList,
    pub scan_receiver: Option<Receiver<UnitEntry>>,
}

impl Default for UnitListState {
    fn default() -> Self {
        Self {
            units: Vec::new(),
            selected_unit: None,
            unit_list: UnitList::default(),
            scan_receiver: Some(scanner::start_scan()),
        }
    }
}

impl UnitListState {
    pub fn update_data(&mut self) {
        if let Some(rx) = &self.scan_receiver {
            let mut found_new = false;
            let mut count = 0;
            while count < 5 { 
                if let Ok(unit) = rx.try_recv() {
                    self.units.push(unit);
                    found_new = true;
                    count += 1;
                } else {
                    break;
                }
            }

            if found_new {
                self.units.sort_by_key(|u| u.id);
            }
        }
    }
    pub fn refresh(&mut self) {
        // 1. Clear the data list
        self.units.clear();
        
        // 2. Clear the texture cache 
        self.unit_list.clear_cache(); 
        
        // 3. Restart the background scanner
        self.scan_receiver = Some(scanner::start_scan());
    }
}

pub fn show(ctx: &egui::Context, state: &mut UnitListState) {
    egui::SidePanel::left("unit_list_panel")
        .resizable(false)
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Unit Data Repository");
            ui.separator();
            state.unit_list.show(ctx, ui, &state.units, &mut state.selected_unit);
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical(|ui| {
            if let Some(id) = state.selected_unit {
                ui.heading(format!("Selected Unit: {:03}", id));
                ui.label("Selection logic to be implemented later.");
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a unit from the left list.");
                });
            }
        });
    });
}