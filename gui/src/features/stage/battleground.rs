use std::collections::HashMap;
use std::path::Path;

use eframe::egui;
use nyanko::chapter::stage::{EnemyAmount, BossType};
use nyanko::common::utils::csv::{strip_html_tags, BreakHandling};
use tracing::{debug, instrument, warn};

use core::enemy::logic::scanner::EnemyEntry;
use core::global::context::GlobalContext;
use core::global::utils::autocrop;
use core::stage::data::scorebonusmap::{BonusType, ScoreBonus};
use core::stage::data::specialrulesmap::{RuleType, SpecialRule};
use core::stage::registry::{Map, Stage};

use super::treasure::center_header;

fn format_enemy_amount(spawn_amount: &EnemyAmount) -> String {
    match spawn_amount {
        EnemyAmount::Infinite => "∞".to_string(),
        EnemyAmount::Limit(limit_amount) => limit_amount.to_string(),
    }
}

fn format_enemy_respawn(spawn_amount: &EnemyAmount, respawn_min_frames: u32, respawn_max_frames: u32) -> String {
    let is_singular_spawn = spawn_amount == &EnemyAmount::Limit(1);
    if is_singular_spawn {
        return "-".to_string();
    }

    if respawn_min_frames == respawn_max_frames {
        return format!("{}f", respawn_min_frames);
    }

    format!("{}f ~ {}f", respawn_min_frames, respawn_max_frames)
}

fn format_layer(layer_min: i32, layer_max: i32) -> String {
    if layer_min == layer_max {
        return layer_min.to_string();
    }
    format!("{} ~ {}", layer_min, layer_max)
}

fn format_boss_type(boss_type: &BossType) -> String {
    match boss_type {
        BossType::None => "-".to_string(),
        BossType::Boss => "Yes".to_string(),
        BossType::ScreenShake => "Yes (Shake)".to_string(),
        BossType::Unknown(_) => "Unknown".to_string(),
    }
}

fn format_kill_count(kill_count: u32) -> String {
    if kill_count == 0 {
        return "-".to_string();
    }
    kill_count.to_string()
}

fn format_score(score: u32) -> String {
    if score == 0 {
        return "-".to_string();
    }
    score.to_string()
}

fn format_base_hp_percentage(base_hp_perc: u32, is_dojo_mechanic: bool) -> String {
    if is_dojo_mechanic {
        return base_hp_perc.to_string();
    }

    if base_hp_perc == 100 {
        return "-".to_string();
    }
    format!("{}%", base_hp_perc)
}

#[instrument(skip(rule, global_ctx))]
fn format_special_rule(rule: &SpecialRule, global_ctx: &GlobalContext) -> String {
    let clean_key = rule.name_label.trim();
    let explanation_key = clean_key.replace("Name", "Explanation");

    let raw_description = global_ctx.localizable.lookup(&explanation_key).unwrap_or_default();
    let mut description = strip_html_tags(&raw_description, BreakHandling::Space);

    if description.is_empty() {
        let raw_title = global_ctx.localizable.lookup(clean_key).unwrap_or_default();
        let mut title = strip_html_tags(&raw_title, BreakHandling::Space);

        if title.is_empty() {
            title = clean_key.to_string();
        }

        description = format!("【{}】 Localization data missing.", title);
    } else {
        for target_rule in &rule.rules {
            let parameters = match target_rule {
                RuleType::TrustFund(params) => params,
                RuleType::CooldownEquality(params) => params,
                RuleType::RarityLimit(params) => params,
                RuleType::CheapLabor(params) => params,
                RuleType::RestrictPrice(params) => params,
                RuleType::RestrictCd(params) => params,
                RuleType::DeployLimit(params) => params,
                RuleType::AwesomeCatSpawn(params) => params,
                RuleType::AwesomeCatCannon(params) => params,
                RuleType::AwesomeUnitSpeed(params) => params,
                RuleType::Unknown(_, params) => params,
            };

            for param in parameters {
                description = description.replacen("%d", &param.to_string(), 1);
            }
        }
    }

    description
}

