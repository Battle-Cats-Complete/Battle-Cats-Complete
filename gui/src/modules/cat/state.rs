use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui;
use nyanko::graphics::rig::Unit;
use serde::{Deserialize, Serialize};

use core::modules::cat::filter::CatFilterState;
use core::modules::cat::{CatDataState, DetailTab};
use core::common::context::GlobalContext;
use core::modules::settings::Settings;

use crate::modules::animation::viewer::AnimViewer;
use crate::common::CustomAssets;
use crate::common::DragGuard;
use crate::common::SpriteSheet;

use super::list::CatList;

pub(crate) const TOP_PANEL_PADDING: f32 = 2.5;
pub(crate) const SEARCH_FILTER_GAP: f32 = 5.0;
pub(crate) const SPACE_BEFORE_SEPARATOR: f32 = 2.0;
pub(crate) const SPACE_AFTER_SEPARATOR: f32 = 2.0;

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub(crate) struct CatListState {
    pub data: CatDataState,

    #[serde(skip)] pub cat_list: CatList,
    #[serde(skip)] pub anim_viewer: AnimViewer,
    #[serde(skip)] pub filter_state: CatFilterState,
    #[serde(skip)] pub drag_guard: DragGuard,
    #[serde(skip)] pub custom_assets: Option<CustomAssets>,

    #[serde(skip)] pub detail_texture: Option<egui::TextureHandle>,
    #[serde(skip)] pub img015_sheets: Vec<SpriteSheet>,
    #[serde(skip)] pub img022_sheets: Vec<SpriteSheet>,
    #[serde(skip)] pub talent_name_textures: HashMap<String, egui::TextureHandle>,
    #[serde(skip)] pub gatya_item_textures: HashMap<i32, Option<egui::TextureHandle>>,
    #[serde(skip)] pub texture_cache_version: u64,
    #[serde(skip)] pub rig: Option<Arc<Unit>>,
}

