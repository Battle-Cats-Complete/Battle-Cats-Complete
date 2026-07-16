use eframe::egui;

use core::settings::logic::Settings;

use super::{details, import, state::ModListState};

pub fn show(ctx: &egui::Context, state: &mut ModListState, settings: &mut Settings) {
    let mut list = state.list.take().unwrap_or_default();

    egui::SidePanel::left("mod_list_panel")
        .resizable(false)
        .exact_width(160.0) 
        .show(ctx, |ui| {
            list.render(ui, &mut state.data, settings);
        });

    state.list = Some(list);

    egui::CentralPanel::default().show(ctx, |ui| {
        details::render(ui, state, settings);
    });

    if state.data.import.is_open {
        import::show(ctx, state, settings);
    }
}