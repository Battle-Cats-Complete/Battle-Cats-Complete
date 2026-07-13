use eframe::egui;
use tracing::{trace, warn};

use core::global::context::GlobalContext;
use core::stage::registry::GlobalMapId;
use super::state::StageListState;

pub fn draw(ctx: &egui::Context, ui: &mut egui::Ui, state: &mut StageListState, global_ctx: GlobalContext) {
    let Some(stage_id) = &state.data.selected_stage else {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("Select a stage to view details").color(egui::Color32::DARK_GRAY));
        });
        return;
    };

    let item_buys = &state.data.item_buy_registry;
    let item_names = &state.data.item_name_registry;
    let drop_charas = &state.data.drop_chara_registry;
    let unit_buys = &state.data.unit_buy_registry;
    let item_textures = &mut state.item_texture_cache;
    let langs = &state.data.active_language_priority;

    let enemies = &state.data.enemy_registry;
    let enemy_names = &state.data.enemy_name_registry;
    let enemy_textures = &mut state.enemy_texture_cache;
    let stage_textures = &mut state.stage_texture_cache;
    let cat_textures = &mut state.cat_texture_cache;

    let Some(stage) = state.data.registry.stages.get(stage_id) else { return; };

    let map_key = GlobalMapId { category: stage.category.clone(), map: stage.map_id };
    let Some(map_data) = state.data.registry.maps.get(&map_key) else {
        warn!(?map_key, "Failed to locate parent map for stage view");
        return;
    };

    if state.selected_crown >= stage.max_crowns {
        trace!("Resetting selected crown out of bounds");
        state.selected_crown = 0;
    }

    egui::ScrollArea::vertical()
        .id_salt("view_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let margin = egui::Margin { left: 40.0, right: 40.0, top: 0.0, bottom: 0.0 };

            egui::Frame::none().inner_margin(margin).show(ui, |ui| {
                ui.vertical(|ui| {
                    super::info::draw(
                        ctx,
                        ui,
                        stage,
                        map_data,
                        langs,
                        stage_textures,
                        &state.data.lock_skip_registry,
                        &state.data.scat_cpu_setting,
                        &mut state.selected_crown
                    );

                    ui.add_space(20.0);

                    if super::materials::has_drops(stage, map_data) {
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                ui.set_max_width(345.0);
                                super::materials::draw(
                                    ctx,
                                    ui,
                                    stage,
                                    map_data,
                                    state.selected_crown,
                                    item_buys,
                                    item_names,
                                    item_textures,
                                    langs
                                );
                            });

                            ui.add_space(15.0);

                            ui.vertical(|ui| {
                                super::treasure::draw(
                                    ctx,
                                    ui,
                                    stage,
                                    item_buys,
                                    item_names,
                                    drop_charas,
                                    unit_buys,
                                    item_textures,
                                    langs
                                );
                            });
                        });
                    } else {
                        super::treasure::draw(
                            ctx,
                            ui,
                            stage,
                            item_buys,
                            item_names,
                            drop_charas,
                            unit_buys,
                            item_textures,
                            langs
                        );
                    }

                    ui.add_space(20.0);

                    if let Some(preset) = stage.fixed_lineups.get(&state.selected_crown) {
                        let resolved = core::stage::logic::fixedlineup::resolve_lineup(preset, langs);

                        super::fixedlineup::draw(
                            ctx,
                            ui,
                            &resolved,
                            preset,
                            cat_textures,
                            langs
                        );

                        ui.add_space(20.0);
                    }

                    super::battleground::draw(
                        ctx,
                        ui,
                        stage,
                        map_data,
                        state.selected_crown,
                        enemies,
                        enemy_names,
                        enemy_textures,
                        global_ctx
                    );
                });
            });
        });
}