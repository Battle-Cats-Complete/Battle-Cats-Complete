use eframe::egui;
use tracing::warn;

use core::global::context::GlobalContext;

use super::state::StageListState;

pub fn draw(ctx: &egui::Context, ui: &mut egui::Ui, state: &mut StageListState, global_ctx: GlobalContext) {
    let Some(stage_id) = &state.data.selected_stage else {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("Select a stage to view details").color(egui::Color32::DARK_GRAY));
        });
        return;
    };

    let item_buy_registry = &state.data.item_buy_registry;
    let item_name_registry = &state.data.item_name_registry;
    let drop_chara_registry = &state.data.drop_chara_registry;
    let unit_buy_registry = &state.data.unit_buy_registry;
    let item_texture_cache = &mut state.item_texture_cache;
    let active_language_priority_array = &state.data.active_language_priority;

    let enemy_registry = &state.data.enemy_registry;
    let enemy_name_registry = &state.data.enemy_name_registry;
    let texture_cache = &mut state.enemy_texture_cache;
    let stage_texture_cache = &mut state.stage_texture_cache;

    let Some(stage) = state.data.registry.stages.get(stage_id) else { return; };

    let map_key = format!("{}_{}", stage.category, stage.map_id);
    let Some(map_data) = state.data.registry.maps.get(&map_key) else {
        warn!(map_key, "Failed to locate parent map for stage view");
        return;
    };

    egui::ScrollArea::vertical()
        .id_salt("view_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let frame = egui::Frame::none().inner_margin(egui::Margin { left: 40.0, right: 40.0, top: 0.0, bottom: 0.0 });

            frame.show(ui, |ui| {
                ui.vertical(|ui| {
                    super::info::draw(
                        ctx,
                        ui,
                        stage,
                        &map_data.name,
                        active_language_priority_array,
                        stage_texture_cache,
                        &state.data.lock_skip_registry,
                        &state.data.scat_cpu_setting
                    );

                    ui.add_space(20.0);

                    if super::materials::has_drops(stage, map_data) {
                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                super::treasure::draw(
                                    ctx,
                                    ui,
                                    stage,
                                    item_buy_registry,
                                    item_name_registry,
                                    drop_chara_registry,
                                    unit_buy_registry,
                                    item_texture_cache,
                                    active_language_priority_array
                                );
                            });

                            ui.add_space(24.0);

                            ui.vertical(|ui| {
                                super::materials::draw(
                                    ctx,
                                    ui,
                                    stage,
                                    map_data,
                                    item_buy_registry,
                                    item_name_registry,
                                    item_texture_cache,
                                    active_language_priority_array
                                );
                            });
                        });
                    } else {
                        super::treasure::draw(
                            ctx,
                            ui,
                            stage,
                            item_buy_registry,
                            item_name_registry,
                            drop_chara_registry,
                            unit_buy_registry,
                            item_texture_cache,
                            active_language_priority_array
                        );
                    }

                    ui.add_space(20.0);

                    super::battleground::draw(
                        ctx,
                        ui,
                        stage,
                        map_data,
                        enemy_registry,
                        enemy_name_registry,
                        texture_cache,
                        global_ctx
                    );
                });
            });
        });
}