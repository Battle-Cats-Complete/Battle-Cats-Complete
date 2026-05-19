use std::fs;
use std::sync::{Arc, Mutex, mpsc};
use eframe::egui;
use crate::features::settings::logic::pem;
use crate::global::ui::shared::DragGuard;

#[derive(Clone)]
struct ManagePemState {
    is_open: bool,
    active_pem: String,
    is_custom: bool,

    generate_click_time: f64,
    confirm_generate: bool,
    is_generating: bool,

    delete_click_time: f64,
    confirm_delete: bool,

    export_time: f64,
    export_res: bool,

    receiver: Option<Arc<Mutex<mpsc::Receiver<Option<String>>>>>,
}

impl Default for ManagePemState {
    fn default() -> Self {
        let (active_pem, is_custom) = pem::get_active_pem();
        Self {
            is_open: false,
            active_pem,
            is_custom,
            generate_click_time: -10.0,
            confirm_generate: false,
            is_generating: false,
            delete_click_time: -10.0,
            confirm_delete: false,
            export_time: -10.0,
            export_res: false,
            receiver: None,
        }
    }
}

pub fn open(context: &egui::Context) {
    let state_id = egui::Id::new("manage_pem_state_v3");
    let mut state = context.data(|data_map| data_map.get_temp::<ManagePemState>(state_id)).unwrap_or_default();
    state.is_open = true;
    context.data_mut(|data_map| data_map.insert_temp(state_id, state));
}