pub(crate) fn show(ctx: &egui::Context, state: &mut CatListState, settings: &mut Settings, global_ctx: GlobalContext, drag_guard: &mut DragGuard) {
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

    egui::SidePanel::left("cat_list_panel")
        .resizable(false)
        .default_width(160.0)
        .show(ctx, |ui| {
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(TOP_PANEL_PADDING);
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let search_response = ui.add(egui::TextEdit::singleline(&mut state.data.search_query)
                        .hint_text(egui::RichText::new("Search Cat...").color(egui::Color32::GRAY))
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

            let old_selection_id = state.data.selected_cat;
            if !state.data.cats.is_empty() {
                state.cat_list.show(
                    ctx, ui, &state.data.cats, &mut state.data.selected_cat,
                    &state.data.search_query, &state.filter_state,
                    settings.cat_data.high_banner_quality
                );
            }

            if state.data.selected_cat != old_selection_id {
                state.detail_texture = None;
                state.data.detail_key.clear();
                state.rig = None;
                state.data.saved_pre_ultra_level = None;
                state.data.is_in_ultra_state = false;

                if let Some(new_id) = state.data.selected_cat {
                    if let Some(pos) = state.data.talent_history.iter().position(|&id| id == new_id) {
                        state.data.talent_history.remove(pos);
                    }
                    state.data.talent_history.push_back(new_id);
                    while state.data.talent_history.len() > 3 {
                        if let Some(popped_id) = state.data.talent_history.pop_front() {
                            state.data.talent_levels.remove(&popped_id);
                        }
                    }

                    if let Some(new_cat) = state.data.cats.iter().find(|c| c.id == new_id) {
                        let mut max_form_index = 0;
                        for (i, &exists) in new_cat.forms.iter().enumerate() {
                            if exists { max_form_index = i; }
                        }
                        if state.data.selected_form > max_form_index || !new_cat.forms[state.data.selected_form] {
                            state.data.selected_form = max_form_index;
                        }
                        if state.data.selected_detail_tab == DetailTab::Talents {
                            let form_valid = state.data.selected_form >= 2;
                            let has_data = new_cat.talent_data.is_some();
                            if !form_valid || !has_data {
                                state.data.selected_detail_tab = DetailTab::Abilities;
                            }
                        }

                        if settings.cat_data.auto_level_calculations {
                            let base_max = new_cat.unitbuy.level_cap_catseye;
                            let plus_max = new_cat.unitbuy.level_cap_plus;
                            let is_legend_rare = new_cat.unitbuy.rarity == 5;
                            let is_normal_rare = new_cat.unitbuy.rarity == 0;

                            if is_legend_rare {
                                state.data.current_level = 50;
                                state.data.level_input = "50".to_string();
                            } else if base_max == 1 || (5..=65).contains(&plus_max) || is_normal_rare {
                                state.data.current_level = base_max + plus_max;
                                if plus_max > 0 {
                                    state.data.level_input = format!("{}+{}", base_max, plus_max);
                                } else {
                                    state.data.level_input = base_max.to_string();
                                }
                            } else if base_max > 50 {
                                state.data.current_level = 50;
                                state.data.level_input = "50".to_string();
                            } else {
                                state.data.current_level = base_max;
                                state.data.level_input = base_max.to_string();
                            }
                        } else {
                            state.data.current_level = settings.cat_data.default_level;
                            state.data.level_input = settings.cat_data.default_level.to_string();
                        }
                    }
                }
            }
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        if state.data.cats.is_empty() {
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
                        ui.label(egui::RichText::new("Could not find any units in game/cats").color(ui.visuals().weak_text_color()));
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

        let Some(selected_id) = state.data.selected_cat else {
            ui.centered_and_justified(|ui| { ui.label("Select a Unit"); });
            return;
        };

        let Some(cat_entry) = state.data.cats.iter().find(|cat| cat.id == selected_id) else {
            ui.centered_and_justified(|ui| { ui.spinner(); });
            return;
        };

        let talent_map = state.data.talent_levels.entry(selected_id).or_default();
        let prev_form = state.data.selected_form;

        crate::modules::cat::master::show(
            ctx, ui, cat_entry,
            &mut state.data.selected_form, &mut state.data.selected_detail_tab,
            &mut state.data.level_input, &mut state.data.current_level,
            &mut state.detail_texture, &mut state.data.detail_key,
            &mut state.img015_sheets, &mut state.img022_sheets,
            &mut state.rig,
            &mut state.anim_viewer, &mut state.talent_name_textures,
            &mut state.gatya_item_textures, Some(cat_entry.skill_descriptions.as_ref()),
            settings, talent_map, cat_entry.talent_costs.as_ref(),
            state.texture_cache_version, global_ctx, &assets, drag_guard
        );

        let mut current_ultra_state = state.data.selected_form == 3;
        if state.data.selected_form >= 2
            && let Some(levels) = state.data.talent_levels.get(&selected_id) {
            if let Some(t_data) = &cat_entry.talent_data {
                for (idx, group) in t_data.groups.iter().enumerate() {
                    if group.limit == 1
                        && let Some(&lvl) = levels.get(&(idx as u8))
                        && lvl > 0 { current_ultra_state = true; break; }
                }
            } else if levels.iter().any(|(&idx, &lvl)| idx >= 5 && lvl > 0) {
                current_ultra_state = true;
            }
        }

        if settings.cat_data.bump_ultra_60 {
            if !state.data.is_in_ultra_state && current_ultra_state {
                state.data.saved_pre_ultra_level = Some((state.data.current_level, state.data.level_input.clone()));
                if state.data.current_level < 60 {
                    state.data.current_level = 60;
                    state.data.level_input = "60".to_string();
                }
            } else if state.data.is_in_ultra_state && !current_ultra_state
                && let Some((saved_lvl, saved_str)) = state.data.saved_pre_ultra_level.take() {
                let expected_ultra_level = if saved_lvl < 60 { 60 } else { saved_lvl };
                if state.data.current_level == expected_ultra_level {
                    state.data.current_level = saved_lvl;
                    state.data.level_input = saved_str;
                }
            }
            state.data.is_in_ultra_state = current_ultra_state;
        } else {
            state.data.is_in_ultra_state = current_ultra_state;
            state.data.saved_pre_ultra_level = None;
        }

        if state.data.selected_form != prev_form {
            state.rig = None;
        }
    });

    crate::modules::cat::filter::show_popup(
        ctx, &mut state.filter_state, &mut state.img015_sheets,
        &assets,
        settings, &mut state.drag_guard,
    );
}