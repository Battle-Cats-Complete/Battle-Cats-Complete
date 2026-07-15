use std::fs;
use std::path::Path;

use eframe::egui;

use core::settings::logic::delete::FolderDeleter;
use core::settings::logic::state::{GameDataSettings, RuntimeState};

use crate::global::shared::DragGuard;

use super::tabs::toggle_ui;

#[derive(Clone, Default)]
struct FolderDeleteState {
    is_open: bool,
    reset_position: bool,
    size_str: Option<String>,
}

fn get_folder_size(path: &Path) -> u64 {
    let mut size_accumulator = 0;
    if let Ok(directory_entries) = fs::read_dir(path) {
        for entry in directory_entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    size_accumulator += get_folder_size(&entry.path());
                } else {
                    size_accumulator += metadata.len();
                }
            }
        }
    }
    size_accumulator
}

fn format_size(size: u64) -> String {
    let kilobyte = 1024.0;
    let megabyte = kilobyte * 1024.0;
    let gigabyte = megabyte * 1024.0;
    let size_float = size as f64;

    if size_float >= gigabyte {
        format!("{:.2} GB", size_float / gigabyte)
    } else if size_float >= megabyte {
        format!("{:.2} MB", size_float / megabyte)
    } else if size_float >= kilobyte {
        format!("{:.2} KB", size_float / kilobyte)
    } else {
        format!("{} B", size)
    }
}

