use std::fs;
use std::path::Path;
use eframe::egui;
use crate::features::settings::logic::keys::UserKeys;
use crate::global::ui::shared::DragGuard;

const COL_REGION_WIDTH: f32 = 40.0;
const COL_INPUT_WIDTH: f32 = 250.0;

#[derive(Clone, Default)]
struct ManageKeysState {
    is_open: bool,
    keys: UserKeys,
    validation_status: Option<[(bool, bool); 4]>,
}

pub fn open(context: &egui::Context) {
    let state_id = egui::Id::new("manage_keys_state");
    let mut state = context.data(|data_map| data_map.get_temp::<ManageKeysState>(state_id)).unwrap_or_else(|| {
        ManageKeysState {
            is_open: false,
            keys: UserKeys::load(),
            validation_status: None,
        }
    });
    state.is_open = true;
    context.data_mut(|data_map| data_map.insert_temp(state_id, state));
}

pub fn show(context: &egui::Context, drag_guard: &mut DragGuard) {
    let state_id = egui::Id::new("manage_keys_state");
    let mut state = context.data(|data_map| data_map.get_temp::<ManageKeysState>(state_id)).unwrap_or_else(|| {
        ManageKeysState {
            is_open: false,
            keys: UserKeys::load(),
            validation_status: None,
        }
    });

    if !state.is_open {
        state.validation_status = None;
        context.data_mut(|data_map| data_map.insert_temp(state_id, state));
        return;
    }

    let window_id = egui::Id::new("manage_keys_window");
    let (allow_drag, fixed_position) = drag_guard.assign_bounds(context, window_id);
    let original_keys = state.keys.clone();
    let mut is_open = state.is_open;

    let mut window = egui::Window::new("Manage Decryption Keys")
        .id(window_id)
        .open(&mut is_open)
        .collapsible(false)
        .resizable(false)
        .constrain(false)
        .movable(allow_drag)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(context.screen_rect().center());

    if let Some(position) = fixed_position { window = window.current_pos(position); }

    window.show(context, |ui_container| {
        ui_container.add_space(10.0);

        let button_height = 24.0;
        let button_width = 110.0;
        let default_color = egui::Color32::from_rgb(31, 106, 165);
        let success_color = egui::Color32::from_rgb(40, 160, 60);
        let fail_color = egui::Color32::from_rgb(200, 40, 40);
        let danger_color = egui::Color32::from_rgb(180, 50, 50);
        let current_time = ui_container.input(|input_state| input_state.time);

        ui_container.vertical_centered(|centered_ui| {
            centered_ui.horizontal(|ui_row| {
                let table_width = COL_REGION_WIDTH + (COL_INPUT_WIDTH * 2.0) + (15.0 * 2.0);
                let spacing = ui_row.spacing().item_spacing.x;
                let total_button_width = (button_width * 4.0) + (spacing * 3.0);

                let x_offset = (table_width - total_button_width) / 2.0;
                ui_row.add_space(x_offset.max(0.0));

                // --- Load Keys ---
                let import_time = context.data(|data_map| data_map.get_temp::<f64>(egui::Id::new("keys_import_time"))).unwrap_or(-10.0);
                let import_result = context.data(|data_map| data_map.get_temp::<bool>(egui::Id::new("keys_import_res"))).unwrap_or(false);

                let (import_text, import_color) = if (current_time - import_time) < 2.0 {
                    ui_row.ctx().request_repaint();
                    if import_result { ("Loaded!", success_color) } else { ("Failed!", fail_color) }
                } else { ("Load Keys", default_color) };

                if ui_row.add_sized([button_width, button_height], egui::Button::new(egui::RichText::new(import_text).size(12.0).strong().color(egui::Color32::WHITE)).fill(import_color).rounding(4.0)).clicked() {
                    if let Some(file_path) = rfd::FileDialog::new().pick_file() {
                        let success = match fs::read_to_string(&file_path) {
                            Ok(file_data) => {
                                if let Ok(parsed_keys) = serde_json::from_str::<UserKeys>(&file_data) {
                                    state.keys = parsed_keys;
                                    state.keys.save();
                                    state.validation_status = None;
                                    true
                                } else { false }
                            },
                            Err(_) => false,
                        };
                        context.data_mut(|data_map| {
                            data_map.insert_temp(egui::Id::new("keys_import_time"), current_time);
                            data_map.insert_temp(egui::Id::new("keys_import_res"), success);
                        });
                    }
                }

                // --- Export Keys ---
                let export_time = context.data(|data_map| data_map.get_temp::<f64>(egui::Id::new("keys_export_time"))).unwrap_or(-10.0);
                let export_result = context.data(|data_map| data_map.get_temp::<bool>(egui::Id::new("keys_export_res"))).unwrap_or(false);

                let (export_text, export_color) = if (current_time - export_time) < 2.0 {
                    ui_row.ctx().request_repaint();
                    if export_result { ("Exported!", success_color) } else { ("Failed!", fail_color) }
                } else { ("Export Keys", default_color) };

                if ui_row.add_sized([button_width, button_height], egui::Button::new(egui::RichText::new(export_text).size(12.0).strong().color(egui::Color32::WHITE)).fill(export_color).rounding(4.0)).clicked() {
                    let export_dir = Path::new("exports");
                    let _ = fs::create_dir_all(export_dir);

                    let export_path = export_dir.join("keys");
                    let json_data = serde_json::to_string_pretty(&state.keys).unwrap_or_default();
                    let success = fs::write(&export_path, json_data).is_ok();

                    context.data_mut(|data_map| {
                        data_map.insert_temp(egui::Id::new("keys_export_time"), current_time);
                        data_map.insert_temp(egui::Id::new("keys_export_res"), success);
                    });
                }

                // --- Validate Keys ---
                if ui_row.add_sized([button_width, button_height], egui::Button::new(egui::RichText::new("Validate Keys").size(12.0).strong().color(egui::Color32::WHITE)).fill(default_color).rounding(4.0)).clicked() {
                    state.validation_status = Some(state.keys.validate());
                }

                // --- Delete Keys ---
                let delete_time = context.data(|data_map| data_map.get_temp::<f64>(egui::Id::new("keys_delete_time"))).unwrap_or(-10.0);
                let mut is_confirming_delete = context.data(|data_map| data_map.get_temp::<bool>(egui::Id::new("keys_confirm_delete"))).unwrap_or(false);

                if is_confirming_delete {
                    if current_time - delete_time > 2.0 {
                        is_confirming_delete = false;
                        context.data_mut(|data_map| data_map.insert_temp(egui::Id::new("keys_confirm_delete"), false));
                    } else {
                        ui_row.ctx().request_repaint();
                    }
                }

                let delete_text = if is_confirming_delete { "Are You Sure?" } else { "Delete Keys" };

                if ui_row.add_sized([button_width, button_height], egui::Button::new(egui::RichText::new(delete_text).size(12.0).strong().color(egui::Color32::WHITE)).fill(danger_color).rounding(4.0)).clicked() {
                    if !is_confirming_delete {
                        context.data_mut(|data_map| {
                            data_map.insert_temp(egui::Id::new("keys_confirm_delete"), true);
                            data_map.insert_temp(egui::Id::new("keys_delete_time"), current_time);
                        });
                    } else {
                        state.keys = UserKeys::default();
                        state.validation_status = None;
                        state.keys.save();
                        context.data_mut(|data_map| data_map.insert_temp(egui::Id::new("keys_confirm_delete"), false));
                    }
                }
            });
        });

        ui_container.add_space(15.0);
        ui_container.separator();
        ui_container.add_space(5.0);

        egui::Grid::new("keys_grid").striped(true).spacing(egui::vec2(15.0, 10.0)).show(ui_container, |grid_ui| {
            grid_ui.vertical_centered(|col_ui| { col_ui.set_min_width(COL_REGION_WIDTH); col_ui.label(egui::RichText::new("Region").strong()); });
            grid_ui.vertical_centered(|col_ui| { col_ui.set_min_width(COL_INPUT_WIDTH); col_ui.label(egui::RichText::new("Decryption Key").strong()); });
            grid_ui.vertical_centered(|col_ui| { col_ui.set_min_width(COL_INPUT_WIDTH); col_ui.label(egui::RichText::new("Initialization Vector").strong()); });
            grid_ui.end_row();

            let mut regions = [
                ("Japan", &mut state.keys.ja),
                ("Global", &mut state.keys.en),
                ("Taiwan", &mut state.keys.tw),
                ("Korea", &mut state.keys.ko),
            ];

            let default_validations = [(true, true); 4];
            let current_validations = state.validation_status.unwrap_or(default_validations);

            for (index, (name, region_data)) in regions.iter_mut().enumerate() {
                grid_ui.centered_and_justified(|col_ui| { col_ui.label(egui::RichText::new(*name).strong()); });

                let (key_valid, iv_valid) = current_validations[index];

                grid_ui.scope(|scope_ui| {
                    if state.validation_status.is_some() {
                        let color = if key_valid { egui::Color32::from_rgb(30, 80, 40) } else { egui::Color32::from_rgb(120, 30, 30) };
                        scope_ui.visuals_mut().extreme_bg_color = color;
                    }
                    if scope_ui.add(egui::TextEdit::singleline(&mut region_data.key).desired_width(COL_INPUT_WIDTH)).changed() {
                        state.validation_status = None;
                    }
                });

                grid_ui.scope(|scope_ui| {
                    if state.validation_status.is_some() {
                        let color = if iv_valid { egui::Color32::from_rgb(30, 80, 40) } else { egui::Color32::from_rgb(120, 30, 30) };
                        scope_ui.visuals_mut().extreme_bg_color = color;
                    }
                    if scope_ui.add(egui::TextEdit::singleline(&mut region_data.iv).desired_width(COL_INPUT_WIDTH)).changed() {
                        state.validation_status = None;
                    }
                });

                grid_ui.end_row();
            }
        })
    });

    if state.keys != original_keys || (state.is_open && !is_open) {
        state.keys.save();
    }

    if !is_open { state.validation_status = None; }

    state.is_open = is_open;
    context.data_mut(|data_map| data_map.insert_temp(state_id, state));
}