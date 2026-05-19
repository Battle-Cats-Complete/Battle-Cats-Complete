use eframe::egui;
use crate::features::mods::logic::state::{ExportType, ModState, PatchMode};
use crate::global::region::Region;
use crate::features::settings::logic::Settings;
use crate::features::mods::logic::metadata;
use crate::features::addons::toolpaths::{self, Presence};
use crate::features::mods::export::{patch, create, update, pack};

pub fn show(ctx: &egui::Context, state: &mut ModState, _settings: &Settings) {
    let mut is_open = state.export.is_open;
    let window_id = egui::Id::new("export_mod_window");
    let tracking_open_id = egui::Id::new("export_was_open");
    let was_open = ctx.data(|d| d.get_temp::<bool>(tracking_open_id)).unwrap_or_default();
    let current_mod = state.selected_mod.clone().unwrap_or_default();
    let tracking_mod_id = egui::Id::new("export_tracking_mod");
    let last_viewed = ctx.data(|d| d.get_temp::<String>(tracking_mod_id)).unwrap_or_default();

    if (!was_open && is_open) || (is_open && last_viewed != current_mod) {
        if state.export.patch_mode == PatchMode::Create {
            if let Some(mod_folder) = &state.selected_mod {
                let meta = metadata::ModMetadata::load(&std::path::Path::new("mods").join(mod_folder));
                state.export.app_title = meta.title;
                state.export.package_suffix = meta.package;
            } else {
                state.export.app_title.clear();
                state.export.package_suffix.clear();
            }
        } else {
            state.export.app_title.clear();
            state.export.package_suffix.clear();
        }
        ctx.data_mut(|d| d.insert_temp(tracking_mod_id, current_mod));
    }
    ctx.data_mut(|d| d.insert_temp(tracking_open_id, is_open));

    let is_busy = patch::process_events(state);
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

    if let Some(position) = fixed_pos { window = window.current_pos(position); }

    window.show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let active_color = egui::Color32::from_rgb(31, 106, 165);
            let inactive_color = egui::Color32::from_gray(60);

            let tabs = [(ExportType::Apk, "APK"), (ExportType::Pack, "Pack")];

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

        ui.add_space(10.0);

        match state.export.tab {
            ExportType::Apk => show_apk_view(ui, state),
            ExportType::Pack => show_pack_view(ui, state),
        }

        ui.add_space(10.0);
        ui.separator();

        let status_message = &state.export.status_message;
        let is_processing = is_busy
            && !status_message.contains("Success")
            && !status_message.contains("Error")
            && !status_message.contains("Failed")
            && !status_message.contains("Complete");

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
    let apktool_present = toolpaths::apktool_status() == Presence::Installed;
    if !apktool_present && state.export.patch_mode == PatchMode::Create {
        state.export.patch_mode = PatchMode::Update;
        if let Some(path) = &state.export.selected_apk {
            if path.extension().and_then(|e| e.to_str()) == Some("xapk") {
                state.export.selected_apk = None;
            }
        }
    }

    if state.export.patch_mode == PatchMode::Create && !apktool_present {
        ui.label(egui::RichText::new("Apktool Add-On Missing: Download through Settings > Add-Ons > Apktool")
            .color(egui::Color32::from_rgb(255, 165, 0)));
    } else {
        ui.label("Update or create modded APK");
    }

    ui.add_space(5.0);

    ui.add_enabled_ui(!state.export.is_busy, |ui| {
        ui.horizontal(|ui| {
            ui.label("Patch:");
            let prev_mode = state.export.patch_mode.clone();

            egui::ComboBox::from_id_salt("patch_mode_combo")
                .selected_text(match state.export.patch_mode {
                    PatchMode::Update => "Update",
                    PatchMode::Create => "Create",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.export.patch_mode, PatchMode::Update, "Update");
                    ui.add_enabled_ui(apktool_present, |ui| {
                        let res = ui.selectable_value(&mut state.export.patch_mode, PatchMode::Create, "Create");
                        if !apktool_present {
                            res.on_disabled_hover_text("Requires Apktool Add-On\nDownload through Settings > Add-Ons > Apktool");
                        }
                    });
                });

            if prev_mode != state.export.patch_mode {
                if state.export.patch_mode == PatchMode::Update {
                    if let Some(path) = &state.export.selected_apk {
                        if path.extension().and_then(|e| e.to_str()) == Some("xapk") {
                            state.export.selected_apk = None;
                        }
                    }
                    state.export.app_title.clear();
                    state.export.package_suffix.clear();
                } else if state.export.patch_mode == PatchMode::Create {
                    if let Some(mod_folder) = &state.selected_mod {
                        let meta = metadata::ModMetadata::load(&std::path::Path::new("mods").join(mod_folder));
                        state.export.app_title = meta.title;
                        state.export.package_suffix = meta.package;
                    }
                }
            }
        });

        ui.add_space(4.0);

        let is_create = state.export.patch_mode == PatchMode::Create;
        let deep_patch_allowed = is_create && apktool_present;

        ui.horizontal(|ui| {
            let label = if deep_patch_allowed { egui::RichText::new("Title:") } else { egui::RichText::new("Title:").weak() };
            ui.label(label);

            let title_field = ui.add_enabled(deep_patch_allowed, egui::TextEdit::singleline(&mut state.export.app_title)
                .desired_width(120.0));

            if !is_create {
                title_field.on_disabled_hover_text("Only available with Patch option \"Create\"");
            } else if !apktool_present {
                title_field.on_disabled_hover_text("Requires Apktool Add-On\nDownload through Settings > Add-Ons > Apktool");
            }
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let label = if deep_patch_allowed { egui::RichText::new("Package:") } else { egui::RichText::new("Package:").weak() };
            ui.label(label);

            let pkg_field = ui.add_enabled(deep_patch_allowed, egui::TextEdit::singleline(&mut state.export.package_suffix)
                .desired_width(40.0));

            if !is_create {
                pkg_field.on_disabled_hover_text("Only available with Patch option \"Create\"");
            } else if !apktool_present {
                pkg_field.on_disabled_hover_text("Requires Apktool Add-On\nDownload through Settings > Add-Ons > Apktool");
            }
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Region:");
            egui::ComboBox::from_id_salt("export_region_apk")
                .selected_text(state.export.target_region.metadata().display_name)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.export.target_region, Region::En, Region::En.metadata().display_name);
                    ui.selectable_value(&mut state.export.target_region, Region::Ja, Region::Ja.metadata().display_name);
                    ui.selectable_value(&mut state.export.target_region, Region::Ko, Region::Ko.metadata().display_name);
                    ui.selectable_value(&mut state.export.target_region, Region::Tw, Region::Tw.metadata().display_name);
                });
        });

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let btn_text = if is_create { "Select (X)APK" } else { "Select APK" };
            if ui.button(btn_text).clicked() {
                let mut dialog = rfd::FileDialog::new();
                if is_create {
                    dialog = dialog.add_filter("Android App", &["apk", "xapk"]);
                } else {
                    dialog = dialog.add_filter("APK", &["apk"]);
                }

                if let Some(file_path) = dialog.pick_file() {
                    state.export.selected_apk = Some(file_path);
                }
            }
            if let Some(file_path) = &state.export.selected_apk {
                ui.label(file_path.file_name().unwrap_or_default().to_string_lossy());
            } else {
                ui.label("No file selected");
            }
        });
    });

    ui.add_space(8.0);

    let is_ready = state.export.selected_apk.is_some() && state.selected_mod.is_some();
    let can_apply = !state.export.is_busy && is_ready && !(state.export.patch_mode == PatchMode::Create && !apktool_present);

    if ui.add_enabled(can_apply, egui::Button::new("Apply Mod")).clicked() {
        if state.export.patch_mode == PatchMode::Create {
            create::start_apk_export(state);
        } else {
            update::start_fast_track_export(state);
        }
    }
}

