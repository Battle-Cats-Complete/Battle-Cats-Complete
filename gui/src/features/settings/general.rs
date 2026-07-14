use std::sync::atomic::Ordering;

use eframe::egui;

use core::settings::logic::state::{GeneralSettings, RuntimeState};
use core::settings::logic::{lang, nightly, upd::UpdateMode};
#[cfg(target_os = "linux")]
use core::settings::logic::desktop;

use super::tabs::toggle_ui;

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, PartialEq)]
#[derive(Default)]
enum DesktopActionState {
    #[default]
    None,
    Created,
    Deleted,
    Failed,
}

pub fn show(ui_container: &mut egui::Ui, settings: &mut GeneralSettings, runtime: &mut RuntimeState) -> bool {
    let mut refresh_needed = false;
    let context = ui_container.ctx().clone();

    lang::ensure_complete_list(&mut settings.language_priority);

    egui::ScrollArea::vertical()
        .id_salt("general_scroll")
        .auto_shrink([false, true])
        .show(ui_container, |scroll_ui| {

            // --- SYSTEM SECTION ---
            scroll_ui.heading("System");
            scroll_ui.add_space(5.0);

            #[cfg(target_os = "linux")]
            {
                let is_installed = desktop::is_desktop_data_present();
                let current_time = scroll_ui.input(|input_state| input_state.time);

                let action_time = context.data(|data_map| data_map.get_temp::<f64>(egui::Id::new("desktop_action_time"))).unwrap_or(-10.0);
                let action_state = context.data(|data_map| data_map.get_temp::<DesktopActionState>(egui::Id::new("desktop_action_state"))).unwrap_or_default();

                let show_status = (current_time - action_time) < 2.0;

                let (button_text, button_color, is_delete_action) = if show_status {
                    scroll_ui.ctx().request_repaint();
                    match action_state {
                        DesktopActionState::Created => ("Desktop Data Created!", egui::Color32::from_rgb(40, 160, 40), true),
                        DesktopActionState::Deleted => ("Desktop Data Deleted!", egui::Color32::from_rgb(40, 160, 40), false),
                        DesktopActionState::Failed => ("Failed to Create Data!", egui::Color32::from_rgb(180, 50, 50), false),
                        DesktopActionState::None => if is_installed {
                            ("Delete Desktop Data", egui::Color32::from_rgb(180, 50, 50), true)
                        } else {
                            ("Create Desktop Data", egui::Color32::from_rgb(40, 90, 160), false)
                        }
                    }
                } else {
                    if is_installed {
                        ("Delete Desktop Data", egui::Color32::from_rgb(180, 50, 50), true)
                    } else {
                        ("Create Desktop Data", egui::Color32::from_rgb(40, 90, 160), false)
                    }
                };

                let desktop_button = egui::Button::new(button_text).fill(button_color);

                if scroll_ui.add_sized([180.0, 30.0], desktop_button).clicked() {
                    if is_delete_action {
                        let success = desktop::delete_desktop_data().is_ok();
                        context.data_mut(|data_map| {
                            data_map.insert_temp(egui::Id::new("desktop_action_time"), current_time);
                            data_map.insert_temp(egui::Id::new("desktop_action_state"), if success { DesktopActionState::Deleted } else { DesktopActionState::Failed });
                        });
                    } else {
                        let success = desktop::create_desktop_data().is_ok();
                        context.data_mut(|data_map| {
                            data_map.insert_temp(egui::Id::new("desktop_action_time"), current_time);
                            data_map.insert_temp(egui::Id::new("desktop_action_state"), if success { DesktopActionState::Created } else { DesktopActionState::Failed });
                        });
                    }
                }

                scroll_ui.add_space(5.0);
            }

            let updater_status = context.data(|data_map| data_map.get_temp::<&'static str>(egui::Id::new("updater_status")).unwrap_or("Idle"));

            match updater_status {
                "Checking" => {
                    let button_widget = egui::Button::new("Checking for Updates...").fill(egui::Color32::from_rgb(200, 180, 50));
                    scroll_ui.add_sized([180.0, 30.0], button_widget);
                },
                "UpToDate" => {
                    let button_widget = egui::Button::new("Up to Date!").fill(egui::Color32::from_rgb(40, 160, 40));
                    scroll_ui.add_sized([180.0, 30.0], button_widget);
                },
                "UpdateFound" => {
                    let button_widget = egui::Button::new("Update Found!").fill(egui::Color32::from_rgb(40, 160, 40));
                    scroll_ui.add_sized([180.0, 30.0], button_widget);
                },
                "CheckFailed" => {
                    let button_widget = egui::Button::new("Failed to Check!").fill(egui::Color32::from_rgb(180, 50, 50));
                    scroll_ui.add_sized([180.0, 30.0], button_widget);
                },
                "Downloading" => {
                    let button_widget = egui::Button::new("Downloading Update...").fill(egui::Color32::from_rgb(40, 90, 160));
                    scroll_ui.add_sized([180.0, 30.0], button_widget);
                },
                "RestartPending" => {
                    let button_widget = egui::Button::new("Restart Pending!").fill(egui::Color32::from_rgb(200, 180, 50));
                    scroll_ui.add_sized([180.0, 30.0], button_widget);
                },
                _ => {
                    if scroll_ui.add_sized([180.0, 30.0], egui::Button::new("Check for Update Now")).clicked() {
                        runtime.manual_check_requested = true;
                    }
                }
            }

            // --- BEHAVIOR SECTION ---
            scroll_ui.add_space(20.0);
            scroll_ui.heading("Behavior");
            scroll_ui.add_space(5.0);
            
            scroll_ui.horizontal(|horizontal_ui| {
                let toggle_resp = toggle_ui(horizontal_ui, &mut settings.enable_logging);
                let label_resp = horizontal_ui.label("Enable Logging");

                if toggle_resp.changed() {
                    refresh_needed = true;
                }

                let hint = "Enables logs for easy debugging\nDisable to improve performance\nDevs may refuse to debug without logs";
                toggle_resp.on_hover_text(hint);
                label_resp.on_hover_text(hint);
            });

            scroll_ui.add_space(5.0);

            let features_available = nightly::NIGHTLY_FEATURES_ACTIVE.load(Ordering::Relaxed);

            if !features_available {
                settings.enable_nightly = false;
            }

            let nightly_row = scroll_ui.horizontal(|horizontal_ui| {
                horizontal_ui.add_enabled_ui(features_available, |enabled_ui| {

                    let toggle_resp = toggle_ui(enabled_ui, &mut settings.enable_nightly);
                    let label_resp = enabled_ui.label("Enable Nightly Features 🌙");

                    if toggle_resp.changed() {
                        refresh_needed = true;
                    }

                    if features_available {
                        // START | Nightly
                        let hint = "Enables work in progress \"Nightly\" features\n\
                            Nightly features are signified using a crescent moon \"🌙\"\n\
                            Expect bugs and poor performance when using Nightly features";

                        toggle_resp.on_hover_text(hint);
                        label_resp.on_hover_text(hint);
                        //  END  | Nightly
                    }
                });
            }).response;

            if !features_available {
                nightly_row.on_hover_text(
                    egui::RichText::new("This app version contains no Nightly features")
                        .color(egui::Color32::from_rgb(230, 130, 10))
                );
            }

            scroll_ui.add_space(8.0);
            scroll_ui.horizontal(|horizontal_ui| {
                horizontal_ui.label("Update Handling:");

                egui::ComboBox::from_id_salt("update_mode_selector")
                    .selected_text(settings.update_mode.label())
                    .show_ui(horizontal_ui, |combo_ui| {
                        combo_ui.selectable_value(&mut settings.update_mode, UpdateMode::AutoReset, "Auto-Reset")
                            .on_hover_text("Automatically downloads updates and restarts the app on startup");
                        combo_ui.selectable_value(&mut settings.update_mode, UpdateMode::AutoLoad, "Auto-Load")
                            .on_hover_text("Automatically downloads updates but waits until the next run to apply them");
                        combo_ui.selectable_value(&mut settings.update_mode, UpdateMode::Prompt, "Prompt")
                            .on_hover_text("Ask permission before downloading updates or restarting");
                        combo_ui.selectable_value(&mut settings.update_mode, UpdateMode::Ignore, "Ignore")
                            .on_hover_text("Never check for updates on startup");
                    });
            });

            // --- LANGUAGE SECTION ---
            scroll_ui.add_space(20.0);
            scroll_ui.heading("Language");
            scroll_ui.add_space(5.0);

            scroll_ui.label("Drag to reorder. The app prioritizes assets from the top down.");
            scroll_ui.small("Languages below 'None' will never be loaded.");
            scroll_ui.add_space(5.0);

            if render_drag_list(scroll_ui, &mut settings.language_priority) {
                refresh_needed = true;
            }

            scroll_ui.add_space(10.0);
            if scroll_ui.button("Restore Defaults").clicked() {
                settings.language_priority = lang::default_priority();
                refresh_needed = true;
            }
        });

    refresh_needed
}

fn render_drag_list(ui_container: &mut egui::Ui, priority: &mut Vec<String>) -> bool {
    let id_source = egui::Id::new("language_priority_drag_list");

    let was_dragging = ui_container.ctx().data(|data_map| data_map.get_temp::<bool>(id_source)).unwrap_or(false);
    let is_dragging = ui_container.ctx().dragged_id().is_some();
    ui_container.ctx().data_mut(|data_map| data_map.insert_temp(id_source, is_dragging));

    let just_dropped = was_dragging && !is_dragging;

    let mut source_index = None;
    let mut target_index = None;
    let mut is_disabled_section = false;

    egui::Frame::group(ui_container.style()).show(ui_container, |frame_ui| {
        frame_ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0);

        for (index, language_code) in priority.clone().iter().enumerate() {
            let is_none = *language_code == "--";
            if is_none { is_disabled_section = true; }

            let item_id = id_source.with(language_code);
            let is_dragged = frame_ui.ctx().is_being_dragged(item_id);

            let mut inner_frame = egui::Frame::none().inner_margin(egui::vec2(5.0, 2.0));
            if is_dragged {
                inner_frame.fill = frame_ui.visuals().widgets.active.bg_fill;
                inner_frame.rounding = frame_ui.visuals().widgets.active.rounding;
            }

            let row_response = frame_ui.scope(|scope_ui| {
                if is_disabled_section && !is_none {
                    scope_ui.visuals_mut().override_text_color = Some(egui::Color32::from_gray(100));
                }

                inner_frame.show(scope_ui, |row_ui| {
                    row_ui.horizontal(|horizontal_ui| {
                        let label_response = horizontal_ui.label("☰");
                        let handle = horizontal_ui.interact(label_response.rect.expand(2.0), item_id, egui::Sense::drag());

                        if handle.hovered() { horizontal_ui.ctx().set_cursor_icon(egui::CursorIcon::Grab); }
                        if handle.dragged() {
                            horizontal_ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                            source_index = Some(index);
                        }

                        horizontal_ui.add_space(5.0);

                        if is_none {
                            horizontal_ui.strong(lang::get_label_for_code(language_code));
                        } else {
                            horizontal_ui.label(lang::get_label_for_code(language_code));
                        }
                    });
                }).response
            }).response;

            let drop_rect = row_response.rect.expand2(egui::vec2(100.0, 0.0));
            if let Some(mouse_position) = frame_ui.ctx().pointer_interact_pos()
                && drop_rect.contains(mouse_position) { target_index = Some(index); }
        }
    });

    if let (Some(source), Some(target)) = (source_index, target_index)
        && source != target {
        let item = priority.remove(source);
        priority.insert(target, item);
    }

    just_dropped
}