pub fn show(context: &egui::Context, drag_guard: &mut DragGuard) {
    let state_id = egui::Id::new("manage_pem_state_v3");
    let mut state = context.data(|data_map| data_map.get_temp::<ManagePemState>(state_id)).unwrap_or_default();

    if !state.is_open {
        state.confirm_generate = false;
        state.confirm_delete = false;
        context.data_mut(|data_map| data_map.insert_temp(state_id, state));
        return;
    }

    let window_id = egui::Id::new("manage_pem_window_v3");
    let (allow_drag, fixed_position) = drag_guard.assign_bounds(context, window_id);
    let mut is_open = state.is_open;

    let mut thread_finished = false;
    let mut generated_pem_result = None;

    if let Some(receiver_arc) = &state.receiver {
        if let Ok(receiver_guard) = receiver_arc.try_lock() {
            if let Ok(received_data) = receiver_guard.try_recv() {
                generated_pem_result = received_data;
                thread_finished = true;
            }
        }
    }

    if thread_finished {
        if let Some(new_pem) = generated_pem_result {
            let _ = pem::save_pem(&new_pem);
            state.active_pem = new_pem;
            state.is_custom = true;
        }
        state.is_generating = false;
        state.receiver = None;
    }

    let mut window = egui::Window::new("Manage PEM")
        .id(window_id)
        .open(&mut is_open)
        .collapsible(false)
        .resizable(false)
        .constrain(false)
        .movable(allow_drag)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(context.screen_rect().center());

    if let Some(position) = fixed_position {
        window = window.current_pos(position);
    }

    window.show(context, |ui_container| {
        ui_container.set_min_width(550.0);
        ui_container.add_space(10.0);

        let button_height = 24.0;
        let button_width = 110.0;
        let default_blue = egui::Color32::from_rgb(31, 106, 165);
        let success_green = egui::Color32::from_rgb(40, 160, 60);
        let warning_yellow = egui::Color32::from_rgb(200, 180, 50);
        let danger_red = egui::Color32::from_rgb(180, 50, 50);
        let current_time = ui_container.input(|input_state| input_state.time);

        ui_container.vertical_centered(|centered_ui| {
            centered_ui.horizontal(|ui_row| {
                let spacing = 10.0;
                ui_row.spacing_mut().item_spacing.x = spacing;

                let total_buttons_width = (button_width * 4.0) + (spacing * 3.0);
                let x_offset = (ui_row.available_width() - total_buttons_width) / 2.0;
                ui_row.add_space(x_offset.max(0.0));

                // --- Import PEM ---
                let import_button = egui::Button::new(
                    egui::RichText::new("Import PEM").size(12.0).strong().color(egui::Color32::WHITE)
                ).fill(default_blue).rounding(4.0);

                ui_row.add_enabled_ui(!state.is_generating, |enabled_ui| {
                    if enabled_ui.add_sized([button_width, button_height], import_button).clicked() {
                        if let Some(file_path) = rfd::FileDialog::new().add_filter("PEM", &["pem", "txt"]).pick_file() {
                            if let Ok(content) = fs::read_to_string(&file_path) {
                                if content.contains("-----BEGIN PRIVATE KEY-----") && content.contains("-----BEGIN CERTIFICATE-----") {
                                    let _ = pem::save_pem(&content);
                                    state.active_pem = content;
                                    state.is_custom = true;
                                    state.confirm_generate = false;
                                    state.confirm_delete = false;
                                }
                            }
                        }
                    }
                });

                // --- Export PEM ---
                let (export_text, export_color) = if (current_time - state.export_time) < 2.0 {
                    ui_row.ctx().request_repaint();
                    if state.export_res { ("Exported!", success_green) } else { ("Failed!", danger_red) }
                } else {
                    ("Export PEM", default_blue)
                };

                let export_button = egui::Button::new(
                    egui::RichText::new(export_text).size(12.0).strong().color(egui::Color32::WHITE)
                ).fill(export_color).rounding(4.0);

                ui_row.add_enabled_ui(!state.is_generating, |enabled_ui| {
                    if enabled_ui.add_sized([button_width, button_height], export_button).clicked() {
                        let export_dir = std::path::Path::new("exports");
                        let _ = fs::create_dir_all(export_dir);

                        let filename = if state.is_custom { "identity.pem" } else { "bcc.pem" };
                        let export_path = export_dir.join(filename);

                        let success = fs::write(export_path, &state.active_pem).is_ok();
                        state.export_time = current_time;
                        state.export_res = success;
                    }
                });

                // --- Generate PEM ---
                if state.confirm_generate {
                    if current_time - state.generate_click_time > 2.0 {
                        state.confirm_generate = false;
                    } else {
                        ui_row.ctx().request_repaint();
                    }
                }

                let generate_text = if state.is_generating {
                    "Generating..."
                } else if state.confirm_generate {
                    "Are You Sure?"
                } else {
                    "Generate PEM"
                };

                let generate_button = egui::Button::new(
                    egui::RichText::new(generate_text).size(12.0).strong().color(egui::Color32::WHITE)
                ).fill(warning_yellow).rounding(4.0);

                ui_row.add_enabled_ui(!state.is_generating, |enabled_ui| {
                    if enabled_ui.add_sized([button_width, button_height], generate_button).clicked() {
                        if state.is_custom && !state.confirm_generate {
                            state.confirm_generate = true;
                            state.generate_click_time = current_time;
                            state.confirm_delete = false;
                        } else {
                            state.confirm_generate = false;
                            state.is_generating = true;

                            let (transmitter, receiver) = mpsc::channel();
                            state.receiver = Some(Arc::new(Mutex::new(receiver)));
                            let context_clone = context.clone();

                            std::thread::spawn(move || {
                                let result = pem::generate_pem().ok();
                                let _ = transmitter.send(result);
                                context_clone.request_repaint();
                            });
                        }
                    }
                });

                // --- Delete PEM ---
                if state.confirm_delete {
                    if current_time - state.delete_click_time > 2.0 {
                        state.confirm_delete = false;
                    } else {
                        ui_row.ctx().request_repaint();
                    }
                }

                let delete_text = if state.confirm_delete { "Are You Sure?" } else { "Delete PEM" };
                let delete_button = egui::Button::new(
                    egui::RichText::new(delete_text).size(12.0).strong().color(egui::Color32::WHITE)
                ).fill(danger_red).rounding(4.0);

                ui_row.add_enabled_ui(state.is_custom && !state.is_generating, |enabled_ui| {
                    if enabled_ui.add_sized([button_width, button_height], delete_button).clicked() {
                        if !state.confirm_delete {
                            state.confirm_delete = true;
                            state.delete_click_time = current_time;
                            state.confirm_generate = false;
                        } else {
                            pem::delete_pem();
                            let (default_pem, _) = pem::get_active_pem();
                            state.active_pem = default_pem;
                            state.is_custom = false;
                            state.confirm_delete = false;
                        }
                    }
                });
            });
        });

        ui_container.add_space(15.0);
        ui_container.separator();
        ui_container.add_space(5.0);

        let text_color = if state.is_custom {
            ui_container.visuals().text_color()
        } else {
            egui::Color32::from_gray(100)
        };

        egui::ScrollArea::vertical().max_height(350.0).show(ui_container, |scroll_ui| {
            let mut read_only_text = state.active_pem.as_str();

            scroll_ui.add(
                egui::TextEdit::multiline(&mut read_only_text)
                    .font(egui::TextStyle::Monospace)
                    .text_color(text_color)
                    .desired_width(f32::INFINITY)
                    .interactive(false)
            );
        });
    });

    state.is_open = is_open;
    context.data_mut(|data_map| data_map.insert_temp(state_id, state));
}