fn show_pack_view(ui: &mut egui::Ui, state: &mut ModState) {
    ui.label("Compile mod files into raw .pack and .list files");
    ui.add_space(5.0);

    ui.add_enabled_ui(!state.export.is_busy, |ui| {
        ui.horizontal(|ui| {
            ui.label("Name:");
            let hint = egui::RichText::new("DownloadLocal").color(egui::Color32::GRAY);
            ui.add(egui::TextEdit::singleline(&mut state.export.pack_name)
                .hint_text(hint)
                .desired_width(100.0));
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Key:");
            egui::ComboBox::from_id_salt("export_region_pack")
                .selected_text(state.export.target_region.metadata().display_name)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.export.target_region, Region::En, Region::En.metadata().display_name);
                    ui.selectable_value(&mut state.export.target_region, Region::Ja, Region::Ja.metadata().display_name);
                    ui.selectable_value(&mut state.export.target_region, Region::Ko, Region::Ko.metadata().display_name);
                    ui.selectable_value(&mut state.export.target_region, Region::Tw, Region::Tw.metadata().display_name);
                });
        });
    });

    ui.add_space(8.0);
    if ui.add_enabled(!state.export.is_busy && state.selected_mod.is_some(), egui::Button::new("Create Pack")).clicked() {
        if state.export.pack_name.is_empty() {
            state.export.pack_name = "DownloadLocal".to_string();
        }
        pack::start_pack_export(state);
    }
}