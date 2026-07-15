use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;

use eframe::egui;

use core::addons::toolpaths::{self, Presence};
use core::data::leaders::{android, pack, raw};
use core::data::state::{AdbImportType, AdbTarget, ImportMode, ImportSubTab};
use core::global::region::Region;
use core::settings::logic::Settings;

use super::state::ImportState;

pub fn show(ui: &mut egui::Ui, state: &mut ImportState, settings: &mut Settings) {
    let current_status = state.config.import_job_status.load(Ordering::Relaxed);
    let is_running = current_status == 1;

    let col_width_reduction = 40.0;
    let column_min_height = 120.0;

    let padding_job_details = 10.0;
    let padding_above_separator = 20.0;
    let padding_below_separator = 15.0;

    ui.add_enabled_ui(!is_running, |ui| {
        let total_width = ui.available_width();
        let spacing = 16.0;
        let col_width = (total_width - (spacing * 2.0) - col_width_reduction) / 3.0;

        ui.horizontal(|ui| {
            let active_color = egui::Color32::from_rgb(31, 106, 165);
            let inactive_color = egui::Color32::from_gray(60);
            
            ui.vertical(|ui| {
                ui.set_min_width(col_width);
                ui.set_max_width(col_width);
                ui.set_min_height(column_min_height);

                let adb_installed = toolpaths::adb_status() == Presence::Installed;

                ui.vertical_centered(|ui| {
                    let header_width = col_width * 0.8;
                    let display_color = if state.config.selected_job == Some(ImportSubTab::Emulator) { active_color } else { inactive_color };

                    let android_button = egui::Button::new(egui::RichText::new("Android").color(egui::Color32::WHITE).size(16.0))
                        .fill(display_color).rounding(egui::Rounding::same(6.0));

                    if ui.add_sized([header_width, 35.0], android_button).clicked() && adb_installed {
                        state.config.selected_job = Some(ImportSubTab::Emulator);
                    }

                    ui.add_space(10.0);

                    if adb_installed {
                        ui.label("Import directly via Bridge");
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(200, 150, 50), "Requires Android Bridge Add-On");
                    }
                });

                ui.add_space(padding_job_details);

                ui.add_enabled_ui(adb_installed, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        ui.label("Type:");

                        let type_text = if settings.game_data.adb_import_type_idx == 1 { "Update Only" } else { "All Content" };

                        egui::ComboBox::from_id_salt("adb_type")
                            .selected_text(type_text)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut settings.game_data.adb_import_type_idx, 0, "All Content");
                                ui.selectable_value(&mut settings.game_data.adb_import_type_idx, 1, "Update Only");
                            });
                    });

                    ui.add_space(padding_job_details);

                    ui.horizontal(|ui| {
                        ui.add_space(10.0);
                        ui.label("Region:");

                        let region_text = match settings.game_data.adb_region_idx {
                            0 => "Global", 1 => "Japan", 2 => "Taiwan", 3 => "Korea", _ => "All Regions"
                        };

                        egui::ComboBox::from_id_salt("adb_region_emu")
                            .selected_text(region_text)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut settings.game_data.adb_region_idx, 0, "Global");
                                ui.selectable_value(&mut settings.game_data.adb_region_idx, 1, "Japan");
                                ui.selectable_value(&mut settings.game_data.adb_region_idx, 2, "Taiwan");
                                ui.selectable_value(&mut settings.game_data.adb_region_idx, 3, "Korea");
                                ui.selectable_value(&mut settings.game_data.adb_region_idx, 4, "All Regions");
                            });
                    });
                });
            });

            ui.add_space(spacing / 2.0);
            ui.add(egui::Separator::default().vertical().spacing(0.0));
            ui.add_space(spacing / 2.0);
            
            ui.vertical(|ui| {
                ui.set_min_width(col_width);
                ui.set_max_width(col_width);
                ui.set_min_height(column_min_height);

                ui.vertical_centered(|ui| {
                    let header_width = col_width * 0.8;
                    let display_color = if state.config.selected_job == Some(ImportSubTab::Decrypt) { active_color } else { inactive_color };

                    let pack_button = egui::Button::new(egui::RichText::new("Pack").color(egui::Color32::WHITE).size(16.0))
                        .fill(display_color).rounding(egui::Rounding::same(6.0));

                    if ui.add_sized([header_width, 35.0], pack_button).clicked() {
                        state.config.selected_job = Some(ImportSubTab::Decrypt);
                    }

                    ui.add_space(10.0);
                    ui.label("Decrypt external pack files");
                });

                ui.add_space(padding_job_details);

                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label("Region:");
                    
                    let region_text = state.config.adb_target.as_name();

                    egui::ComboBox::from_id_salt("dec_region")
                        .selected_text(region_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.config.adb_target, AdbTarget::Specific(Region::En), "Global");
                            ui.selectable_value(&mut state.config.adb_target, AdbTarget::Specific(Region::Ja), "Japan");
                            ui.selectable_value(&mut state.config.adb_target, AdbTarget::Specific(Region::Tw), "Taiwan");
                            ui.selectable_value(&mut state.config.adb_target, AdbTarget::Specific(Region::Ko), "Korea");
                            ui.selectable_value(&mut state.config.adb_target, AdbTarget::All, "All Regions");
                        });
                });

                ui.add_space(padding_job_details);

                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    if ui.button("Select Folder").clicked()
                        && let Some(folder_path) = rfd::FileDialog::new().pick_folder() {
                            state.config.decrypt_path = folder_path.to_string_lossy().to_string();
                            state.decrypt_censored = crate::features::data::state::censor_path(&state.config.decrypt_path);
                        }
                    ui.label(if state.decrypt_censored.is_empty() { "None selected" } else { &state.decrypt_censored });
                });
            });

            ui.add_space(spacing / 2.0);
            ui.add(egui::Separator::default().vertical().spacing(0.0));
            ui.add_space(spacing / 2.0);
            
            ui.vertical(|ui| {
                ui.set_min_width(col_width);
                ui.set_max_width(col_width);
                ui.set_min_height(column_min_height);

                ui.vertical_centered(|ui| {
                    let header_width = col_width * 0.8;
                    let display_color = if state.config.selected_job == Some(ImportSubTab::Sort) { active_color } else { inactive_color };

                    let raw_button = egui::Button::new(egui::RichText::new("Raw").color(egui::Color32::WHITE).size(16.0))
                        .fill(display_color).rounding(egui::Rounding::same(6.0));

                    if ui.add_sized([header_width, 35.0], raw_button).clicked() {
                        state.config.selected_job = Some(ImportSubTab::Sort);
                    }

                    ui.add_space(10.0);
                    ui.label("Sort archive or raw files");
                });

                ui.add_space(padding_job_details);

                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label("Source:");

                    let source_text = match state.config.import_mode { ImportMode::Folder => "Folder", _ => "Archive" };

                    egui::ComboBox::from_id_salt("raw_mode")
                        .selected_text(source_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.config.import_mode, ImportMode::Folder, "Folder");
                            ui.selectable_value(&mut state.config.import_mode, ImportMode::Zip, "Archive");
                        });
                });

                ui.add_space(padding_job_details);

                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    if ui.button("Select Data").clicked() {
                        let dialog_result = match state.config.import_mode {
                            ImportMode::Zip => rfd::FileDialog::new().add_filter("Archive", &["zst", "tar", "zip"]).pick_file(),
                            ImportMode::Folder => rfd::FileDialog::new().pick_folder(),
                            _ => None,
                        };
                        if let Some(file_path) = dialog_result {
                            state.config.import_path = file_path.to_string_lossy().to_string();
                            state.import_censored = crate::features::data::state::censor_path(&state.config.import_path);
                        }
                    }
                    ui.label(if state.import_censored.is_empty() { "None selected" } else { &state.import_censored });
                });
            });
        });
    });

    ui.add_space(padding_above_separator);
    ui.add(egui::Separator::default().spacing(0.0));
    ui.add_space(padding_below_separator);

    ui.horizontal(|ui| {
        let button_width = 300.0;
        ui.add_space((ui.available_width() - button_width) / 2.0);

        let show_success = state.config.import_job_completed_time.is_some_and(|time| time.elapsed().as_secs() < 2);
        let show_aborted = state.config.import_job_aborted_time.is_some_and(|time| time.elapsed().as_secs() < 2);
        let is_aborting = is_running && state.config.import_abort_flag.load(Ordering::Relaxed);

        let (button_text, can_run, active_color) = match state.config.selected_job {
            Some(ImportSubTab::Emulator) => {
                let is_installed = toolpaths::adb_status() == Presence::Installed;
                (if is_installed { "Start Job" } else { "Bridge Missing" }, is_installed, egui::Color32::from_rgb(31, 106, 165))
            },
            Some(ImportSubTab::Decrypt) => {
                let has_path = !state.config.decrypt_path.is_empty();
                (if has_path { "Start Job" } else { "Select Source Folder" }, has_path, egui::Color32::from_rgb(31, 106, 165))
            },
            Some(ImportSubTab::Sort) => {
                let has_path = !state.config.import_path.is_empty();
                (if has_path { "Start Job" } else { "Select Source Data" }, has_path, egui::Color32::from_rgb(31, 106, 165))
            },
            None => ("Select a Job", false, egui::Color32::from_gray(80)),
        };

        if show_success {
            let success_btn = egui::Button::new(egui::RichText::new("Job Complete!").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(50, 180, 50))
                .min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));

            if ui.add_enabled(can_run, success_btn).clicked() { trigger_import_job(state, settings); }
            return;
        }

        if show_aborted {
            let aborted_btn = egui::Button::new(egui::RichText::new("Job Aborted!").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(200, 50, 50))
                .min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));

            if ui.add_enabled(can_run, aborted_btn).clicked() { trigger_import_job(state, settings); }
            return;
        }

        if is_aborting {
            let aborting_btn = egui::Button::new(egui::RichText::new("Aborting Job...").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(220, 180, 40))
                .min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));

            ui.add(aborting_btn);
            return;
        }

        if is_running {
            let cancel_btn = egui::Button::new(egui::RichText::new("Abort Job").color(egui::Color32::WHITE).size(18.0).strong())
                .fill(egui::Color32::from_rgb(200, 50, 50))
                .min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));

            if ui.add(cancel_btn).clicked() {
                state.config.import_abort_flag.store(true, Ordering::Relaxed);
                state.config.import_progress_current.store(0, Ordering::Relaxed);
                state.config.import_progress_maximum.store(0, Ordering::Relaxed);
            }
            return;
        }

        let final_color = if can_run { active_color } else { egui::Color32::from_gray(80) };
        let action_btn = egui::Button::new(egui::RichText::new(button_text).color(egui::Color32::WHITE).size(18.0).strong())
            .fill(final_color).min_size(egui::vec2(button_width, 45.0)).rounding(egui::Rounding::same(8.0));

        if ui.add_enabled(can_run, action_btn).clicked() {
            trigger_import_job(state, settings);
        }
    });
}

