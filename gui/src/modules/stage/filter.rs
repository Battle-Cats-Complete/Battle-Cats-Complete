use eframe::egui;

use core::modules::stage::filter::{enemy::EnemyFilter, lineup::LineupFilter, material::MaterialFilter, range::StatRange, treasure::TreasureFilter};
use core::modules::stage::filter::StageFilterState;

use crate::common::shared::DragGuard;

pub(crate) const WINDOW_WIDTH: f32 = 400.0;
pub(crate) const WINDOW_HEIGHT: f32 = 500.0;
pub(crate) const TILDE_SPACING: f32 = 5.0;

pub(crate) fn show_popup(
    ctx: &egui::Context,
    state: &mut StageFilterState,
    drag_guard: &mut DragGuard,
) {
    if !state.is_open {
        return;
    }

    let window_id = egui::Id::new("Stage Filter");
    let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);

    let mut clear_filters = false;
    let mut is_open_local = state.is_open;

    let mut window = egui::Window::new("Advanced Stage Filter")
        .id(window_id)
        .open(&mut is_open_local)
        .collapsible(false)
        .resizable(true)
        .constrain(false)
        .movable(allow_drag)
        .default_pos(ctx.screen_rect().center() - egui::vec2(WINDOW_WIDTH / 2.0, WINDOW_HEIGHT / 2.0))
        .default_size([WINDOW_WIDTH, WINDOW_HEIGHT])
        .min_width(400.0)
        .min_height(500.0);

    if let Some(pos) = fixed_pos {
        window = window.current_pos(pos);
    }

    window.show(ctx, |ui| {
        let max_rect = ui.max_rect();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.heading("Name");
                ui.add_space(5.0);

                egui::Grid::new("stage_name_filter_grid")
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Category:").strong());
                        ui.add_sized(
                            egui::vec2(150.0, 20.0),
                            egui::TextEdit::singleline(&mut state.category_name).hint_text(egui::RichText::new("Any").color(egui::Color32::from_gray(100)))
                        );
                        ui.end_row();

                        ui.label(egui::RichText::new("Map:").strong());
                        ui.add_sized(
                            egui::vec2(150.0, 20.0),
                            egui::TextEdit::singleline(&mut state.map_name).hint_text(egui::RichText::new("Any").color(egui::Color32::from_gray(100)))
                        );
                        ui.end_row();

                        ui.label(egui::RichText::new("Stage:").strong());
                        ui.add_sized(
                            egui::vec2(150.0, 20.0),
                            egui::TextEdit::singleline(&mut state.stage_name).hint_text(egui::RichText::new("Any").color(egui::Color32::from_gray(100)))
                        );
                        ui.end_row();
                    });

                ui.add_space(15.0);
                ui.heading("General Rules");
                ui.add_space(5.0);

                egui::Grid::new("general_rules_grid")
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Continues:").strong());
                        tristate_btn(ui, &mut state.continues);

                        ui.label(egui::RichText::new("Boss Guard:").strong());
                        tristate_btn(ui, &mut state.boss_guard);
                        ui.end_row();

                        ui.label(egui::RichText::new("Use Super CPU:").strong());
                        tristate_btn(ui, &mut state.use_super_cpu);
                        ui.end_row();
                    });

                ui.add_space(15.0);
                ui.heading("Special Map Rules");
                ui.add_space(5.0);

                let map_rules = [
                    ("Trust Fund", &mut state.rule_trust_fund),
                    ("Cooldown Equality", &mut state.rule_cooldown_equality),
                    ("Rarity Limit", &mut state.rule_rarity_limit),
                    ("Cheap Labor", &mut state.rule_cheap_labor),
                    ("Cat Cost", &mut state.rule_cat_cost),
                    ("Cat Production", &mut state.rule_cat_production),
                    ("Total Deploy Limit", &mut state.rule_total_deploy_limit),
                    ("More Than One", &mut state.rule_more_than_one),
                    ("Mega Cat Cannon", &mut state.rule_mega_cat_cannon),
                    ("Uniform Motion", &mut state.rule_uniform_motion),
                    ("Invalid Combos", &mut state.invalid_combos),
                ];

                egui::Grid::new("special_rules_grid")
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        for (i, (label, val_ref)) in map_rules.into_iter().enumerate() {
                            ui.label(egui::RichText::new(label).strong());
                            tristate_btn(ui, val_ref);
                            if (i + 1) % 2 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(15.0);
                ui.heading("Score Bonuses");
                ui.add_space(5.0);

                let bonus_rules = [
                    ("Weaken", &mut state.bonus_weaken),
                    ("Freeze", &mut state.bonus_freeze),
                    ("Slow", &mut state.bonus_slow),
                    ("Knockback", &mut state.bonus_knockback),
                    ("Strong Attack", &mut state.bonus_strong_attack),
                    ("Massive Damage", &mut state.bonus_massive_damage),
                    ("Strong Defense", &mut state.bonus_strong_defense),
                    ("Resist", &mut state.bonus_resist),
                ];

                egui::Grid::new("score_bonus_grid")
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        for (i, (label, val_ref)) in bonus_rules.into_iter().enumerate() {
                            ui.label(egui::RichText::new(label).strong());
                            tristate_btn(ui, val_ref);
                            if (i + 1) % 2 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(15.0);
                ui.heading("Stats");
                ui.add_space(5.0);

                let stat_rows = [
                    ("Base HP", &mut state.base_hp),
                    ("Width", &mut state.width),
                    ("Time Limit (f)", &mut state.time_limit),
                    ("Max Enemies", &mut state.max_enemies),
                    ("Energy Cost", &mut state.energy),
                    ("XP Reward", &mut state.xp),
                    ("Difficulty", &mut state.difficulty),
                    ("Max Crowns", &mut state.max_crowns),
                    ("Target Crowns", &mut state.target_crowns),
                    ("Min Spawn", &mut state.min_spawn),
                    ("Max Spawn", &mut state.max_spawn),
                ];

                egui::Grid::new("stage_stat_filter_grid")
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        for (i, (label, range)) in stat_rows.into_iter().enumerate() {
                            draw_stat_range(ui, label, range);
                            if (i + 1) % 2 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(15.0);
                ui.heading("Restrictions");
                ui.add_space(5.0);

                let restriction_rows = [
                    ("Deploy Limit", &mut state.deploy_limit),
                    ("Allowed Rows", &mut state.allowed_rows),
                    ("Min Cost", &mut state.min_cost),
                    ("Max Cost", &mut state.max_cost),
                ];

                egui::Grid::new("stage_restriction_filter_grid")
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        for (i, (label, range)) in restriction_rows.into_iter().enumerate() {
                            draw_stat_range(ui, label, range);
                            if (i + 1) % 2 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(15.0);
                ui.heading("IDs & Audio");
                ui.add_space(5.0);

                let id_rows = [
                    ("Base ID", &mut state.base_id),
                    ("Anim Base ID", &mut state.anim_base_id),
                    ("Background ID", &mut state.background_id),
                    ("Init Track", &mut state.init_track),
                    ("Boss Track", &mut state.boss_track),
                    ("BGM Change (%)", &mut state.bgm_change_percent),
                ];

                egui::Grid::new("stage_id_filter_grid")
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        for (i, (label, range)) in id_rows.into_iter().enumerate() {
                            draw_stat_range(ui, label, range);
                            if (i + 1) % 2 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(15.0);
                ui.heading("Battleground");
                ui.add_space(5.0);

                let mut to_remove_e = None;
                for (idx, enemy) in state.enemies.iter_mut().enumerate() {
                    ui.add_space(8.0);

                    egui::Frame::none()
                        .fill(egui::Color32::from_black_alpha(150))
                        .rounding(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let mode_label = if enemy.is_exclude { "Exclude" } else { "Include" };
                                egui::ComboBox::from_id_salt(format!("emode_{}", idx))
                                    .selected_text(mode_label)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut enemy.is_exclude, false, "Include");
                                        ui.selectable_value(&mut enemy.is_exclude, true, "Exclude");
                                    });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let close_btn = egui::Button::new(
                                        egui::RichText::new(" X ").color(egui::Color32::WHITE).strong()
                                    )
                                        .fill(egui::Color32::from_rgb(210, 50, 50))
                                        .rounding(4.0)
                                        .min_size(egui::vec2(24.0, 24.0));

                                    if ui.add(close_btn).clicked() {
                                        to_remove_e = Some(idx);
                                    }
                                });
                            });

                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Boss Type:").strong());
                                boss_type_combo(ui, format!("boss_type_{}", idx), &mut enemy.boss_type);

                                ui.add_space(10.0);
                                ui.label(egui::RichText::new("Is Base:").strong());
                                tristate_btn(ui, &mut enemy.is_base);
                            });

                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Name or ID:").strong());
                                ui.add_sized(
                                    egui::vec2(150.0, 20.0),
                                    egui::TextEdit::singleline(&mut enemy.name_or_id).hint_text(egui::RichText::new("Any").color(egui::Color32::from_gray(100)))
                                );
                            });

                            ui.add_space(6.0);

                            let enemy_rows = [
                                ("Amount", &mut enemy.amount),
                                ("Start Frame", &mut enemy.start_frame),
                                ("Respawn Min", &mut enemy.respawn_min),
                                ("Respawn Max", &mut enemy.respawn_max),
                                ("Base HP (%)", &mut enemy.base_hp_perc),
                                ("Layer Min", &mut enemy.layer_min),
                                ("Layer Max", &mut enemy.layer_max),
                                ("Magnification", &mut enemy.magnification),
                                ("Atk Mag", &mut enemy.atk_magnification),
                                ("Score", &mut enemy.score),
                                ("Time Flag", &mut enemy.time_flag),
                                ("Kill Count", &mut enemy.kill_count),
                            ];

                            egui::Grid::new(format!("enemy_grid_{}", idx))
                                .spacing([16.0, 6.0])
                                .show(ui, |ui| {
                                    for (i, (label, range)) in enemy_rows.into_iter().enumerate() {
                                        draw_stat_range(ui, label, range);
                                        if (i + 1) % 2 == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                }

                if let Some(idx) = to_remove_e {
                    state.enemies.remove(idx);
                }

                ui.add_space(8.0);
                if ui.button("+ Add New Enemy").clicked() {
                    state.enemies.push(EnemyFilter::default());
                }

                ui.add_space(15.0);
                ui.heading("Fixed Lineup Cats");
                ui.add_space(5.0);

                let mut to_remove_l = None;
                for (idx, cat) in state.lineup_cats.iter_mut().enumerate() {
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(egui::Color32::from_black_alpha(150))
                        .rounding(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let mode_label = if cat.is_exclude { "Exclude" } else { "Include" };
                                egui::ComboBox::from_id_salt(format!("lmode_{}", idx))
                                    .selected_text(mode_label)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut cat.is_exclude, false, "Include");
                                        ui.selectable_value(&mut cat.is_exclude, true, "Exclude");
                                    });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let close_btn = egui::Button::new(
                                        egui::RichText::new(" X ").color(egui::Color32::WHITE).strong()
                                    )
                                        .fill(egui::Color32::from_rgb(210, 50, 50))
                                        .rounding(4.0)
                                        .min_size(egui::vec2(24.0, 24.0));
                                    if ui.add(close_btn).clicked() { to_remove_l = Some(idx); }
                                });
                            });

                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Name or ID:").strong());
                                ui.add_sized(
                                    egui::vec2(150.0, 20.0),
                                    egui::TextEdit::singleline(&mut cat.name_or_id).hint_text(egui::RichText::new("Any").color(egui::Color32::from_gray(100)))
                                );
                            });

                            ui.add_space(6.0);

                            egui::Grid::new(format!("lineup_grid_{}", idx))
                                .spacing([16.0, 6.0])
                                .show(ui, |ui| {
                                    draw_stat_range(ui, "Total Level", &mut cat.level);
                                });
                        });
                }

                if let Some(idx) = to_remove_l { state.lineup_cats.remove(idx); }

                ui.add_space(8.0);
                if ui.button("+ Add Lineup Cat").clicked() {
                    state.lineup_cats.push(LineupFilter::default());
                }

                ui.add_space(15.0);
                ui.heading("Treasures");
                ui.add_space(5.0);

                let mut to_remove_t = None;
                for (idx, treasure) in state.treasures.iter_mut().enumerate() {
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(egui::Color32::from_black_alpha(150))
                        .rounding(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let mode_label = if treasure.is_exclude { "Exclude" } else { "Include" };
                                egui::ComboBox::from_id_salt(format!("tmode_{}", idx))
                                    .selected_text(mode_label)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut treasure.is_exclude, false, "Include");
                                        ui.selectable_value(&mut treasure.is_exclude, true, "Exclude");
                                    });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let close_btn = egui::Button::new(
                                        egui::RichText::new(" X ").color(egui::Color32::WHITE).strong()
                                    )
                                        .fill(egui::Color32::from_rgb(210, 50, 50))
                                        .rounding(4.0)
                                        .min_size(egui::vec2(24.0, 24.0));
                                    if ui.add(close_btn).clicked() { to_remove_t = Some(idx); }
                                });
                            });

                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Name or ID:").strong());
                                ui.add_sized(
                                    egui::vec2(150.0, 20.0),
                                    egui::TextEdit::singleline(&mut treasure.name_or_id).hint_text(egui::RichText::new("Any").color(egui::Color32::from_gray(100)))
                                );
                            });

                            ui.add_space(6.0);

                            egui::Grid::new(format!("treasure_grid_{}", idx))
                                .spacing([16.0, 6.0])
                                .show(ui, |ui| {
                                    draw_stat_range(ui, "Amount", &mut treasure.amount);
                                    draw_stat_range(ui, "Chance (%)", &mut treasure.chance);
                                });
                        });
                }

                if let Some(idx) = to_remove_t { state.treasures.remove(idx); }

                ui.add_space(8.0);
                if ui.button("+ Add New Treasure").clicked() {
                    state.treasures.push(TreasureFilter::default());
                }

                ui.add_space(15.0);
                ui.heading("Materials");
                ui.add_space(5.0);

                let mut to_remove_m = None;
                for (idx, material) in state.materials.iter_mut().enumerate() {
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(egui::Color32::from_black_alpha(150))
                        .rounding(6.0)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let mode_label = if material.is_exclude { "Exclude" } else { "Include" };
                                egui::ComboBox::from_id_salt(format!("mmode_{}", idx))
                                    .selected_text(mode_label)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut material.is_exclude, false, "Include");
                                        ui.selectable_value(&mut material.is_exclude, true, "Exclude");
                                    });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let close_btn = egui::Button::new(
                                        egui::RichText::new(" X ").color(egui::Color32::WHITE).strong()
                                    )
                                        .fill(egui::Color32::from_rgb(210, 50, 50))
                                        .rounding(4.0)
                                        .min_size(egui::vec2(24.0, 24.0));
                                    if ui.add(close_btn).clicked() { to_remove_m = Some(idx); }
                                });
                            });

                            ui.add_space(8.0);

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Name or ID:").strong());
                                ui.add_sized(
                                    egui::vec2(150.0, 20.0),
                                    egui::TextEdit::singleline(&mut material.name_or_id).hint_text(egui::RichText::new("Any").color(egui::Color32::from_gray(100)))
                                );
                            });

                            ui.add_space(6.0);

                            egui::Grid::new(format!("material_grid_{}", idx))
                                .spacing([16.0, 6.0])
                                .show(ui, |ui| {
                                    draw_stat_range(ui, "Amount", &mut material.amount);
                                });
                        });
                }

                if let Some(idx) = to_remove_m { state.materials.remove(idx); }

                ui.add_space(8.0);
                if ui.button("+ Add New Material").clicked() {
                    state.materials.push(MaterialFilter::default());
                }

                ui.add_space(60.0);
            });

        let btn_size = egui::vec2(160.0, 34.0);
        let btn_rect = egui::Rect::from_center_size(
            max_rect.center_bottom() - egui::vec2(0.0, btn_size.y / 2.0 + 12.0),
            btn_size
        );

        let clear_btn = egui::Button::new(
            egui::RichText::new("Clear Filter").color(egui::Color32::WHITE).strong().size(15.0)
        )
            .fill(egui::Color32::from_rgb(210, 50, 50))
            .rounding(6.0);

        if ui.put(btn_rect, clear_btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            clear_filters = true;
        }
    });

    state.is_open = is_open_local;

    if clear_filters {
        *state = StageFilterState { is_open: state.is_open, ..Default::default() };
    }
}