#[instrument(skip(score_bonus, global_ctx))]
fn format_score_bonus(score_bonus: &ScoreBonus, global_ctx: &GlobalContext) -> String {
    let lookup_key = if !score_bonus.explanation_label.is_empty() {
        &score_bonus.explanation_label
    } else {
        &score_bonus.name_label
    };

    let raw_description = global_ctx.localizable.lookup(lookup_key).unwrap_or_default();
    let mut description = strip_html_tags(&raw_description, BreakHandling::Space);

    if description.is_empty() {
        description = format!("【{}】 Localization data missing.", lookup_key);
    } else {
        for bonus in &score_bonus.bonuses {
            let parameters = match bonus {
                BonusType::Weaken(params) => params,
                BonusType::Freeze(params) => params,
                BonusType::Slow(params) => params,
                BonusType::Knockback(params) => params,
                BonusType::StrongAttack(params) => params,
                BonusType::MassiveDamage(params) => params,
                BonusType::StrongDefense(params) => params,
                BonusType::Resist(params) => params,
                BonusType::Unknown(_, params) => params,
            };

            for param in parameters {
                description = description.replacen("%d", &param.to_string(), 1);
            }
        }
    }

    description
}

#[instrument(skip(icon_file_path), fields(path = %icon_file_path.display()))]
fn process_enemy_icon_texture(icon_file_path: &Path) -> Option<egui::ColorImage> {
    debug!("Loading raw image file for icon processing");
    let loaded_raw_image_data = match image::open(icon_file_path) {
        Ok(data) => data,
        Err(err) => {
            warn!(error = %err, "Failed to open enemy icon texture file");
            return None;
        }
    };

    let autocropped_rgba_image = autocrop(loaded_raw_image_data.to_rgba8());
    let image_dimensions = [autocropped_rgba_image.width() as usize, autocropped_rgba_image.height() as usize];

    debug!(width = image_dimensions[0], height = image_dimensions[1], "Icon image autocropped successfully");
    Some(egui::ColorImage::from_rgba_unmultiplied(image_dimensions, autocropped_rgba_image.as_flat_samples().as_slice()))
}

