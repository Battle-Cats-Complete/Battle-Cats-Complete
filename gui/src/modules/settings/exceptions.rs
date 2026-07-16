use std::fs;
use std::path::Path;

use eframe::egui;

use core::settings::logic::exceptions::{ExceptionList, ExceptionRule, RuleHandling};

use crate::global::shared::DragGuard;

use super::tabs::toggle_ui;

const COLUMN_PATTERN_WIDTH: f32 = 210.0;
const COLUMN_EXTENSION_WIDTH: f32 = 100.0;
const COLUMN_HANDLING_WIDTH: f32 = 110.0;
const COLUMN_LANGUAGE_WIDTH: f32 = 85.0;
const COLUMN_ACTION_WIDTH: f32 = 45.0;
const WINDOW_MAX_HEIGHT: f32 = 600.0;

#[derive(Clone)]
struct ManageExceptionsState {
    is_open: bool,
    reset_position: bool,
    rules: Vec<ExceptionRule>,
}

impl Default for ManageExceptionsState {
    fn default() -> Self {
        Self {
            is_open: false,
            reset_position: false,
            rules: ExceptionList::load_or_default().rules
        }
    }
}

#[derive(Clone, Default)]
struct ResetConfirmState {
    is_open: bool,
    reset_position: bool,
}

pub fn open(context: &egui::Context) {
    let state_id = egui::Id::new("manage_exceptions_state");
    let mut state = context.data(|data_map| data_map.get_temp::<ManageExceptionsState>(state_id)).unwrap_or_default();
    state.is_open = true;
    state.reset_position = true;
    context.data_mut(|data_map| data_map.insert_temp(state_id, state));
}

fn show_reset_confirm_modal(context: &egui::Context, drag_guard: &mut DragGuard) -> bool {
    let state_id = egui::Id::new("reset_rules_modal");
    let mut state = context.data(|data_map| data_map.get_temp::<ResetConfirmState>(state_id)).unwrap_or_default();

    if !state.is_open { return false; }

    let mut yes_clicked = false;
    let window_id = egui::Id::new("reset_rules_window");
    let (allow_drag, fixed_position) = drag_guard.assign_bounds(context, window_id);
    let mut should_close = false;

    let mut window = egui::Window::new("Confirm Reset")
        .id(window_id)
        .collapsible(false)
        .resizable(false)
        .constrain(false)
        .movable(allow_drag)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(context.screen_rect().center());

    if state.reset_position {
        window = window.current_pos(context.screen_rect().center());
        state.reset_position = false;
    } else if let Some(position) = fixed_position {
        window = window.current_pos(position);
    }

    window.show(context, |ui_container| {
        ui_container.set_min_width(280.0);
        ui_container.vertical_centered(|centered_ui| {
            centered_ui.add_space(5.0);
            centered_ui.label("Are you sure you want to reset to default exception rules?\nYour custom rules will be lost");
            centered_ui.add_space(15.0);

            centered_ui.horizontal(|horizontal_ui| {
                let total_width = 130.0;
                let x_offset = (horizontal_ui.available_width() - total_width) / 2.0;
                horizontal_ui.add_space(x_offset);

                if horizontal_ui.add_sized([60.0, 30.0], egui::Button::new("Yes")).clicked() {
                    yes_clicked = true;
                    should_close = true;
                }
                horizontal_ui.add_space(10.0);
                if horizontal_ui.add_sized([60.0, 30.0], egui::Button::new("No")).clicked() {
                    should_close = true;
                }
            });
            centered_ui.add_space(5.0);
        });
    });

    if should_close { state.is_open = false; }
    context.data_mut(|data_map| data_map.insert_temp(state_id, state));
    yes_clicked
}

