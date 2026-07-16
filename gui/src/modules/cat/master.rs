use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui;
use nyanko::cat::unit::TalentCost;
use nyanko::graphics::rig::Unit;

use core::modules::cat::logic::context::CatRenderContext;
use core::modules::cat::logic::scanner::CatEntry;
use core::modules::cat::logic::state::DetailTab;
use core::modules::cat::waiter::unitid;
use core::common::context::GlobalContext;
use core::modules::settings::logic::Settings;

use crate::modules::animation::viewer::AnimViewer;
use crate::modules::cat::statblock::build_cat_statblock;
use crate::modules::statblock::builder::{generate_and_copy, generate_and_save};
use crate::common::assets::CustomAssets;
use crate::common::shared::DragGuard;
use crate::common::sheet::SpriteSheet;

use super::{abilities, details, header, stats, talents, viewer};
use super::header::ExportAction;

pub fn show(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    cat_entry: &CatEntry,
    current_form: &mut usize,
    current_tab: &mut DetailTab,
    level_input: &mut String,
    current_level: &mut i32,
    texture_cache: &mut Option<egui::TextureHandle>,
    current_key: &mut String,
    img015_sheets: &mut Vec<SpriteSheet>,
    img022_sheets: &mut Vec<SpriteSheet>,
    unit_sync: &mut Option<Arc<Unit>>,
    anim_viewer: &mut AnimViewer,
    talent_name_cache: &mut HashMap<String, egui::TextureHandle>,
    gatya_item_textures: &mut HashMap<i32, Option<egui::TextureHandle>>,
    skill_descriptions: Option<&Vec<String>>,
    settings: &mut Settings,
    talent_levels: &mut HashMap<u8, u8>,
    talent_costs: &HashMap<u8, TalentCost>,
    cache_version: u64,
    global_ctx: GlobalContext,
    assets: &CustomAssets,
    drag_guard: &mut DragGuard,
) {
    crate::common::img015::ensure_loaded(ctx, img015_sheets, settings);
    crate::common::img022::ensure_loaded(ctx, img022_sheets, settings);

    let export_action = header::render(
        ctx, ui, cat_entry, current_form, current_tab, current_level, level_input, texture_cache, current_key, settings, talent_levels, talent_costs, img022_sheets
    );

    let dynamic_stats = unitid(cat_entry.id as i32, &settings.general.language_priority);
    let base_stats = dynamic_stats.as_ref().and_then(|v| v.get(*current_form));
    let form_allows_talents = *current_form >= 2;

    let final_stats_owned = base_stats.map(|base| core::modules::cat::logic::stats::get_final_stats(
        base,
        cat_entry.curve.as_ref(),
        *current_level,
        if form_allows_talents { cat_entry.talent_data.as_ref() } else { None },
        if form_allows_talents { Some(&*talent_levels) } else { None }
    ));

    match export_action {
        ExportAction::Copy | ExportAction::Save => {
            if let (Some(final_s), Some(base_s)) = (final_stats_owned.as_ref(), base_stats) {

                let expand_id = egui::Id::new(format!("conjure_expand_{}", cat_entry.id));
                let is_conjure_expanded = ctx.data(|d| d.get_temp::<bool>(expand_id).unwrap_or(settings.cat_data.expand_spirit_details));

                let cat_ctx = CatRenderContext {
                    global: global_ctx,
                    base_stats: base_s,
                    final_stats: final_s,
                    current_level: *current_level,
                    level_curve: cat_entry.curve.as_ref(),
                    talent_data: if form_allows_talents { cat_entry.talent_data.as_ref() } else { None },
                    talent_levels: if form_allows_talents { Some(&*talent_levels) } else { None },
                    is_conjure_unit: false,
                };

                let data = build_cat_statblock(
                    &cat_ctx,
                    cat_entry,
                    *current_form,
                    level_input.clone(),
                    is_conjure_expanded,
                    settings
                );

                let priority_clone = settings.general.language_priority.clone();
                let mut cuts_clone = std::collections::HashMap::new();
                for sheet in img015_sheets.iter().rev() {
                    cuts_clone.extend(sheet.core.cuts_map.clone());
                }

                if export_action == ExportAction::Copy {
                    generate_and_copy(ctx.clone(), priority_clone, data, cuts_clone);
                } else {
                    generate_and_save(ctx.clone(), priority_clone, data, cuts_clone);
                }
            }
        },
        ExportAction::None => {}
    }

    ui.separator();
    ui.add_space(0.0);

    match current_tab {
        DetailTab::Abilities => {
            if let (Some(final_s), Some(base_s)) = (final_stats_owned.as_ref(), base_stats) {

                let cat_ctx = CatRenderContext {
                    global: global_ctx,
                    base_stats: base_s,
                    final_stats: final_s,
                    current_level: *current_level,
                    level_curve: cat_entry.curve.as_ref(),
                    talent_data: if form_allows_talents { cat_entry.talent_data.as_ref() } else { None },
                    talent_levels: if form_allows_talents { Some(&*talent_levels) } else { None },
                    is_conjure_unit: false,
                };

                stats::render(ui, cat_entry, final_s, *current_form);
                ui.spacing_mut().item_spacing.y = 7.0;
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        abilities::render(
                            ui, &cat_ctx, cat_entry, img015_sheets, assets, settings
                        );
                    });
            }
        },
        DetailTab::Talents => {
            if let Some(raw) = &cat_entry.talent_data {
                talents::render(ui, raw, img015_sheets, img022_sheets, talent_name_cache, skill_descriptions, settings, base_stats, cat_entry.curve.as_ref(), *current_level, talent_levels, cat_entry.id, talent_costs, assets);
            }
        },
        DetailTab::Details => {
            let fallback = Vec::new();
            let desc = cat_entry.description[*current_form].as_ref().unwrap_or(&fallback);
            details::render(ui, desc);
            let text_fallback = Vec::new();
            let ev_text = cat_entry.evolve_text.texts[*current_form].as_ref().unwrap_or(&text_fallback);
            details::render_evolve(
                ui,
                ctx,
                &cat_entry.unitbuy,
                ev_text,
                *current_form,
                gatya_item_textures,
                cache_version,
                &settings.general.language_priority
            );
        }
        DetailTab::Animation => {
            viewer::show(ui, ctx, cat_entry, *current_form, anim_viewer, unit_sync, settings, drag_guard);
        }
    }
}