fn center_enemy_text(ui: &mut egui::Ui, display_text: impl Into<String>) {
    ui.centered_and_justified(|ui| {
        ui.add(egui::Label::new(display_text.into()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

#[instrument(skip_all, fields(stage_id = %stage_data.stage_id))]
pub fn draw(
    egui_context: &egui::Context,
    ui: &mut egui::Ui,
    stage_data: &Stage,
    map_data: &Map,
    selected_crown: u8,
    enemy_registry: &HashMap<u32, EnemyEntry>,
    enemy_name_registry: &[String],
    texture_cache: &mut HashMap<u32, egui::TextureHandle>,
    global_ctx: GlobalContext
) {
    ui.strong("Battleground");
    ui.separator();

    let restrictions = core::stage::logic::restrictions::parse_restrictions(stage_data, selected_crown as i8, global_ctx.clone());

    if !restrictions.is_empty() {
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Stage Restrictions").strong());

        ui.indent("stage_restrictions_indent", |ui| {
            for restriction in &restrictions {
                ui.label(format!("• {}", restriction));
            }
        });
    }

    if let Some(rule) = &map_data.special_rules {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Special Rules").strong());

        ui.indent("special_rules_indent", |ui| {
            let rule_description = format_special_rule(rule, &global_ctx);

            ui.label(format!("• {}", rule_description));

            if !map_data.invalid_combos.is_empty() {
                ui.label(format!("• Disabled Combos: {} total", map_data.invalid_combos.len()));
            }
        });
    }

    if let Some(score_bonus) = &map_data.score_bonuses {
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Dojo Score Bonus").strong());

        ui.indent("score_bonus_indent", |ui| {
            let desc = format_score_bonus(score_bonus, &global_ctx);
            ui.label(format!("• {}", desc));
        });
    }

    if restrictions.is_empty() && map_data.special_rules.is_none() && map_data.score_bonuses.is_none() {
        debug!("No stage restrictions, special rules, or score bonuses found for current crown to display");
    } else {
        ui.add_space(8.0);
    }

    if stage_data.enemies.is_empty() {
        ui.label("No enemies defined for this stage.");
        return;
    }

    let crown_mag = match selected_crown {
        1 => map_data.crown_2_mag.unwrap_or(100),
        2 => map_data.crown_3_mag.unwrap_or(100),
        3 => map_data.crown_4_mag.unwrap_or(100),
        _ => map_data.crown_1_mag.unwrap_or(100),
    };

    let show_score_column = stage_data.enemies.iter().any(|enemy| enemy.score > 0);
    let is_dojo_mechanic = stage_data.enemies.iter().any(|enemy| enemy.base_hp_perc > 100);

    let has_split_magnification = stage_data.enemies.iter().any(|enemy| {
        let final_hp_mag = (enemy.magnification * crown_mag) / 100;
        let final_atk_mag = (enemy.atk_magnification * crown_mag) / 100;
        final_hp_mag != final_atk_mag
    });

    egui::Grid::new("enemy_grid")
        .striped(true)
        .spacing([15.0, 4.0])
        .min_row_height(32.0)
        .show(ui, |grid| {
            center_header(grid, "Enemy");
            center_header(grid, "Count");

            if has_split_magnification {
                center_header(grid, "Magnification %\n(HP% / ATK%)");
            } else {
                center_header(grid, "Magnification %");
            }

            center_header(grid, if is_dojo_mechanic { "Dmg #" } else { "Base %" });
            center_header(grid, "Spawn");
            center_header(grid, "Respawn");
            center_header(grid, "Layer");
            center_header(grid, "Boss");
            if show_score_column {
                center_header(grid, "Score");
            }
            center_header(grid, "Kills");
            grid.end_row();

            for spawn in &stage_data.enemies {
                let resolved_enemy_name = enemy_name_registry
                    .get(spawn.enemy_id as usize)
                    .filter(|string_val| !string_val.is_empty())
                    .cloned()
                    .unwrap_or_else(|| format!("{:03}-E", spawn.enemy_id));

                grid.with_layout(egui::Layout::bottom_up(egui::Align::Center), |icon_layout| {
                    let has_rendered_icon = 'icon: {
                        let Some(located_enemy_entry) = enemy_registry.get(&spawn.enemy_id) else {
                            break 'icon false;
                        };
                        let Some(enemy_icon_path) = &located_enemy_entry.icon_path else {
                            break 'icon false;
                        };

                        if let std::collections::hash_map::Entry::Vacant(cache_entry) = texture_cache.entry(spawn.enemy_id) {
                            debug!(id = spawn.enemy_id, "Texture cache miss, attempting processing");
                            let Some(processed_color_image) = process_enemy_icon_texture(enemy_icon_path) else {
                                break 'icon false;
                            };
                            let generated_texture_handle = egui_context.load_texture(
                                format!("stage_enemy_icon_{}", spawn.enemy_id),
                                processed_color_image,
                                egui::TextureOptions::LINEAR
                            );
                            cache_entry.insert(generated_texture_handle);
                        }

                        let Some(cached_texture_handle) = texture_cache.get(&spawn.enemy_id) else {
                            break 'icon false;
                        };
                        let image_response = icon_layout.add(egui::Image::new(cached_texture_handle).max_size(egui::vec2(32.0, 32.0)));
                        image_response.on_hover_text(resolved_enemy_name.clone());
                        true
                    };

                    if !has_rendered_icon {
                        icon_layout.add_space(6.0);
                        let label_response = icon_layout.add(egui::Label::new(format!("{:03}", spawn.enemy_id)).wrap_mode(egui::TextWrapMode::Extend));
                        label_response.on_hover_text(resolved_enemy_name);
                    }
                });

                let final_hp_mag = (spawn.magnification * crown_mag) / 100;
                let final_atk_mag = (spawn.atk_magnification * crown_mag) / 100;

                let formatted_mag = if final_hp_mag == final_atk_mag {
                    format!("{}%", final_hp_mag)
                } else {
                    format!("{}% / {}%", final_hp_mag, final_atk_mag)
                };

                let formatted_amount = format_enemy_amount(&spawn.amount);
                let formatted_base_hp = format_base_hp_percentage(spawn.base_hp_perc, is_dojo_mechanic);
                let formatted_respawn = format_enemy_respawn(&spawn.amount, spawn.respawn_min, spawn.respawn_max);
                let formatted_layer = format_layer(spawn.layer_min, spawn.layer_max);
                let formatted_boss_type = format_boss_type(&spawn.boss_type);
                let formatted_score = format_score(spawn.score);
                let formatted_kill_count = format_kill_count(spawn.kill_count);

                center_enemy_text(grid, formatted_amount);
                center_enemy_text(grid, formatted_mag);
                center_enemy_text(grid, formatted_base_hp);
                center_enemy_text(grid, format!("{}f", spawn.start_frame));
                center_enemy_text(grid, formatted_respawn);
                center_enemy_text(grid, formatted_layer);
                center_enemy_text(grid, formatted_boss_type);
                if show_score_column {
                    center_enemy_text(grid, formatted_score);
                }
                center_enemy_text(grid, formatted_kill_count);

                grid.end_row();
            }
        });
}