fn draw_stat_range(ui: &mut egui::Ui, label: &str, range: &mut StatRange) {
    ui.label(format!("{}:", label));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TILDE_SPACING;
        let hint = egui::RichText::new("Any").color(egui::Color32::from_gray(100));
        ui.add_sized(
            egui::vec2(55.0, 20.0),
            egui::TextEdit::singleline(&mut range.min).hint_text(hint.clone())
        );
        ui.label("~");
        ui.add_sized(
            egui::vec2(55.0, 20.0),
            egui::TextEdit::singleline(&mut range.max).hint_text(hint)
        );
    });
}

fn tristate_btn(ui: &mut egui::Ui, val: &mut Option<bool>) {
    let label = val.map(|v| if v { "Yes" } else { "No" }).unwrap_or("Any");
    let bg_color = val.map(|v| if v { egui::Color32::from_rgb(31, 106, 165) } else { egui::Color32::from_rgb(210, 50, 50) }).unwrap_or_else(|| ui.visuals().widgets.inactive.bg_fill);

    let btn = egui::Button::new(label).fill(bg_color).min_size(egui::vec2(50.0, 20.0));

    if ui.add(btn).clicked() {
        *val = val.map(|v| if v { Some(false) } else { None }).unwrap_or(Some(true));
    }
}

fn boss_type_combo(ui: &mut egui::Ui, id_source: impl std::hash::Hash, val: &mut Option<u32>) {
    let label = val.map(|v| ["None", "Boss", "Screen Shake"].get(v as usize).copied().unwrap_or("Unknown")).unwrap_or("Any");

    egui::ComboBox::from_id_salt(id_source).selected_text(label).show_ui(ui, |ui| {
        ui.selectable_value(val, None, "Any");
        ui.selectable_value(val, Some(0), "None");
        ui.selectable_value(val, Some(1), "Boss");
        ui.selectable_value(val, Some(2), "Screen Shake");
    });
}