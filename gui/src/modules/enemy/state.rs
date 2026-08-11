use std::sync::Arc;

use eframe::egui;
use nyanko::graphics::rig::Unit;
use serde::{Deserialize, Serialize};

use core::modules::enemy::filter::EnemyFilterState;
use core::modules::enemy::EnemyDataState;
use core::common::context::GlobalContext;
use core::modules::settings::Settings;

use crate::modules::animation::AnimViewer;
use crate::common::CustomAssets;
use crate::common::DragGuard;
use crate::common::SpriteSheet;

use super::list::EnemyList;
use super::master;

pub(crate) const TOP_PANEL_PADDING: f32 = 2.5;
pub(crate) const SEARCH_FILTER_GAP: f32 = 5.0;
pub(crate) const SPACE_BEFORE_SEPARATOR: f32 = 2.0;
pub(crate) const SPACE_AFTER_SEPARATOR: f32 = 2.0;

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub(crate) struct EnemyListState {
    pub data: EnemyDataState,

    #[serde(skip)] pub enemy_list: EnemyList,
    #[serde(skip)] pub anim_viewer: AnimViewer,
    #[serde(skip)] pub filter_state: EnemyFilterState,
    #[serde(skip)] pub drag_guard: DragGuard,
    #[serde(skip)] pub custom_assets: Option<CustomAssets>,

    #[serde(skip)] pub detail_texture: Option<egui::TextureHandle>,
    #[serde(skip)] pub img015_sheets: Vec<SpriteSheet>,

    #[serde(skip)] pub rig: Option<Arc<Unit>>,
}

pub(crate) fn show(ctx: &egui::Context, state: &mut EnemyListState, settings: &mut Settings, global_ctx: GlobalContext, drag_guard: &mut DragGuard) {
    if state.custom_assets.is_none() {
        state.custom_assets = Some(CustomAssets::new(ctx));
    }
    let assets = state.custom_assets.as_ref().unwrap().clone();

    if !state.data.initialized {
        state.data.initialized = true;
    }

    if state.data.scan_receiver.is_some() {
        state.data.update_data();
        ctx.request_repaint();
    }

    let old_selection_id = state.data.selected_enemy;

    egui::SidePanel::left("enemy_list_panel")
        .resizable(false)
        .default_width(160.0)
        .show(ctx, |ui| {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(TOP_PANEL_PADDING);

                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let search_response = ui.add(egui::TextEdit::singleline(&mut state.data.search_query)
                        .hint_text(egui::RichText::new("Search Enemy...").color(egui::Color32::GRAY))
                        .desired_width(140.0));

                    ui.add_space(SEARCH_FILTER_GAP);

                    let btn_size = egui::vec2(140.0, search_response.rect.height());
                    let filter_active = state.filter_state.is_active();

                    let mut filter_btn = egui::Button::new("Filter");
                    if filter_active {
                        filter_btn = filter_btn.fill(egui::Color32::from_rgb(31, 106, 165));
                    }
                    if ui.add_sized(btn_size, filter_btn).clicked() {
                        state.filter_state.is_open = !state.filter_state.is_open;
                    }
                });

                ui.add_space(SPACE_BEFORE_SEPARATOR);
                ui.separator();
                ui.add_space(SPACE_AFTER_SEPARATOR);
            });

            if !state.data.enemies.is_empty() {
                state.enemy_list.show(
                    ctx,
                    ui,
                    &state.data.enemies,
                    &mut state.data.selected_enemy,
                    &state.data.search_query,
                    &state.filter_state
                );
            }
        });

    if state.data.selected_enemy != old_selection_id {
        state.detail_texture = None;
        state.data.detail_key.clear();
        state.rig = None;
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        if state.data.enemies.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.4);
                    ui.set_max_width(400.0);
                    if state.data.scan_receiver.is_some() {
                        ui.spinner();
                        ui.add_space(10.0);
                        ui.label("Loading Unit Data...");
                    } else {
                        ui.heading("No Data Found");
                        ui.label(egui::RichText::new("Could not find any units in game/enemies").color(ui.visuals().weak_text_color()));
                        ui.add_space(5.0);
                        if ui.button("Retry Scan").clicked() {
                            state.data.restart_scan(settings.scanner_config());
                            ui.ctx().request_repaint();
                        }
                    }
                });
            });
            return;
        }

        let Some(selected_id) = state.data.selected_enemy else {
            ui.centered_and_justified(|ui| { ui.label("Select an Enemy"); });
            return;
        };

        let Some(enemy_entry) = state.data.enemies.iter().find(|e| e.id == selected_id) else {
            ui.centered_and_justified(|ui| { ui.spinner(); });
            return;
        };

        master::show(
            ctx, ui, enemy_entry,
            &mut state.data.selected_tab, &mut state.data.mag_input, &mut state.data.magnification,
            &mut state.img015_sheets,
            &mut state.rig,
            &mut state.anim_viewer, settings,
            &mut state.detail_texture, &mut state.data.detail_key, global_ctx, &assets, drag_guard
        );
    });

    crate::modules::enemy::filter::show_popup(
        ctx, &mut state.filter_state, &mut state.img015_sheets,
        &assets, settings, &mut state.drag_guard,
    );
}