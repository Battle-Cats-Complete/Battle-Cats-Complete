use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use eframe::egui;
use nyanko::chapter::{Category, Map};

use core::modules::stage::logic::filter::CompiledStageFilter;
use core::modules::stage::logic::navigate;
use core::modules::stage::registry::{GlobalMapId, GlobalStageId};

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

    let mut hasher = DefaultHasher::new();
    state.filter_state.hash(&mut hasher);
    let current_hash = hasher.finish();

    if state.compiled_filter.as_ref().map_or(true, |cf| cf.source_hash != current_hash) {
        let mut new_compiled = state.filter_state.compile();
        new_compiled.source_hash = current_hash;
        state.compiled_filter = Some(new_compiled);
    }

    let Some(compiled_filter) = state.compiled_filter.take() else { return; };

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

            draw_categories(ui, state, &compiled_filter, &categories);

            if state.data.selected_category.is_some() {
                ui.add(egui::Separator::default().vertical().spacing(BTN_SPACING_X));
                draw_maps(ui, state, &compiled_filter);

                if state.data.selected_map.is_some() {
                    ui.add(egui::Separator::default().vertical().spacing(BTN_SPACING_X));
                    draw_stages(ui, state, &compiled_filter);
                }
            }
        });
    });

    state.compiled_filter = Some(compiled_filter);
}

fn has_matching_stage_in_map(state: &StageListState, compiled_filter: &CompiledStageFilter, cat: &Category, map_key: &GlobalMapId, map: &Map) -> bool {
    if !compiled_filter.is_active() { return true; }

    let stages = navigate::get_stages(&state.data.registry, map_key);
    let cat_name = cat.display_name();

    for stage in stages {
        if compiled_filter.matches(
            cat_name,
            map,
            &stage,
            &state.data.enemy_name_registry,
            &state.data.lock_skip_registry,
            &state.data.scat_cpu_setting,
            &state.data.item_buy_registry,
            &state.data.item_name_registry,
            &state.data.drop_chara_registry,
            &state.data.unit_buy_registry,
            &state.data.cat_name_registry,
        ) {
            return true;
        }
    }

    false
}

fn has_matching_stage_in_category(state: &StageListState, compiled_filter: &CompiledStageFilter, cat: &Category) -> bool {
    if !compiled_filter.is_active() { return true; }

    let maps = navigate::get_maps(&state.data.registry, cat);
    for map in maps {
        let map_key = GlobalMapId { category: cat.clone(), map: map.map_id };
        if has_matching_stage_in_map(state, compiled_filter, cat, &map_key, &map) {
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

fn draw_categories(ui: &mut egui::Ui, state: &mut StageListState, compiled_filter: &CompiledStageFilter, categories: &[Category]) {
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
                    if !has_matching_stage_in_category(state, compiled_filter, cat) {
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

fn draw_maps(ui: &mut egui::Ui, state: &mut StageListState, compiled_filter: &CompiledStageFilter) {
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
                    let map_key = GlobalMapId { category: cat.clone(), map: map.map_id };

                    if !has_matching_stage_in_map(state, compiled_filter, &cat, &map_key, &map) {
                        continue;
                    }

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

fn draw_stages(ui: &mut egui::Ui, state: &mut StageListState, compiled_filter: &CompiledStageFilter) {
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

                let maps = navigate::get_maps(&state.data.registry, &map_id.category);
                let Some(map) = maps.into_iter().find(|m| m.map_id == map_id.map) else { return; };
                let cat_name = map_id.category.display_name();

                let stages = navigate::get_stages(&state.data.registry, &map_id);
                for stage in stages {
                    if !compiled_filter.matches(
                        cat_name,
                        &map,
                        &stage,
                        &state.data.enemy_name_registry,
                        &state.data.lock_skip_registry,
                        &state.data.scat_cpu_setting,
                        &state.data.item_buy_registry,
                        &state.data.item_name_registry,
                        &state.data.drop_chara_registry,
                        &state.data.unit_buy_registry,
                        &state.data.cat_name_registry,
                    ) {
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