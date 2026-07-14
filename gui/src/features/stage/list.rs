use eframe::egui;
use nyanko::chapter::Category;

use core::stage::logic::navigate;
use core::stage::registry::{GlobalMapId, GlobalStageId};

use super::category::CategoryExt;
use super::state::StageListState;

pub const BTN_SPACING_X: f32 = 14.0;
pub const BTN_SPACING_Y: f32 = 6.0;
pub const FILTER_BTN_PAD: f32 = 5.0;

pub fn draw(ui: &mut egui::Ui, state: &mut StageListState) {
    let categories = navigate::get_categories(&state.data.registry);

    if categories.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(egui::RichText::new("No Stages Found").strong().color(egui::Color32::LIGHT_RED));
        });
        return;
    }

    ui.vertical(|ui| {
        ui.add_space(FILTER_BTN_PAD);

        ui.horizontal(|ui| {
            ui.add_space(FILTER_BTN_PAD);
            let btn_w = ui.available_width() - (FILTER_BTN_PAD * 2.0);

            let mut filter_btn = egui::Button::new("Filter Stages");
            if state.filter_state.is_active() {
                filter_btn = filter_btn.fill(egui::Color32::from_rgb(31, 106, 165));
            }

            if ui.add_sized([btn_w, 28.0], filter_btn).clicked() {
                state.filter_state.is_open = !state.filter_state.is_open;
            }
        });

        ui.add_space(FILTER_BTN_PAD);
        ui.separator();

        let available_height = ui.available_height();

        ui.horizontal(|ui| {
            ui.set_min_height(available_height);
            ui.set_max_height(available_height);
            ui.spacing_mut().item_spacing.x = 0.0;

            draw_categories(ui, state, &categories);

            if state.data.selected_category.is_some() {
                ui.add(egui::Separator::default().vertical().spacing(BTN_SPACING_X));
                draw_maps(ui, state);

                if state.data.selected_map.is_some() {
                    ui.add(egui::Separator::default().vertical().spacing(BTN_SPACING_X));
                    draw_stages(ui, state);
                }
            }
        });
    });
}

fn has_matching_stage_in_map(state: &StageListState, cat: &Category, map_id: u32) -> bool {
    if !state.filter_state.is_active() {
        return true;
    }

    let map_key = GlobalMapId { category: cat.clone(), map: map_id };
    let stages = navigate::get_stages(&state.data.registry, &map_key);

    for stage in stages {
        if state.filter_state.matches_stage(&stage) {
            return true;
        }
    }

    false
}

fn has_matching_stage_in_category(state: &StageListState, cat: &Category) -> bool {
    if !state.filter_state.is_active() {
        return true;
    }

    let maps = navigate::get_maps(&state.data.registry, cat);
    for map in maps {
        if has_matching_stage_in_map(state, cat, map.map_id) {
            return true;
        }
    }

    false
}

fn draw_sidebar_btn(ui: &mut egui::Ui, text: &str, is_selected: bool) -> bool {
    let bg_color = if is_selected {
        egui::Color32::from_rgb(31, 106, 165)
    } else {
        egui::Color32::from_rgb(50, 50, 50)
    };

    let btn_text = egui::RichText::new(text).size(13.0);
    let btn = egui::Button::new(btn_text).fill(bg_color).wrap();

    ui.add_sized([ui.available_width(), 30.0], btn).clicked()
}

fn draw_categories(ui: &mut egui::Ui, state: &mut StageListState, categories: &[Category]) {
    ui.vertical(|ui| {
        ui.set_min_width(180.0);
        ui.set_max_width(180.0);
        ui.set_min_height(ui.available_height());

        egui::ScrollArea::vertical()
            .id_salt("cat_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = BTN_SPACING_Y;
                ui.add_space(BTN_SPACING_Y);

                let mut sorted_categories = categories.to_vec();
                sorted_categories.sort_by_key(|cat| cat.sort_order());

                for cat in &sorted_categories {
                    if !has_matching_stage_in_category(state, cat) {
                        continue;
                    }

                    let is_selected = state.data.selected_category.as_ref() == Some(cat);

                    if draw_sidebar_btn(ui, cat.display_name(), is_selected) {
                        state.data.selected_category = Some(cat.clone());
                        state.data.selected_map = None;
                        state.data.selected_stage = None;
                    }
                }

                ui.add_space(BTN_SPACING_Y);
            });
    });
}

fn draw_maps(ui: &mut egui::Ui, state: &mut StageListState) {
    let Some(cat) = state.data.selected_category.clone() else { return; };

    ui.vertical(|ui| {
        ui.set_min_width(200.0);
        ui.set_max_width(200.0);
        ui.set_min_height(ui.available_height());

        egui::ScrollArea::vertical()
            .id_salt("map_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = BTN_SPACING_Y;
                ui.add_space(BTN_SPACING_Y);

                let maps = navigate::get_maps(&state.data.registry, &cat);
                for map in maps {
                    if !has_matching_stage_in_map(state, &cat, map.map_id) {
                        continue;
                    }

                    let map_key = GlobalMapId { category: cat.clone(), map: map.map_id };
                    let is_selected = state.data.selected_map.as_ref() == Some(&map_key);

                    if draw_sidebar_btn(ui, &map.name, is_selected) {
                        state.data.selected_map = Some(map_key);
                        state.data.selected_stage = None;
                    }
                }

                ui.add_space(BTN_SPACING_Y);
            });
    });
}

fn draw_stages(ui: &mut egui::Ui, state: &mut StageListState) {
    let Some(map_id) = state.data.selected_map.clone() else { return; };

    ui.vertical(|ui| {
        ui.set_min_width(200.0);
        ui.set_max_width(200.0);
        ui.set_min_height(ui.available_height());

        egui::ScrollArea::vertical()
            .id_salt("stage_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = BTN_SPACING_Y;
                ui.add_space(BTN_SPACING_Y);

                let stages = navigate::get_stages(&state.data.registry, &map_id);
                for stage in stages {
                    if !state.filter_state.matches_stage(&stage) {
                        continue;
                    }

                    let stage_key = GlobalStageId {
                        category: map_id.category.clone(),
                        map: map_id.map,
                        stage: stage.stage_id,
                    };

                    let is_selected = state.data.selected_stage.as_ref() == Some(&stage_key);

                    if draw_sidebar_btn(ui, &stage.name, is_selected) {
                        state.data.selected_stage = Some(stage_key);
                    }
                }

                ui.add_space(BTN_SPACING_Y);
            });
    });
}