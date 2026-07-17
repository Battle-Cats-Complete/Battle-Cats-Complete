use eframe::egui;

use core::common::region::Region;
use core::modules::mods::export::{self, apk, pack, bcm};
use core::modules::mods::ModMetadata;
use core::modules::mods::export::ExportType;
use core::modules::settings::logic::Settings;

use super::state::ModListState;

pub(super) fn show(context: &egui::Context, state: &mut ModListState, settings: &Settings) {
    let mut is_open = state.data.export.is_open;
    let window_id = egui::Id::new("export_mod_window");
    let tracking_open_id = egui::Id::new("export_was_open");
    let was_open = context.data(|data_map| data_map.get_temp::<bool>(tracking_open_id)).unwrap_or_default();
    let current_mod = state.data.selected_mod.clone().unwrap_or_default();
    let tracking_mod_id = egui::Id::new("export_tracking_mod");
    let last_viewed = context.data(|data_map| data_map.get_temp::<String>(tracking_mod_id)).unwrap_or_default();

    if (!was_open && is_open) || (is_open && last_viewed != current_mod) {
        context.data_mut(|data_map| data_map.insert_temp(tracking_mod_id, current_mod.clone()));

        if let Some(mod_folder) = &state.data.selected_mod {
            let metadata = ModMetadata::load(std::path::Path::new("mods").join(mod_folder));
            state.data.export.app_title = metadata.title;
            state.data.export.package_suffix = metadata.package;
        } else {
            state.data.export.app_title.clear();
            state.data.export.package_suffix.clear();
        }
    }
    context.data_mut(|data_map| data_map.insert_temp(tracking_open_id, is_open));

    let is_busy = export::process_events(&mut state.data);
    if is_busy {
        context.request_repaint();
    }

    let (allow_drag, fixed_position) = state.drag_guard.assign_bounds(context, window_id);

    let mut window = egui::Window::new("Export Mod")
        .id(window_id)
        .open(&mut is_open)
        .resizable(true)
        .default_size(egui::vec2(400.0, 300.0))
        .collapsible(false)
        .constrain(false)
        .movable(allow_drag);

    if let Some(position) = fixed_position {
        window = window.current_pos(position);
    }

    window.show(context, |ui_container| {
        ui_container.horizontal(|ui_row| {
            ui_row.spacing_mut().item_spacing.x = 5.0;
            let active_color = egui::Color32::from_rgb(31, 106, 165);
            let inactive_color = egui::Color32::from_gray(60);

            let tabs = [
                (ExportType::Apk, "APK"),
                (ExportType::Bcm, "BCM"),
                (ExportType::Pack, "Pack")
            ];

            for (tab_enum, label) in tabs {
                let is_active = state.data.export.tab == tab_enum;
                let button = egui::Button::new(egui::RichText::new(label).color(egui::Color32::WHITE).size(14.0))
                    .fill(if is_active { active_color } else { inactive_color })
                    .min_size(egui::vec2(80.0, 30.0));

                if ui_row.add_enabled(!state.data.export.is_busy, button).clicked() {
                    state.data.export.tab = tab_enum;
                }
            }
        });

        ui_container.add_space(10.0);

        match state.data.export.tab {
            ExportType::Apk => show_apk_view(ui_container, state, settings),
            ExportType::Bcm => show_bcm_view(ui_container, state),
            ExportType::Pack => show_pack_view(ui_container, state),
        }

        ui_container.add_space(10.0);
        ui_container.separator();

        let raw_log = state.data.export.log_content.trim_end();
        let mut display_status = raw_log.lines().last().unwrap_or("Ready").to_string();

        display_status = display_status.replace('\n', ", ");

        let is_error = display_status.contains("ERROR") || display_status.contains("Error") || display_status.contains("Failed");
        let is_success = display_status.contains("Successfully") || display_status.contains("Complete");

        if is_busy {
            ui_container.horizontal(|ui_row| {
                ui_row.spinner();
                ui_row.label(&display_status);
            });
        } else {
            let status_color = if is_error {
                egui::Color32::LIGHT_RED
            } else if is_success {
                egui::Color32::LIGHT_GREEN
            } else {
                egui::Color32::LIGHT_BLUE
            };
            ui_container.colored_label(status_color, &display_status);
        }

        ui_container.separator();

        egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui_container, |scroll_ui| {
            scroll_ui.set_min_width(scroll_ui.available_width());
            scroll_ui.label(egui::RichText::new(&state.data.export.log_content).monospace().size(12.0));
        });
    });

    state.data.export.is_open = is_open;
}