pub fn show(context: &egui::Context, drag_guard: &mut DragGuard) {
    let state_id = egui::Id::new("manage_exceptions_state");
    let mut state = context.data(|data_map| data_map.get_temp::<ManageExceptionsState>(state_id)).unwrap_or_default();

    if !state.is_open { return; }

    let window_id = egui::Id::new("manage_exceptions_window_v2");
    let (allow_drag, fixed_position) = drag_guard.assign_bounds(context, window_id);
    let original_rules = state.rules.clone();
    let mut is_open = state.is_open;

    let mut window = egui::Window::new("Manage Exceptions")
        .id(window_id)
        .open(&mut is_open)
        .collapsible(false)
        .resizable(false)
        .constrain(false)
        .movable(allow_drag)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(context.screen_rect().center());

    if state.reset_position {
        window = window.current_pos(context.screen_rect().center());
        state.reset_position = false;
    } else if let Some(position) = fixed_position {
        window = window.current_pos(position);
    }

    window.show(context, |ui_container| {
        ui_container.add_space(10.0);

        let button_height = 24.0;
        let button_width = 120.0;
        let default_color = egui::Color32::from_rgb(31, 106, 165);
        let success_color = egui::Color32::from_rgb(40, 160, 60);
        let fail_color = egui::Color32::from_rgb(200, 40, 40);
        let danger_color = egui::Color32::from_rgb(180, 50, 50);
        let current_time = ui_container.input(|input_state| input_state.time);

        ui_container.vertical_centered(|centered_ui| {
            centered_ui.horizontal(|horizontal_ui| {
                let table_width = COLUMN_PATTERN_WIDTH + COLUMN_EXTENSION_WIDTH + COLUMN_HANDLING_WIDTH + COLUMN_LANGUAGE_WIDTH + COLUMN_ACTION_WIDTH + (15.0 * 4.0);
                let spacing = horizontal_ui.spacing().item_spacing.x;
                let total_button_width = (button_width * 4.0) + (spacing * 3.0);

                let x_offset = (table_width - total_button_width) / 2.0;
                horizontal_ui.add_space(x_offset.max(0.0));

                if horizontal_ui.add_sized([button_width, button_height], egui::Button::new(egui::RichText::new("Add Rule").size(12.0).strong().color(egui::Color32::WHITE)).fill(default_color).rounding(4.0)).clicked() {
                    state.rules.push(ExceptionRule::default());
                }

                let import_time = context.data(|data_map| data_map.get_temp::<f64>(egui::Id::new("exceptions_import_time"))).unwrap_or(-10.0);
                let import_result = context.data(|data_map| data_map.get_temp::<bool>(egui::Id::new("exceptions_import_result"))).unwrap_or(false);
                let (import_text, import_color) = if (current_time - import_time) < 2.0 {
                    if import_result { ("Imported!", success_color) } else { ("Failed!", fail_color) }
                } else { ("Load List", default_color) };

                if horizontal_ui.add_sized([button_width, button_height], egui::Button::new(egui::RichText::new(import_text).size(12.0).strong().color(egui::Color32::WHITE)).fill(import_color).rounding(4.0)).clicked()
                    && let Some(file_path) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_file() {
                    let success = match ExceptionList::load_from_file(&file_path) {
                        Ok(list) => {
                            state.rules = list.rules;
                            let mut imported_list = ExceptionList { rules: state.rules.clone(), ..Default::default() };
                            imported_list.save();
                            true
                        },
                        Err(_) => false,
                    };
                    context.data_mut(|data_map| {
                        data_map.insert_temp(egui::Id::new("exceptions_import_time"), current_time);
                        data_map.insert_temp(egui::Id::new("exceptions_import_result"), success);
                    });
                }

                let export_time = context.data(|data_map| data_map.get_temp::<f64>(egui::Id::new("exceptions_export_time"))).unwrap_or(-10.0);
                let export_result = context.data(|data_map| data_map.get_temp::<bool>(egui::Id::new("exceptions_export_result"))).unwrap_or(false);
                let (export_text, export_color) = if (current_time - export_time) < 2.0 {
                    if export_result { ("Exported!", success_color) } else { ("Failed!", fail_color) }
                } else { ("Export List", default_color) };

                if horizontal_ui.add_sized([button_width, button_height], egui::Button::new(egui::RichText::new(export_text).size(12.0).strong().color(egui::Color32::WHITE)).fill(export_color).rounding(4.0)).clicked() {
                    let export_directory = Path::new("exports");
                    let _ = fs::create_dir_all(export_directory);
                    let mut export_list = ExceptionList { rules: state.rules.clone(), ..Default::default() };
                    let success = export_list.save_to_file(&export_directory.join("exceptions.json")).is_ok();
                    context.data_mut(|data_map| {
                        data_map.insert_temp(egui::Id::new("exceptions_export_time"), current_time);
                        data_map.insert_temp(egui::Id::new("exceptions_export_result"), success);
                    });
                }

                if horizontal_ui.add_sized([button_width, button_height], egui::Button::new(egui::RichText::new("Reset to Default").size(12.0).strong().color(egui::Color32::WHITE)).fill(danger_color).rounding(4.0)).clicked() {
                    context.data_mut(|data_map| data_map.insert_temp(egui::Id::new("reset_rules_modal"), ResetConfirmState { is_open: true, reset_position: true }));
                }
            });
        });

        ui_container.add_space(15.0);
        ui_container.separator();
        ui_container.add_space(5.0);

        egui::ScrollArea::vertical().max_height(WINDOW_MAX_HEIGHT).auto_shrink([true, true]).show(ui_container, |scroll_ui| {
            egui::Grid::new("exceptions_grid").striped(true).spacing(egui::vec2(15.0, 10.0)).show(scroll_ui, |grid_ui| {
                grid_ui.vertical_centered(|column_ui| { column_ui.set_min_width(COLUMN_PATTERN_WIDTH); column_ui.label(egui::RichText::new("Stem").strong()); });
                grid_ui.vertical_centered(|column_ui| { column_ui.set_min_width(COLUMN_EXTENSION_WIDTH); column_ui.label(egui::RichText::new("Extension").strong()); });
                grid_ui.vertical_centered(|column_ui| { column_ui.set_min_width(COLUMN_HANDLING_WIDTH); column_ui.label(egui::RichText::new("Handling").strong()); });
                grid_ui.vertical_centered(|column_ui| { column_ui.set_min_width(COLUMN_LANGUAGE_WIDTH); column_ui.label(egui::RichText::new("Languages").strong()); });
                grid_ui.vertical_centered(|column_ui| { column_ui.set_min_width(COLUMN_ACTION_WIDTH); column_ui.label(egui::RichText::new("Actions").strong()); });
                grid_ui.end_row();

                let mut row_to_delete = None;
                for (index, rule) in state.rules.iter_mut().enumerate() {
                    grid_ui.add(egui::TextEdit::singleline(&mut rule.pattern).desired_width(COLUMN_PATTERN_WIDTH));
                    grid_ui.add(egui::TextEdit::singleline(&mut rule.extension).desired_width(COLUMN_EXTENSION_WIDTH));

                    grid_ui.horizontal(|inner_horizontal_ui| {
                        inner_horizontal_ui.set_min_width(COLUMN_HANDLING_WIDTH);
                        let combo_width = 90.0;
                        let x_offset = (COLUMN_HANDLING_WIDTH - combo_width) / 2.0;
                        inner_horizontal_ui.add_space(x_offset);

                        egui::ComboBox::from_id_salt(format!("handling_{}", index))
                            .width(combo_width)
                            .selected_text(rule.handling.to_string())
                            .show_ui(inner_horizontal_ui, |combo_ui| {
                                for option in RuleHandling::all() {
                                    combo_ui.selectable_value(&mut rule.handling, option, option.to_string());
                                }
                            });
                    });

                    grid_ui.vertical_centered(|inner_vertical_ui| {
                        let active_count = rule.languages.values().filter(|&&is_enabled| is_enabled).count();
                        inner_vertical_ui.menu_button(format!("Manage ({})", active_count), |menu_ui| {
                            egui::Grid::new(format!("lang_popup_grid_{}", index)).num_columns(2).spacing(egui::vec2(10.0, 5.0)).show(menu_ui, |menu_grid_ui| {
                                for &(language_code, _) in core::global::io::patterns::APP_LANGUAGES {
                                    if let Some(is_enabled) = rule.languages.get_mut(language_code) {
                                        menu_grid_ui.label(language_code.to_uppercase());
                                        toggle_ui(menu_grid_ui, is_enabled);
                                        menu_grid_ui.end_row();
                                    }
                                }
                            });
                        });
                    });

                    grid_ui.vertical_centered(|inner_vertical_ui| {
                        if inner_vertical_ui.button("🗑").on_hover_text("Delete Rule").clicked() { row_to_delete = Some(index); }
                    });
                    grid_ui.end_row();
                }

                if let Some(target_index) = row_to_delete { state.rules.remove(target_index); }
            });
        });
    });

    if show_reset_confirm_modal(context, drag_guard) {
        state.rules = ExceptionList::default().rules;
        let mut reset_list = ExceptionList { rules: state.rules.clone(), ..Default::default() };
        reset_list.save();
    }

    if state.rules != original_rules || (state.is_open && !is_open) {
        let mut final_list = ExceptionList { rules: state.rules.clone(), ..Default::default() };
        final_list.save();
    }

    state.is_open = is_open;
    context.data_mut(|data_map| data_map.insert_temp(state_id, state));
}