fn show_folder_delete_modal(
    context: &egui::Context,
    drag_guard: &mut DragGuard,
    identifier_string: &str,
    content: &str,
) -> bool {
    let state_id = egui::Id::new(identifier_string);
    let mut state = context.data(|data_map| data_map.get_temp::<FolderDeleteState>(state_id)).unwrap_or_default();
    let mut yes_clicked = false;

    if state.is_open {
        let window_id = egui::Id::new(format!("{}_window", identifier_string));
        let (allow_drag, fixed_position) = drag_guard.assign_bounds(context, window_id);
        let mut should_close = false;

        let mut window = egui::Window::new("Confirm Deletion")
            .id(window_id)
            .collapsible(false)
            .resizable(false)
            .constrain(false)
            .movable(allow_drag)
            .default_pos(context.screen_rect().center() - egui::vec2(150.0, 50.0));

        if state.reset_position {
            window = window.current_pos(context.screen_rect().center() - egui::vec2(150.0, 50.0));
            state.reset_position = false;
        } else if let Some(position) = fixed_position {
            window = window.current_pos(position);
        }

        window.show(context, |ui_container| {
            ui_container.set_min_width(280.0);
            ui_container.vertical_centered(|centered_ui| {
                centered_ui.add_space(5.0);
                centered_ui.label(content);

                if let Some(folder_size) = &state.size_str {
                    centered_ui.add_space(5.0);
                    centered_ui.label(egui::RichText::new(format!("Folder size: {}", folder_size)).color(centered_ui.visuals().weak_text_color()));
                }

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

        if should_close {
            state.is_open = false;
        }

        context.data_mut(|data_map| data_map.insert_temp(state_id, state));
    }

    yes_clicked
}

pub fn show(ui_container: &mut egui::Ui, settings: &mut GameDataSettings, runtime: &mut RuntimeState, drag_guard: &mut DragGuard) -> bool {
    let mut refresh_needed = false;
    let context = ui_container.ctx().clone();

    let mut game_deleter = context.data_mut(|data_map| data_map.get_temp::<FolderDeleter>(egui::Id::new("game_deleter")).unwrap_or_default());
    let mut raw_deleter = context.data_mut(|data_map| data_map.get_temp::<FolderDeleter>(egui::Id::new("raw_deleter")).unwrap_or_default());
    let mut cache_deleter = context.data_mut(|data_map| data_map.get_temp::<FolderDeleter>(egui::Id::new("cache_deleter")).unwrap_or_default());

    game_deleter.update();
    raw_deleter.update();
    cache_deleter.update();

    if game_deleter.is_active() || raw_deleter.is_active() || cache_deleter.is_active() {
        context.request_repaint();
    }

    let game_exists = Path::new("game").exists();
    let raw_exists = Path::new("game/raw").exists();

    let cache_directory_optional = core::global::io::cache::get_cache_dir();
    let cache_size = cache_directory_optional.as_ref().map(|path| get_folder_size(path)).unwrap_or(0);
    let cache_exists = cache_size > 0;

    egui::ScrollArea::vertical()
        .id_salt("game_data_scroll")
        .auto_shrink([false, true])
        .show(ui_container, |scroll_ui| {
            scroll_ui.heading("Disk");
            scroll_ui.add_space(5.0);

            if game_deleter.is_deleting() {
                let button_widget = egui::Button::new("Deleting \"game\" Folder...")
                    .fill(egui::Color32::from_rgb(200, 180, 50));
                scroll_ui.add_sized([180.0, 30.0], button_widget);
            } else if game_deleter.is_done() {
                let button_widget = egui::Button::new("Deleted \"game\" Folder!")
                    .fill(egui::Color32::from_rgb(40, 160, 40));
                scroll_ui.add_sized([180.0, 30.0], button_widget);
            } else if game_exists {
                let button_widget = egui::Button::new("Delete \"game\" Folder")
                    .fill(egui::Color32::from_rgb(180, 50, 50));
                if scroll_ui.add_sized([180.0, 30.0], button_widget).clicked() {
                    let state_id = egui::Id::new("delete_game_modal");
                    context.data_mut(|data_map| data_map.insert_temp(state_id, FolderDeleteState { is_open: true, reset_position: true, size_str: None }));
                }
            } else {
                let button_widget = egui::Button::new("No \"game\" Folder")
                    .fill(egui::Color32::from_rgb(60, 60, 60));
                scroll_ui.add_sized([180.0, 30.0], button_widget);
            }

            scroll_ui.add_space(5.0);

            if game_deleter.is_deleting() {
                let button_widget = egui::Button::new("No \"raw\" Folder")
                    .fill(egui::Color32::from_rgb(60, 60, 60));
                scroll_ui.add_sized([180.0, 30.0], button_widget);
            } else if raw_deleter.is_deleting() {
                let button_widget = egui::Button::new("Deleting \"raw\" Folder...")
                    .fill(egui::Color32::from_rgb(200, 180, 50));
                scroll_ui.add_sized([180.0, 30.0], button_widget);
            } else if raw_deleter.is_done() {
                let button_widget = egui::Button::new("Deleted \"raw\" Folder!")
                    .fill(egui::Color32::from_rgb(40, 160, 40));
                scroll_ui.add_sized([180.0, 30.0], button_widget);
            } else if raw_exists {
                let button_widget = egui::Button::new("Delete \"raw\" Folder")
                    .fill(egui::Color32::from_rgb(180, 50, 50));
                if scroll_ui.add_sized([180.0, 30.0], button_widget).clicked() {
                    let raw_folder_size = get_folder_size(Path::new("game/raw"));
                    let state_id = egui::Id::new("delete_raw_modal");
                    context.data_mut(|data_map| data_map.insert_temp(state_id, FolderDeleteState {
                        is_open: true,
                        reset_position: true,
                        size_str: Some(format_size(raw_folder_size))
                    }));
                }
            } else {
                let button_widget = egui::Button::new("No \"raw\" Folder")
                    .fill(egui::Color32::from_rgb(60, 60, 60));
                scroll_ui.add_sized([180.0, 30.0], button_widget);
            }

            scroll_ui.add_space(5.0);

            if cache_deleter.is_deleting() {
                let button_widget = egui::Button::new("Clearing Cache...")
                    .fill(egui::Color32::from_rgb(200, 180, 50));
                scroll_ui.add_sized([180.0, 30.0], button_widget);
            } else if cache_deleter.is_done() {
                let button_widget = egui::Button::new("Cleared Cache!")
                    .fill(egui::Color32::from_rgb(40, 160, 40));
                scroll_ui.add_sized([180.0, 30.0], button_widget);
            } else if cache_exists {
                let button_widget = egui::Button::new("Clear Cache")
                    .fill(egui::Color32::from_rgb(180, 50, 50));
                if scroll_ui.add_sized([180.0, 30.0], button_widget).clicked() {
                    let state_id = egui::Id::new("delete_cache_modal");
                    context.data_mut(|data_map| data_map.insert_temp(state_id, FolderDeleteState {
                        is_open: true,
                        reset_position: true,
                        size_str: Some(format_size(cache_size))
                    }));
                }
            } else {
                let button_widget = egui::Button::new("Cache Empty")
                    .fill(egui::Color32::from_rgb(60, 60, 60));
                scroll_ui.add_sized([180.0, 30.0], button_widget);
            }
            scroll_ui.add_space(20.0);
            scroll_ui.heading("Management");
            scroll_ui.add_space(5.0);

            let keys_button = egui::Button::new("Manage Keys")
                .fill(egui::Color32::from_rgb(40, 90, 160));
            if scroll_ui.add_sized([180.0, 30.0], keys_button).clicked() {
                crate::features::settings::keys::open(&context);
            }

            scroll_ui.add_space(5.0);

            let import_button = egui::Button::new("Manage Exceptions")
                .fill(egui::Color32::from_rgb(40, 90, 160));
            if scroll_ui.add_sized([180.0, 30.0], import_button).clicked() {
                crate::features::settings::exceptions::open(&context);
            }

            scroll_ui.add_space(10.0);

            scroll_ui.horizontal(|horizontal_ui| {
                let label_response = horizontal_ui.label("Enforce Key Validation");
                let tooltip_text = "Prevents decryption/encryption if the cryptographic keys don't match the known official file hashes\nTurn this off only if the game keys have changed and you haven't updated BCC yet";
                label_response.on_hover_text(tooltip_text);

                let toggle_response = toggle_ui(horizontal_ui, &mut settings.enforce_key_validation).on_hover_text(tooltip_text);
                if toggle_response.changed() { refresh_needed = true; }
            });

            scroll_ui.add_space(5.0);

            scroll_ui.horizontal(|horizontal_ui| {
                let label_response = horizontal_ui.label("Enable Ultra Compression");
                let tooltip_text = "Allows compression levels up to 21\nWARNING: Levels above 15 require significant RAM and time";
                label_response.on_hover_text(tooltip_text);

                let toggle_response = toggle_ui(horizontal_ui, &mut settings.enable_ultra_compression).on_hover_text(tooltip_text);

                if toggle_response.changed() {
                    refresh_needed = true;
                    if !settings.enable_ultra_compression && settings.last_compression_level > 15 {
                        settings.last_compression_level = 15;
                    }
                }
            });

            scroll_ui.add_space(20.0);
            scroll_ui.heading("Android");
            scroll_ui.add_space(5.0);

            scroll_ui.horizontal(|horizontal_ui| {
                let tooltip = "Attempt to connect to this IP Address Wirelessly if not automatically found when using Android import method\nMake sure you have \"Wireless USB Debugging\" enabled in your devices developer settings\nRequires ABD OEM Drivers Add-On to function";

                horizontal_ui.label("Fallback IP Address").on_hover_text(tooltip);
                horizontal_ui.spacing_mut().item_spacing.x = 4.0;

                horizontal_ui.allocate_ui(egui::vec2(100.0, 20.0), |allocated_ui| {
                    allocated_ui.centered_and_justified(|centered_ui| {
                        if runtime.show_ip_field {
                            let hint = egui::RichText::new("192.168.X.X").color(egui::Color32::GRAY);
                            centered_ui.add(egui::TextEdit::singleline(&mut settings.manual_ip)
                                .hint_text(hint)
                                .vertical_align(egui::Align::Center))
                                .on_hover_text(tooltip);
                        } else {
                            if centered_ui.button("Click to Reveal").on_hover_text(tooltip).clicked() {
                                runtime.show_ip_field = true;
                            }
                        }
                    });
                });

                horizontal_ui.add_space(2.0);

                if horizontal_ui.button("👁").on_hover_text("Toggle Visibility").clicked() {
                    runtime.show_ip_field = !runtime.show_ip_field;
                }
            });

            scroll_ui.add_space(5.0);

            scroll_ui.horizontal(|horizontal_ui| {
                let label_response = horizontal_ui.label("App Folder Persistence");
                let tooltip_text = "Skip the deletion of the \"game/app\" directory after android import";
                label_response.on_hover_text(tooltip_text);

                let toggle_response = toggle_ui(horizontal_ui, &mut settings.app_folder_persistence).on_hover_text(tooltip_text);
                if toggle_response.changed() { refresh_needed = true; }
            });

            scroll_ui.add_space(10.0);
        });

    if show_folder_delete_modal(&context, drag_guard, "delete_game_modal", "Are you sure you want to delete the \"game\" folder?\nMost app function will be lost.") {
        if raw_deleter.is_active() {
            tracing::info!("Pre-empting raw folder deletion UI state in favor of full game deletion");
            raw_deleter = FolderDeleter::default();
        }
        game_deleter.start("game");
    }

    if show_folder_delete_modal(&context, drag_guard, "delete_raw_modal", "Are you sure you want to delete the \"raw\" folder?\nYou may need to import again if an app update requires new game assets.") {
        raw_deleter.start("game/raw");
    }

    if show_folder_delete_modal(&context, drag_guard, "delete_cache_modal", "Are you sure you want to clear the Cache?\nIt will automatically rebuild the next time the app loads.")
        && let Some(cache_directory) = core::global::io::cache::get_cache_dir() {
        cache_deleter.start(cache_directory);
    }

    crate::features::settings::exceptions::show(&context, drag_guard);
    crate::features::settings::keys::show(&context, drag_guard);

    context.data_mut(|data_map| {
        data_map.insert_temp(egui::Id::new("game_deleter"), game_deleter);
        data_map.insert_temp(egui::Id::new("raw_deleter"), raw_deleter);
        data_map.insert_temp(egui::Id::new("cache_deleter"), cache_deleter);
    });

    refresh_needed
}