fn trigger_import_job(state: &mut ImportState, settings: &mut Settings) {
    state.config.import_job_status.store(1, Ordering::Relaxed);
    state.config.import_abort_flag.store(false, Ordering::Relaxed);
    state.config.import_progress_current.store(0, Ordering::Relaxed);
    state.config.import_progress_maximum.store(0, Ordering::Relaxed);
    state.config.import_log_content.clear();
    state.config.import_job_completed_time = None;
    state.config.import_job_aborted_time = None;

    let (sender, receiver) = mpsc::channel();
    state.config.import_rx = Some(receiver);

    let abort = state.config.import_abort_flag.clone();
    let status = state.config.import_job_status.clone();
    let progress_current = state.config.import_progress_current.clone();
    let progress_max = state.config.import_progress_maximum.clone();
    let enforce_val = settings.game_data.enforce_key_validation;

    match state.config.selected_job {
        Some(ImportSubTab::Emulator) => {
            let mode = if settings.game_data.adb_import_type_idx == 1 { AdbImportType::Update } else { AdbImportType::All };
            let region = match settings.game_data.adb_region_idx {
                0 => AdbTarget::Specific(Region::En),
                1 => AdbTarget::Specific(Region::Ja),
                2 => AdbTarget::Specific(Region::Tw),
                3 => AdbTarget::Specific(Region::Ko),
                _ => AdbTarget::All
            };
            android::run(
                sender,
                mode,
                region,
                settings.emulator_config(),
                enforce_val,
                abort,
                status,
                progress_current,
                progress_max
            );
        },
        Some(ImportSubTab::Decrypt) => {
            let folder_path = state.config.decrypt_path.clone();
            let mode = ImportMode::Folder;
            let region = state.config.adb_target;

            thread::spawn(move || {
                let result = pack::run(
                    &folder_path,
                    mode,
                    region,
                    enforce_val,
                    sender,
                    abort,
                    progress_current,
                    progress_max
                );

                if result.is_err() {
                    status.store(3, Ordering::Relaxed);
                } else {
                    status.store(2, Ordering::Relaxed);
                }
            });
        },
        Some(ImportSubTab::Sort) => {
            let data_path = state.config.import_path.clone();
            let lang_priority = settings.general.language_priority.clone();

            thread::spawn(move || {
                let result = raw::run(
                    &data_path,
                    sender,
                    abort,
                    progress_current,
                    progress_max,
                    &lang_priority
                );

                if result.is_err() {
                    status.store(3, Ordering::Relaxed);
                } else {
                    status.store(2, Ordering::Relaxed);
                }
            });
        },
        None => {}
    }
}