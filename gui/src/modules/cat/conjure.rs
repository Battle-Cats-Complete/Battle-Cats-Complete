use eframe::egui;
use nyanko::common::data::img015;

use core::modules::cat::game::abilities;
use core::modules::cat::game::CatRenderContext;
use core::modules::cat::game::stats;
use core::modules::cat::waiter::unitid;
use core::common::game::abilities::ABILITY_Y;
use core::modules::settings::Settings;

use crate::modules::statblock::builder::SpiritData;
use crate::common::shared::{render_fallback_icon, text_with_superscript, ICON_SIZE};
use crate::common::sheet::SpriteSheet;

pub(crate) fn render_conjure_toggle(ui: &mut egui::Ui, text: &str, id: egui::Id, settings: &Settings) {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 7.0;
        let mut expanded = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(settings.cat_data.expand_spirit_details));
        text_with_superscript(ui, text);
        let btn_text = egui::RichText::new("Details").size(11.0);
        let btn = if expanded {
            egui::Button::new(btn_text.color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(0, 100, 200))
        } else {
            egui::Button::new(btn_text)
        };

        if ui.add(btn).clicked() {
            expanded = !expanded;
            ui.data_mut(|d| d.insert_temp(id, expanded));
        }
    });
}

pub(crate) fn render_conjure_details(
    ui: &mut egui::Ui,
    ctx: &CatRenderContext,
    sheets: &[SpriteSheet],
    assets: &crate::common::assets::CustomAssets,
    settings: &Settings
) {
    egui::Frame::none()
        .fill(egui::Color32::from_black_alpha(220)) 
        .rounding(egui::Rounding { nw: 0.0, ne: 0.0, sw: 8.0, se: 8.0 }) 
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let spirit_border = egui::Color32::WHITE;
            
            let conjure_stats_vec = match unitid(ctx.base_stats.conjure_unit_id, &settings.general.language_priority) {
                Some(s) => s,
                None => {
                    ui.label(egui::RichText::new("Spirit data not found").weak());
                    return;
                }
            };

            let conjure_stats = match conjure_stats_vec.first() {
                Some(s) => s,
                None => return,
            };

            let conjure_final = stats::get_final_stats(
                conjure_stats, ctx.level_curve, ctx.current_level, None, None
            );

            let spirit_ctx = CatRenderContext {
                global: ctx.global,
                base_stats: conjure_stats,
                final_stats: &conjure_final,
                current_level: ctx.current_level,
                level_curve: ctx.level_curve,
                talent_data: None,
                talent_levels: None,
                is_conjure_unit: true,
            };

            let dmg = conjure_final.attack_1;
            
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let icon = img015::ICON_AREA_ATTACK;
                let size = egui::vec2(ICON_SIZE, ICON_SIZE);
                
                let mut drawn = false;
                for sheet in sheets {
                    if let Some(cut) = sheet.core.cuts_map.get(&icon)
                        && let Some(tex) = &sheet.texture_handle {
                             ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(egui::Rect::from_min_max(egui::pos2(cut.uv_coordinates.min.x, cut.uv_coordinates.min.y), egui::pos2(cut.uv_coordinates.max.x, cut.uv_coordinates.max.y))));
                             drawn = true;
                             break;
                        }
                }
                if !drawn {
                    let alt = core::modules::cat::game::registry::get_fallback_by_icon(core::modules::cat::game::registry::AbilityIcon::Standard(icon));
                    render_fallback_icon(ui, alt, spirit_border);
                }
                ui.label(format!("Damage {}\nRange {}", dmg, conjure_final.standing_range));
            });
            
            ui.add_space(ABILITY_Y);

            let (spirit_traits, spirit_head_1, spirit_head_2, spirit_body_1, spirit_body_2, spirit_footer) = abilities::collect_ability_data(&spirit_ctx);
            
            let mut prev = false;
            let mut last_was_trait = false;

            if !spirit_traits.is_empty() { 
                crate::modules::cat::abilities::render_icon_row(ui, &spirit_traits, sheets, spirit_border, assets);
                prev = true;
                last_was_trait = true;
            }

            if !spirit_head_1.is_empty() {
                if prev { ui.add_space(if last_was_trait { core::common::game::abilities::TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
                crate::modules::cat::abilities::render_icon_row(ui, &spirit_head_1, sheets, spirit_border, assets);
                prev = true;
            }

            if !spirit_head_2.is_empty() {
                if prev { ui.add_space(if last_was_trait { core::common::game::abilities::TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
                crate::modules::cat::abilities::render_icon_row(ui, &spirit_head_2, sheets, spirit_border, assets);
                prev = true;
            }
            
            let has_body = !spirit_body_1.is_empty() || !spirit_body_2.is_empty();
            if has_body {
                if prev { ui.add_space(if last_was_trait { core::common::game::abilities::TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
                crate::modules::cat::abilities::render_list_view(ui, &spirit_body_1, sheets, 0, &spirit_ctx, spirit_border, assets, settings);
                if !spirit_body_1.is_empty() && !spirit_body_2.is_empty() { ui.add_space(ABILITY_Y); }
                crate::modules::cat::abilities::render_list_view(ui, &spirit_body_2, sheets, 0, &spirit_ctx, spirit_border, assets, settings);
                prev = true;
            }
            
            if !spirit_footer.is_empty() {
                if prev { ui.add_space(if last_was_trait { core::common::game::abilities::TRAIT_Y } else { ABILITY_Y }); }
                crate::modules::cat::abilities::render_icon_row(ui, &spirit_footer, sheets, spirit_border, assets);
            }
        });
}

pub(crate) fn build_spirit_data(
    ctx: &CatRenderContext,
    settings: &Settings
) -> Option<SpiritData> {
    if ctx.base_stats.conjure_unit_id > 0
        && let Some(c_vec) = unitid(ctx.base_stats.conjure_unit_id, &settings.general.language_priority)
            && let Some(c_stats) = c_vec.first() {
                let conjure_final = stats::get_final_stats(c_stats, ctx.level_curve, ctx.current_level, None, None);

                let spirit_ctx = CatRenderContext {
                    global: ctx.global,
                    base_stats: c_stats,
                    final_stats: &conjure_final,
                    current_level: ctx.current_level,
                    level_curve: ctx.level_curve,
                    talent_data: None,
                    talent_levels: None,
                    is_conjure_unit: true,
                };

                let (s_traits, s_h1, s_h2, s_b1, s_b2, s_footer) = abilities::collect_ability_data(&spirit_ctx);

                return Some(SpiritData {
                    dmg_text: format!("Damage {}\nRange {}", conjure_final.attack_1, conjure_final.standing_range),
                    traits: s_traits,
                    h1: s_h1,
                    h2: s_h2,
                    b1: s_b1,
                    b2: s_b2,
                    footer: s_footer,
                });
            }
    None
}