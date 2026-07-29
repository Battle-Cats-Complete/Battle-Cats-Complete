use std::cell::RefCell;
use std::collections::HashMap;

use iced::widget::image::Handle;
use iced::widget::{column, container, image as iced_image, row, text, tooltip};
use iced::{font, Alignment, Element, Length};
use nyanko::chapter::map::{BonusType, RuleType, ScoreBonusMapEntry, SpecialRulesMapEntry};
use nyanko::chapter::stage::{BossType, EnemyAmount};
use nyanko::common::tools::file::{strip_html_tags, BreakHandling};
use tracing::warn;

use core::common::context::GlobalContext;
use core::modules::enemy::scanner::EnemyEntry;
use core::modules::stage::{restrictions, Map, Stage};

use super::icons;

const SUBHEADER_SIZE: f32 = 14.0;
const MAX_ICON_SIZE: f32 = 32.0;

fn bold_text<'a>(content: impl ToString) -> iced::widget::Text<'a> {
    text(content.to_string()).font(font::Font {
        weight: font::Weight::Bold,
        ..Default::default()
    })
}

fn format_enemy_amount(spawn_amount: &EnemyAmount) -> String {
    match spawn_amount {
        EnemyAmount::Infinite => "∞".to_string(),
        EnemyAmount::Limit(limit_amount) => limit_amount.to_string(),
    }
}

