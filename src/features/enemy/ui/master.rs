use eframe::egui;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::enemy::logic::state::EnemyDetailTab;
use crate::features::settings::logic::Settings;
use crate::global::imgcut::SpriteSheet;
use crate::global::img015;
use crate::global::mamodel::Model;
use crate::features::animation::ui::viewer::AnimViewer;
use crate::global::assets::CustomAssets; // Import CustomAssets

use crate::features::statblock::logic::builder::{StatblockData, generate_and_copy, generate_and_save};
use crate::global::abilities::{AbilityItem, CustomIcon};

use super::{header, stats, abilities, details, viewer}; 
use super::header::ExportAction;

fn map_enemy_abilities(items: Vec<crate::features::enemy::logic::abilities::EnemyAbilityItem>) -> Vec<AbilityItem> {
    items.into_iter().map(|item| AbilityItem {
        icon_id: item.icon_id,
        text: item.text,
        custom_icon: match item.custom_icon {
            crate::features::enemy::logic::abilities::EnemyCustomIcon::None => CustomIcon::None,
            crate::features::enemy::logic::abilities::EnemyCustomIcon::Multihit => CustomIcon::Multihit,
            crate::features::enemy::logic::abilities::EnemyCustomIcon::Kamikaze => CustomIcon::Kamikaze,
        },
        border_id: item.border_id,
    }).collect()
}

pub fn show(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    enemy_entry: &EnemyEntry, 
    current_tab: &mut EnemyDetailTab, 
    mag_input: &mut String,
    magnification: &mut i32,
    settings: &mut Settings,
    icon_sheet: &mut SpriteSheet,
    anim_sheet: &mut SpriteSheet,
    model_data: &mut Option<Model>,
    anim_viewer: &mut AnimViewer,
    
    // REFACTORED: One struct replaces 6 textures
    assets: &CustomAssets, 
    
    detail_texture: &mut Option<egui::TextureHandle>,
    detail_key: &mut String,
) {
    img015::ensure_loaded(ctx, icon_sheet, settings);

    let export_action = header::render(
        ctx,
        ui,
        enemy_entry,
        current_tab,
        mag_input,
        magnification,
        detail_texture,
        detail_key,
    );

    // --- TRIGGER EXPORT ---
    match export_action {
        ExportAction::Copy | ExportAction::Save => {
            
            let (e_traits, e_h1, e_h2, e_b1, e_b2, e_footer) = crate::features::enemy::logic::abilities::collect_ability_data(
                &enemy_entry.stats, settings, *magnification
            );

            let traits = map_enemy_abilities(e_traits);
            let h1 = map_enemy_abilities(e_h1);
            let h2 = map_enemy_abilities(e_h2);
            let b1 = map_enemy_abilities(e_b1);
            let b2 = map_enemy_abilities(e_b2);
            let footer = map_enemy_abilities(e_footer);

            let total_atk = enemy_entry.stats.attack_1 + enemy_entry.stats.attack_2 + enemy_entry.stats.attack_3;
            let mag_f = *magnification as f32 / 100.0;
            let final_hp = (enemy_entry.stats.hitpoints as f32 * mag_f) as i32;
            let final_atk = (total_atk as f32 * mag_f) as i32;
            
            let cycle = enemy_entry.stats.attack_cycle(enemy_entry.atk_anim_frames);
            let dps = if cycle > 0 { (final_atk as f32 * 30.0 / cycle as f32) as i32 } else { 0 };
            let atk_type = if enemy_entry.stats.area_attack == 0 { "Single" } else { "Area" };

            let endure = if enemy_entry.stats.knockbacks > 0 { final_hp / enemy_entry.stats.knockbacks } else { final_hp };

            let data = StatblockData {
                id_str: enemy_entry.id_str(),
                name: enemy_entry.display_name(),
                icon_path: enemy_entry.icon_path.clone(),
                top_label: "Magnification:".to_string(),
                top_value: format!("{}%", magnification),
                
                hp: final_hp.to_string(),
                kb: enemy_entry.stats.knockbacks.to_string(),
                speed: enemy_entry.stats.speed.to_string(),
                
                cd_label: "Endure".to_string(),
                cd_value: endure.to_string(),
                is_cd_time: false, 
                cd_frames: 0,
                
                cost_label: "Cash Drop".to_string(),
                cost_value: format!("{}¢", enemy_entry.stats.cash_drop),
                
                atk: final_atk.to_string(),
                dps: dps.to_string(),
                range: enemy_entry.stats.standing_range.to_string(),
                atk_cycle: cycle,
                atk_type: atk_type.to_string(),
                
                traits, h1, h2, b1, b2, footer, spirit_data: None,
            };

            let lang_clone = settings.game_language.clone();
            let cuts_clone = icon_sheet.cuts_map.clone(); 

            if export_action == ExportAction::Copy {
                generate_and_copy(ctx.clone(), lang_clone, data, cuts_clone);
            } else {
                generate_and_save(ctx.clone(), lang_clone, data, cuts_clone);
            }
        },
        ExportAction::None => {}
    }

    ui.separator(); 
    ui.add_space(0.0);

    match current_tab {
        EnemyDetailTab::Abilities => {
            stats::render(ui, enemy_entry, *magnification);
            ui.spacing_mut().item_spacing.y = 7.0;
            ui.separator();
            
            egui::ScrollArea::vertical()
                .auto_shrink([false, false]) 
                .show(ui, |ui| {
                    abilities::render(
                        ui, 
                        enemy_entry, 
                        icon_sheet, 
                        assets, // PASSED: Centralized struct
                        settings,
                        *magnification
                    );
                });
        },
        EnemyDetailTab::Details => {
            details::render(ui, &enemy_entry.description);
        },
        EnemyDetailTab::Animation => {
            viewer::show(ui, ctx, enemy_entry, anim_viewer, model_data, anim_sheet, settings);
        }
    }
}