fn show_apk_view(ui_container: &mut egui::Ui, state: &mut ModListState, settings: &Settings) {
    ui_container.label("Patch and export modded APK");
    ui_container.add_space(5.0);

    ui_container.add_enabled_ui(!state.data.export.is_busy, |enabled_ui| {
        enabled_ui.horizontal(|ui_row| {
            ui_row.label("Title:");
            ui_row.add(egui::TextEdit::singleline(&mut state.data.export.app_title).desired_width(120.0));
        });

        enabled_ui.add_space(4.0);

        enabled_ui.horizontal(|ui_row| {
            ui_row.label("Package:");
            ui_row.add(egui::TextEdit::singleline(&mut state.data.export.package_suffix).desired_width(40.0));
        });

        enabled_ui.add_space(4.0);

        enabled_ui.horizontal(|ui_row| {
            ui_row.label("Region:");
            egui::ComboBox::from_id_salt("export_region_apk")
                .selected_text(state.data.export.target_region.metadata().display_name)
                .show_ui(ui_row, |combo_ui| {
                    combo_ui.selectable_value(&mut state.data.export.target_region, Region::En, Region::En.metadata().display_name);
                    combo_ui.selectable_value(&mut state.data.export.target_region, Region::Ja, Region::Ja.metadata().display_name);
                    combo_ui.selectable_value(&mut state.data.export.target_region, Region::Ko, Region::Ko.metadata().display_name);
                    combo_ui.selectable_value(&mut state.data.export.target_region, Region::Tw, Region::Tw.metadata().display_name);
                });
        });

        enabled_ui.add_space(8.0);

        enabled_ui.horizontal(|ui_row| {
            if ui_row.button("Select App File").clicked() {
                let mut file_dialog = rfd::FileDialog::new();
                file_dialog = file_dialog.add_filter("Android App", &["apk", "xapk", "apkm", "apks"]);

                if let Some(selected_path) = file_dialog.pick_file() {
                    state.data.export.selected_apk = Some(selected_path);
                }
            }

            if let Some(file_path) = &state.data.export.selected_apk {
                let file_name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();

                let display_name = if file_name.chars().count() > 30 {
                    let stem = file_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let ext = file_path.extension().unwrap_or_default().to_string_lossy().to_string();
                    let stem_chars: Vec<char> = stem.chars().collect();

                    if stem_chars.len() > 15 {
                        let first: String = stem_chars[..12].iter().collect();
                        let last: String = stem_chars[stem_chars.len() - 5..].iter().collect();
                        format!("{}...{}.{}", first, last, ext)
                    } else {
                        file_name
                    }
                } else {
                    file_name
                };

                ui_row.label(display_name);
            } else {
                ui_row.label("No file selected");
            }
        });
    });

    ui_container.add_space(8.0);

    let is_ready = state.data.export.selected_apk.is_some() && state.data.selected_mod.is_some();

    if ui_container.add_enabled(!state.data.export.is_busy && is_ready, egui::Button::new("Apply Mod")).clicked() {
        apk::start_export(&mut state.data, settings);
    }
}

fn show_bcm_view(ui_container: &mut egui::Ui, state: &mut ModListState) {
    ui_container.label("Package mod into a standalone .bcm archive");
    ui_container.add_space(5.0);

    let compression_id = egui::Id::new("bcm_compression_slider");
    let mut compression_level = ui_container.ctx().data(|d| d.get_temp::<i64>(compression_id).unwrap_or(bcm::BCM_COMPRESSION_DEFAULT));

    ui_container.add_enabled_ui(!state.data.export.is_busy, |enabled_ui| {
        enabled_ui.horizontal(|ui_row| {
            ui_row.label("Title:");
            ui_row.add(egui::TextEdit::singleline(&mut state.data.export.app_title).desired_width(120.0));
        });

        enabled_ui.add_space(4.0);

        enabled_ui.horizontal(|ui_row| {
            ui_row.label("Compression:");
            ui_row.add(egui::Slider::new(&mut compression_level, bcm::BCM_COMPRESSION_MIN..=bcm::BCM_COMPRESSION_MAX));
        });
    });

    ui_container.ctx().data_mut(|d| d.insert_temp(compression_id, compression_level));

    ui_container.add_space(8.0);

    if ui_container.add_enabled(!state.data.export.is_busy && state.data.selected_mod.is_some(), egui::Button::new("Create BCM Package")).clicked() {
        bcm::start_bcm_export(&mut state.data, compression_level);
    }
}

fn show_pack_view(ui_container: &mut egui::Ui, state: &mut ModListState) {
    ui_container.label("Compile mod files into raw .pack and .list files");
    ui_container.add_space(5.0);

    ui_container.add_enabled_ui(!state.data.export.is_busy, |enabled_ui| {
        enabled_ui.horizontal(|ui_row| {
            ui_row.label("Name:");
            let input_hint = egui::RichText::new("DownloadLocal").color(egui::Color32::GRAY);
            ui_row.add(egui::TextEdit::singleline(&mut state.data.export.pack_name)
                .hint_text(input_hint)
                .desired_width(100.0));
        });

        enabled_ui.add_space(4.0);

        enabled_ui.horizontal(|ui_row| {
            ui_row.label("Key:");
            egui::ComboBox::from_id_salt("export_region_pack")
                .selected_text(state.data.export.target_region.metadata().display_name)
                .show_ui(ui_row, |combo_ui| {
                    combo_ui.selectable_value(&mut state.data.export.target_region, Region::En, Region::En.metadata().display_name);
                    combo_ui.selectable_value(&mut state.data.export.target_region, Region::Ja, Region::Ja.metadata().display_name);
                    combo_ui.selectable_value(&mut state.data.export.target_region, Region::Ko, Region::Ko.metadata().display_name);
                    combo_ui.selectable_value(&mut state.data.export.target_region, Region::Tw, Region::Tw.metadata().display_name);
                });
        });
    });

    ui_container.add_space(8.0);

    if ui_container.add_enabled(!state.data.export.is_busy && state.data.selected_mod.is_some(), egui::Button::new("Create Pack")).clicked() {
        if state.data.export.pack_name.is_empty() {
            state.data.export.pack_name = "DownloadLocal".to_string();
        }
        pack::start_pack_export(&mut state.data);
    }
}