fn format_enemy_respawn(spawn_amount: &EnemyAmount, respawn_min_frames: u32, respawn_max_frames: u32) -> String {
    if spawn_amount == &EnemyAmount::Limit(1) {
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
    if kill_count == 0 { "-".to_string() } else { kill_count.to_string() }
}

fn format_score(score: u32) -> String {
    if score == 0 { "-".to_string() } else { score.to_string() }
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

fn format_special_rule(rule: &SpecialRulesMapEntry, global_ctx: &GlobalContext) -> String {
    let explanation_key = if !rule.explanation_label.is_empty() {
        rule.explanation_label.clone()
    } else if let Some(prefix) = rule.name_label.trim().strip_suffix("Name") {
        format!("{prefix}Explanation")
    } else {
        String::new()
    };

    let raw_description = if !explanation_key.is_empty() {
        global_ctx.localizable.lookup(&explanation_key).unwrap_or_default()
    } else {
        &String::new()
    };

    let mut description = strip_html_tags(raw_description, BreakHandling::Space);

    if description.is_empty() {
        warn!(key = %explanation_key, name_label = %rule.name_label, "missing localization for special rule explanation, falling back to raw enum parsing");

        let mut fallback = String::new();
        for target_rule in &rule.rules {
            let formatted_rule = match target_rule {
                RuleType::TrustFund(params) => format!("Trust Fund {:?}", params),
                RuleType::CooldownEquality(params) => format!("Cooldown Equality {:?}", params),
                RuleType::RarityLimit(params) => format!("Rarity Limit {:?}", params),
                RuleType::CheapLabor(params) => format!("Cheap Labor {:?}", params),
                RuleType::CatCost(params) => format!("Restrict Price {:?}", params),
                RuleType::CatProduction(params) => format!("Restrict CD {:?}", params),
                RuleType::TotalDeployLimit(params) => format!("Deploy Limit {:?}", params),
                RuleType::MoreThanOne(params) => format!("Awesome Cat Spawn {:?}", params),
                RuleType::MegaCatCannon(params) => format!("Awesome Cat Cannon {:?}", params),
                RuleType::UniformMotion(params) => format!("Awesome Unit Speed {:?}", params),
                RuleType::Unknown(id, params) => format!("Unknown Rule {} {:?}", id, params),
            };
            fallback.push_str(&formatted_rule);
            fallback.push('\n');
        }
        description = fallback.trim().to_string();
    } else {
        for target_rule in &rule.rules {
            let parameters = match target_rule {
                RuleType::TrustFund(params) => params,
                RuleType::CooldownEquality(params) => params,
                RuleType::RarityLimit(params) => params,
                RuleType::CheapLabor(params) => params,
                RuleType::CatCost(params) => params,
                RuleType::CatProduction(params) => params,
                RuleType::TotalDeployLimit(params) => params,
                RuleType::MoreThanOne(params) => params,
                RuleType::MegaCatCannon(params) => params,
                RuleType::UniformMotion(params) => params,
                RuleType::Unknown(_, params) => params,
            };

            for param in parameters {
                description = description.replacen("%d", &param.to_string(), 1);
            }
        }
    }

    description
}

fn format_score_bonus(score_bonus: &ScoreBonusMapEntry, global_ctx: &GlobalContext) -> String {
    let lookup_key = if !score_bonus.explanation_label.is_empty() {
        &score_bonus.explanation_label
    } else {
        &score_bonus.name_label
    };

    let raw_description = global_ctx.localizable.lookup(lookup_key).unwrap_or_default();
    let mut description = strip_html_tags(raw_description, BreakHandling::Space);

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

fn header_cell<'a>(label: impl ToString, width: f32) -> Element<'a, super::Message> {
    container(bold_text(label).size(12)).width(Length::Fixed(width)).center_x(Length::Fill).into()
}

fn value_cell<'a>(label: impl ToString, width: f32) -> Element<'a, super::Message> {
    container(text(label.to_string()).size(12)).width(Length::Fixed(width)).center_x(Length::Fill).into()
}

#[derive(Default)]
pub struct State {
    icon_cache: RefCell<HashMap<u32, Handle>>,
}

impl State {
    pub fn clear_icons(&self) {
        self.icon_cache.borrow_mut().clear();
    }

    fn icon(&self, id: u32, path: &std::path::Path) -> Option<Handle> {
        if let Some(cached) = self.icon_cache.borrow().get(&id) {
            return Some(cached.clone());
        }

        let handle = icons::load_scaled(path, MAX_ICON_SIZE as u32)?;
        self.icon_cache.borrow_mut().insert(id, handle.clone());
        Some(handle)
    }

    pub fn view<'a>(
        &'a self,
        stage: &'a Stage,
        map: &'a Map,
        selected_crown: u8,
        enemy_registry: &'a HashMap<u32, EnemyEntry>,
        enemy_name_registry: &'a [String],
        global_ctx: GlobalContext<'a>,
    ) -> Element<'a, super::Message> {
        let mut content = column![bold_text("Battleground").size(18), iced::widget::rule::horizontal(1)].spacing(6);

        let restriction_lines = restrictions::parse_restrictions(stage, selected_crown as i8, global_ctx);

        if !restriction_lines.is_empty() {
            let mut restriction_col = column![bold_text("Restrictions").size(SUBHEADER_SIZE)].spacing(2);
            for line in &restriction_lines {
                restriction_col = restriction_col.push(text(line.clone()));
            }
            content = content.push(restriction_col);
        }

        if let Some(rule) = &map.special_rules {
            let mut rules_col = column![bold_text("Rules").size(SUBHEADER_SIZE)].spacing(2);
            rules_col = rules_col.push(text(format_special_rule(rule, &global_ctx)));
            if !map.invalid_combos.is_empty() {
                rules_col = rules_col.push(text(format!("Disabled Combos: {} total", map.invalid_combos.len())));
            }
            content = content.push(rules_col);
        }

        if let Some(score_bonus) = &map.score_bonuses {
            let bonus_col = column![
                bold_text("Score Bonus").size(SUBHEADER_SIZE),
                text(format_score_bonus(score_bonus, &global_ctx)),
            ].spacing(2);
            content = content.push(bonus_col);
        }

        if stage.enemies.is_empty() {
            return content.push(text("No enemies defined for this stage.")).into();
        }

        let crown_mag = match selected_crown {
            1 => map.crown_2_mag.unwrap_or(100),
            2 => map.crown_3_mag.unwrap_or(100),
            3 => map.crown_4_mag.unwrap_or(100),
            _ => map.crown_1_mag.unwrap_or(100),
        };

        let show_score_column = stage.enemies.iter().any(|enemy| enemy.score > 0);
        let is_dojo_mechanic = stage.enemies.iter().any(|enemy| enemy.base_hp_perc > 100);
        let has_split_magnification = stage.enemies.iter().any(|enemy| {
            let final_hp_mag = (enemy.magnification * crown_mag) / 100;
            let final_atk_mag = (enemy.atk_magnification * crown_mag) / 100;
            final_hp_mag != final_atk_mag
        });

        let mag_width = if has_split_magnification { 110.0 } else { 80.0 };
        let mag_header = if has_split_magnification { "Magnification %\n(HP% / ATK%)" } else { "Magnification %" };
        let base_header = if is_dojo_mechanic { "Dmg #" } else { "Base %" };

        let mut header_row = row![
            header_cell("Enemy", 60.0),
            header_cell("Count", 45.0),
            header_cell(mag_header, mag_width),
            header_cell(base_header, 55.0),
            header_cell("Spawn", 55.0),
            header_cell("Respawn", 70.0),
            header_cell("Layer", 50.0),
            header_cell("Boss", 60.0),
        ].spacing(6);

        if show_score_column {
            header_row = header_row.push(header_cell("Score", 50.0));
        }
        header_row = header_row.push(header_cell("Kills", 50.0));

        let mut grid = column![header_row].spacing(4);

        for spawn in &stage.enemies {
            let resolved_name = enemy_name_registry
                .get(spawn.enemy_id as usize)
                .filter(|name| !name.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("{:03}-E", spawn.enemy_id));

            let icon_element: Element<'a, super::Message> = enemy_registry.get(&spawn.enemy_id)
                .and_then(|entry| entry.icon_path.as_ref())
                .and_then(|path| self.icon(spawn.enemy_id, path))
                .map(|handle| tooltip(
                    iced_image(handle).width(Length::Fixed(MAX_ICON_SIZE)).height(Length::Fixed(MAX_ICON_SIZE)),
                    container(text(resolved_name.clone())).padding(6).style(container::bordered_box),
                    tooltip::Position::Top,
                ).into())
                .unwrap_or_else(|| tooltip(
                    text(format!("{:03}", spawn.enemy_id)).size(11),
                    container(text(resolved_name.clone())).padding(6).style(container::bordered_box),
                    tooltip::Position::Top,
                ).into());

            let final_hp_mag = (spawn.magnification * crown_mag) / 100;
            let final_atk_mag = (spawn.atk_magnification * crown_mag) / 100;
            let formatted_mag = if final_hp_mag == final_atk_mag {
                format!("{}%", final_hp_mag)
            } else {
                format!("{}% / {}%", final_hp_mag, final_atk_mag)
            };

            let mut value_row = row![
                container(icon_element).width(Length::Fixed(60.0)).center_x(Length::Fill).align_y(Alignment::Center),
                value_cell(format_enemy_amount(&spawn.amount), 45.0),
                value_cell(formatted_mag, mag_width),
                value_cell(format_base_hp_percentage(spawn.base_hp_perc, is_dojo_mechanic), 55.0),
                value_cell(format!("{}f", spawn.start_frame), 55.0),
                value_cell(format_enemy_respawn(&spawn.amount, spawn.respawn_min, spawn.respawn_max), 70.0),
                value_cell(format_layer(spawn.layer_min, spawn.layer_max), 50.0),
                value_cell(format_boss_type(&spawn.boss_type), 60.0),
            ].spacing(6).align_y(Alignment::Center);

            if show_score_column {
                value_row = value_row.push(value_cell(format_score(spawn.score), 50.0));
            }
            value_row = value_row.push(value_cell(format_kill_count(spawn.kill_count), 50.0));

            grid = grid.push(value_row);
        }

        content.push(grid).into()
    }
}
