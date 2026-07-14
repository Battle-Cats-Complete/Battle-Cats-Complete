use eframe::egui;

use core::stage::logic::filter::StageFilterState;
use crate::global::shared::DragGuard;

pub const WINDOW_WIDTH: f32 = 380.0;
pub const WINDOW_HEIGHT: f32 = 500.0;
pub const TILDE_SPACING: f32 = 5.0;

pub fn show_popup(
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
        .min_width(360.0)
        .min_height(400.0);

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
                ui.heading("Rules");
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Continues:").strong());
                    tristate_btn(ui, &mut state.continues);
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Boss Guard:").strong());
                    tristate_btn(ui, &mut state.boss_guard);
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

                            if (i + 1) % 2 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(50.0);
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

fn tristate_btn(ui: &mut egui::Ui, val: &mut Option<bool>) {
    let (label, bg_color) = match val {
        None => ("Any", ui.visuals().widgets.inactive.bg_fill),
        Some(true) => ("Yes", egui::Color32::from_rgb(31, 106, 165)),
        Some(false) => ("No", egui::Color32::from_rgb(210, 50, 50)),
    };

    let btn = egui::Button::new(label).fill(bg_color).min_size(egui::vec2(50.0, 20.0));

    if ui.add(btn).clicked() {
        *val = match val {
            None => Some(true),
            Some(true) => Some(false),
            Some(false) => None,
        };
    }
}