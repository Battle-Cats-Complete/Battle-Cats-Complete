use eframe::egui;
use crate::features::mods::logic::state::{ModState, ExportType, SignType, TargetRegion};
use crate::features::settings::logic::Settings;
use crate::features::mods::logic::encrypt;

pub fn show(ctx: &egui::Context, state: &mut ModState, _settings: &Settings) {
    let mut is_open = state.export.is_open;
    let window_id = egui::Id::new("export_mod_window");

    let is_busy = encrypt::process_events(state);
    if is_busy {
        ctx.request_repaint();
    }

    let (allow_drag, fixed_pos) = state.drag_guard.assign_bounds(ctx, window_id);

    let mut window = egui::Window::new("Export Mod")
        .id(window_id)
        .open(&mut is_open)
        .resizable(true)
        .default_size(egui::vec2(500.0, 400.0))
        .collapsible(false)
        .constrain(false)
        .movable(allow_drag);

    if let Some(position) = fixed_pos {
        window = window.current_pos(position);
    }

    window.show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let active_color = egui::Color32::from_rgb(31, 106, 165);
            let inactive_color = egui::Color32::from_gray(60);

            let tabs = [
                (ExportType::Apk, "APK"),
                (ExportType::Pack, "Pack"),
            ];

            for (tab_enum, label) in tabs {
                let is_active = state.export.tab == tab_enum;
                let button = egui::Button::new(egui::RichText::new(label).color(egui::Color32::WHITE).size(14.0))
                    .fill(if is_active { active_color } else { inactive_color })
                    .min_size(egui::vec2(80.0, 30.0));

                if ui.add_enabled(!state.export.is_busy, button).clicked() {
                    state.export.tab = tab_enum;
                }
            }
        });

        ui.add_space(15.0);

        match state.export.tab {
            ExportType::Apk => show_apk_view(ui, state),
            ExportType::Pack => show_pack_view(ui, state),
        }

        ui.add_space(15.0);
        ui.separator();

        let status_message = &state.export.status_message;
        let is_processing = is_busy
            && !status_message.contains("Success")
            && !status_message.contains("Error")
            && !status_message.contains("Failed");

        if is_processing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(status_message);
            });
        } else {
            let status_color = if status_message.contains("Error") || status_message.contains("Failed") {
                egui::Color32::LIGHT_RED
            } else if status_message.contains("Success") || status_message.contains("Complete") {
                egui::Color32::LIGHT_GREEN
            } else {
                egui::Color32::LIGHT_BLUE
            };
            ui.colored_label(status_color, status_message);
        }

        ui.separator();

        egui::ScrollArea::vertical().stick_to_bottom(true).auto_shrink([false, false]).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(egui::RichText::new(&state.export.log_content).monospace().size(12.0));
        });
    });

    state.export.is_open = is_open;
}

fn show_apk_view(ui: &mut egui::Ui, state: &mut ModState) {
    ui.label("Patch and sign an existing APK or XAPK file.");
    ui.add_space(10.0);

    ui.add_enabled_ui(!state.export.is_busy, |ui| {
        ui.horizontal(|ui| {
            ui.label("Custom Suffix:");
            ui.add(egui::TextEdit::singleline(&mut state.export.package_suffix)
                .hint_text("None")
                .desired_width(60.0));
            ui.label(egui::RichText::new("(Used for side-by-side install)").weak().size(10.0));
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Target Region:");
            egui::ComboBox::from_id_salt("export_region_type_apk")
                .selected_text(match state.export.target_region {
                    TargetRegion::En => "EN",
                    TargetRegion::Jp => "JP",
                    TargetRegion::Kr => "KR",
                    TargetRegion::Tw => "TW",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.export.target_region, TargetRegion::En, "EN");
                    ui.selectable_value(&mut state.export.target_region, TargetRegion::Jp, "JP");
                    ui.selectable_value(&mut state.export.target_region, TargetRegion::Kr, "KR");
                    ui.selectable_value(&mut state.export.target_region, TargetRegion::Tw, "TW");
                });
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Sign Type:");
            egui::ComboBox::from_id_salt("export_sign_type")
                .selected_text(match state.export.sign_type {
                    SignType::V1 => "v1",
                    SignType::V2 => "v2 (Recommended)",
                    SignType::V3 => "v3",
                    SignType::V4 => "v4",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.export.sign_type, SignType::V1, "v1");
                    ui.selectable_value(&mut state.export.sign_type, SignType::V2, "v2 (Recommended)");
                    ui.selectable_value(&mut state.export.sign_type, SignType::V3, "v3");
                    ui.selectable_value(&mut state.export.sign_type, SignType::V4, "v4");
                });
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.button("Select App File").clicked() {
                if let Some(file_path) = rfd::FileDialog::new()
                    .add_filter("Android App", &["apk", "xapk"])
                    .pick_file()
                {
                    state.export.selected_apk = Some(file_path);
                }
            }
            if let Some(file_path) = &state.export.selected_apk {
                ui.label(file_path.file_name().unwrap_or_default().to_string_lossy());
            } else {
                ui.label("No file selected.");
            }
        });
    });

    ui.add_space(15.0);

    let is_ready_to_export = state.export.selected_apk.is_some() && state.selected_mod.is_some();
    if ui.add_enabled(!state.export.is_busy && is_ready_to_export, egui::Button::new("Apply Mod")).clicked() {
        encrypt::start_apk_export(state);
    }
}

fn show_pack_view(ui: &mut egui::Ui, state: &mut ModState) {
    ui.label("Compile mod files into raw .pack and .list files.");
    ui.add_space(10.0);

    ui.add_enabled_ui(!state.export.is_busy, |ui| {
        ui.horizontal(|ui| {
            ui.label("Pack Name:");
            ui.add(egui::TextEdit::singleline(&mut state.export.pack_name)
                .hint_text("mod")
                .desired_width(150.0));
        });

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Target Region:");
            egui::ComboBox::from_id_salt("export_region_type")
                .selected_text(match state.export.target_region {
                    TargetRegion::En => "EN",
                    TargetRegion::Jp => "JP",
                    TargetRegion::Kr => "KR",
                    TargetRegion::Tw => "TW",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.export.target_region, TargetRegion::En, "EN");
                    ui.selectable_value(&mut state.export.target_region, TargetRegion::Jp, "JP");
                    ui.selectable_value(&mut state.export.target_region, TargetRegion::Kr, "KR");
                    ui.selectable_value(&mut state.export.target_region, TargetRegion::Tw, "TW");
                });
        });
    });

    ui.add_space(15.0);
    if ui.add_enabled(!state.export.is_busy && state.selected_mod.is_some(), egui::Button::new("Create Pack")).clicked() {
        encrypt::start_pack_export(state);
    }
}