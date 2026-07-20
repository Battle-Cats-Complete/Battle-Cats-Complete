use std::env;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use iced::widget::{
    button, checkbox, column, container, pick_list, progress_bar, row, scrollable, slider, text,
    text_input, Space,
};
use iced::{Alignment, Element, Font, Length, Subscription, Task};
use tracing::{info, trace, warn};

use core::common::region::Region;
use core::modules::addons::paths::{self, Presence};
use core::modules::data::{
    android, export, pack, raw, AdbImportType, AdbTarget, DataConfigState, DataTab, ImportMode,
    ImportSubTab,
};
use core::modules::settings::Settings;

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    TabSelected(DataTab),
    ImportJobSelected(ImportSubTab),
    AdbImportTypeChanged(usize),
    AdbRegionChangedEmu(usize),
    AdbRegionChangedDec(AdbTarget),
    ImportModeChanged(ImportMode),
    SelectDecryptFolder,
    SelectImportData,
    TriggerImportJob,
    AbortImportJob,
    ToggleIncludeRaw(bool),
    ExportFilenameChanged(String),
    CompressionLevelChanged(i32),
    TriggerExportJob,
    AbortExportJob,
}

#[derive(Default)]
pub struct State {
    pub config: DataConfigState,
    pub import_censored: String,
    pub decrypt_censored: String,
}

impl State {
    pub fn subscription(&self) -> Subscription<Message> {
        // TODO: Rewrite `core` to handle `iced` without ticking
        iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
    }

    pub fn update(&mut self, message: Message, settings: &mut Settings) -> Task<Message> {
        match message {
            Message::Tick => {
                let flags = self.config.tick_threads();
                if flags.import_finished_just_now {
                    info!("Import/Export thread job completed.");
                }

                self.import_censored = censor_path(&self.config.import_path);
                self.decrypt_censored = censor_path(&self.config.decrypt_path);
            }
            Message::TabSelected(tab) => {
                trace!("Switching data tab");
                self.config.active_tab = tab;
            }
            Message::ImportJobSelected(job) => {
                trace!("Selected import sub-job");
                self.config.selected_job = Some(job);
            }
            Message::AdbImportTypeChanged(idx) => {
                settings.game_data.adb_import_type_idx = idx;
            }
            Message::AdbRegionChangedEmu(idx) => {
                settings.game_data.adb_region_idx = idx;
            }
            Message::AdbRegionChangedDec(target) => {
                self.config.adb_target = target;
            }
            Message::ImportModeChanged(mode) => {
                self.config.import_mode = mode;
            }
            Message::SelectDecryptFolder => {
                if let Some(folder_path) = rfd::FileDialog::new().pick_folder() {
                    self.config.decrypt_path = folder_path.to_string_lossy().to_string();
                    self.decrypt_censored = censor_path(&self.config.decrypt_path);
                    info!("Selected decrypt folder: {}", self.decrypt_censored);
                }
            }
            Message::SelectImportData => {
                let dialog_result = match self.config.import_mode {
                    ImportMode::Zip => rfd::FileDialog::new()
                        .add_filter("Archive", &["zst", "tar", "zip"])
                        .pick_file(),
                    ImportMode::Folder => rfd::FileDialog::new().pick_folder(),
                    _ => None,
                };

                if let Some(file_path) = dialog_result {
                    self.config.import_path = file_path.to_string_lossy().to_string();
                    self.import_censored = censor_path(&self.config.import_path);
                    info!("Selected import data path: {}", self.import_censored);
                }
            }
            Message::TriggerImportJob => {
                info!("Starting import job.");
                self.trigger_import_job(settings);
            }
            Message::AbortImportJob => {
                warn!("Aborting import job.");
                self.config.import_abort_flag.store(true, Ordering::Relaxed);
                self.config.import_progress_current.store(0, Ordering::Relaxed);
                self.config.import_progress_maximum.store(0, Ordering::Relaxed);
            }
            Message::ToggleIncludeRaw(include) => {
                self.config.include_raw = include;
            }
            Message::ExportFilenameChanged(name) => {
                self.config.export_filename = name;
            }
            Message::CompressionLevelChanged(level) => {
                self.config.compression_level = level;
                settings.game_data.last_compression_level = level;
            }
            Message::TriggerExportJob => {
                info!("Starting export job.");
                self.trigger_export_job();
            }
            Message::AbortExportJob => {
                warn!("Aborting export job.");
                self.config.export_abort_flag.store(true, Ordering::Relaxed);
                self.config.export_progress_current.store(0, Ordering::Relaxed);
                self.config.export_progress_maximum.store(0, Ordering::Relaxed);
            }
        }
        Task::none()
    }

    pub fn view(&self, settings: &Settings) -> Element<'_, Message> {
        let is_import = self.config.active_tab == DataTab::Import;
        let is_export = self.config.active_tab == DataTab::Export;

        let tabs_row = row![
            button(text("Import").size(16))
                .style(if is_import { button::primary } else { button::secondary })
                .width(Length::Fixed(120.0))
                .on_press(Message::TabSelected(DataTab::Import)),
            button(text("Export").size(16))
                .style(if is_export { button::primary } else { button::secondary })
                .width(Length::Fixed(120.0))
                .on_press(Message::TabSelected(DataTab::Export)),
        ]
            .spacing(10);

        let content = if is_import {
            self.view_import(settings)
        } else {
            self.view_export(settings)
        };

        let progress_section = self.view_progress_and_console();

        column![
            tabs_row,
            Space::new().height(10),
            content,
            Space::new().height(10),
            progress_section
        ]
            .spacing(10)
            .padding(20)
            .into()
    }

    fn view_import(&self, settings: &Settings) -> Element<'_, Message> {
        let current_status = self.config.import_job_status.load(Ordering::Relaxed);
        let is_running = current_status == 1;
        let adb_installed = paths::adb_status() == Presence::Installed;

        let android_btn = button(
            text("Android")
                .size(16)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        )
            .style(if self.config.selected_job == Some(ImportSubTab::Emulator) {
                button::primary
            } else {
                button::secondary
            })
            .on_press_maybe(if !is_running && adb_installed {
                Some(Message::ImportJobSelected(ImportSubTab::Emulator))
            } else {
                None
            });

        let import_types = vec!["All Content", "Update Only"];
        let current_type = if settings.game_data.adb_import_type_idx == 1 {
            "Update Only"
        } else {
            "All Content"
        };
        let type_picker = pick_list(import_types, Some(current_type), |sel| {
            Message::AdbImportTypeChanged(if sel == "Update Only" { 1 } else { 0 })
        });

        let emu_regions = vec!["Global", "Japan", "Taiwan", "Korea", "All Regions"];
        let emu_selected = emu_regions
            .get(settings.game_data.adb_region_idx)
            .copied()
            .unwrap_or("Global");
        let emu_region_picker = pick_list(emu_regions, Some(emu_selected), |sel| {
            let idx = match sel {
                "Japan" => 1,
                "Taiwan" => 2,
                "Korea" => 3,
                "All Regions" => 4,
                _ => 0,
            };
            Message::AdbRegionChangedEmu(idx)
        });

        let mut android_col = column![
            android_btn,
            if adb_installed {
                text("Import directly via Bridge").size(14)
            } else {
                text("Requires Android Bridge Add-On").size(14).style(text::danger)
            },
            Space::new().height(10),
        ];

        if adb_installed {
            android_col = android_col
                .push(row![text("Type: "), type_picker].align_y(Alignment::Center).spacing(5))
                .push(Space::new().height(10))
                .push(row![text("Region: "), emu_region_picker].align_y(Alignment::Center).spacing(5));
        }
        let android_col = android_col.spacing(5).width(Length::FillPortion(1));

        let pack_btn = button(
            text("Pack")
                .size(16)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        )
            .style(if self.config.selected_job == Some(ImportSubTab::Decrypt) {
                button::primary
            } else {
                button::secondary
            })
            .on_press_maybe(if !is_running {
                Some(Message::ImportJobSelected(ImportSubTab::Decrypt))
            } else {
                None
            });

        let dec_regions = vec!["Global", "Japan", "Taiwan", "Korea", "All Regions"];
        let dec_selected = match self.config.adb_target {
            AdbTarget::Specific(Region::En) => "Global",
            AdbTarget::Specific(Region::Ja) => "Japan",
            AdbTarget::Specific(Region::Tw) => "Taiwan",
            AdbTarget::Specific(Region::Ko) => "Korea",
            AdbTarget::All => "All Regions",
        };
        let dec_region_picker = pick_list(dec_regions, Some(dec_selected), |sel| {
            let target = match sel {
                "Japan" => AdbTarget::Specific(Region::Ja),
                "Taiwan" => AdbTarget::Specific(Region::Tw),
                "Korea" => AdbTarget::Specific(Region::Ko),
                "All Regions" => AdbTarget::All,
                _ => AdbTarget::Specific(Region::En),
            };
            Message::AdbRegionChangedDec(target)
        });

        let pack_folder_label = if self.decrypt_censored.is_empty() {
            "None selected"
        } else {
            &self.decrypt_censored
        };

        let pack_col = column![
            pack_btn,
            text("Decrypt external pack files").size(14),
            Space::new().height(10),
            row![text("Region: "), dec_region_picker].align_y(Alignment::Center).spacing(5),
            Space::new().height(10),
            row![
                button("Select Folder").on_press_maybe(if !is_running {
                    Some(Message::SelectDecryptFolder)
                } else {
                    None
                }),
                text(pack_folder_label)
            ]
            .align_y(Alignment::Center)
            .spacing(10)
        ]
            .spacing(5)
            .width(Length::FillPortion(1));

        let raw_btn = button(
            text("Raw")
                .size(16)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        )
            .style(if self.config.selected_job == Some(ImportSubTab::Sort) {
                button::primary
            } else {
                button::secondary
            })
            .on_press_maybe(if !is_running {
                Some(Message::ImportJobSelected(ImportSubTab::Sort))
            } else {
                None
            });

        let modes = vec!["Folder", "Archive"];
        let current_mode = match self.config.import_mode {
            ImportMode::Folder => "Folder",
            _ => "Archive",
        };
        let mode_picker = pick_list(modes, Some(current_mode), |sel| {
            Message::ImportModeChanged(if sel == "Folder" {
                ImportMode::Folder
            } else {
                ImportMode::Zip
            })
        });

        let raw_folder_label = if self.import_censored.is_empty() {
            "None selected"
        } else {
            &self.import_censored
        };

        let raw_col = column![
            raw_btn,
            text("Sort archive or raw files").size(14),
            Space::new().height(10),
            row![text("Source: "), mode_picker].align_y(Alignment::Center).spacing(5),
            Space::new().height(10),
            row![
                button("Select Data").on_press_maybe(if !is_running {
                    Some(Message::SelectImportData)
                } else {
                    None
                }),
                text(raw_folder_label)
            ]
            .align_y(Alignment::Center)
            .spacing(10)
        ]
            .spacing(5)
            .width(Length::FillPortion(1));

        let sections_row = row![android_col, pack_col, raw_col].spacing(20);

        let show_success = self
            .config
            .import_job_completed_time
            .is_some_and(|time| time.elapsed().as_secs() < 2);
        let show_aborted = self
            .config
            .import_job_aborted_time
            .is_some_and(|time| time.elapsed().as_secs() < 2);
        let is_aborting = is_running && self.config.import_abort_flag.load(Ordering::Relaxed);

        let (button_text, can_run) = match self.config.selected_job {
            Some(ImportSubTab::Emulator) => {
                let is_installed = paths::adb_status() == Presence::Installed;
                (
                    if is_installed { "Start Job" } else { "Bridge Missing" },
                    is_installed,
                )
            }
            Some(ImportSubTab::Decrypt) => {
                let has_path = !self.config.decrypt_path.is_empty();
                (
                    if has_path { "Start Job" } else { "Select Source Folder" },
                    has_path,
                )
            }
            Some(ImportSubTab::Sort) => {
                let has_path = !self.config.import_path.is_empty();
                (
                    if has_path { "Start Job" } else { "Select Source Data" },
                    has_path,
                )
            }
            None => ("Select a Job", false),
        };

        let action_btn = if show_success {
            button(text("Job Complete!").size(18))
                .style(button::success)
                .width(Length::Fixed(300.0))
                .on_press_maybe(if can_run { Some(Message::TriggerImportJob) } else { None })
        } else if show_aborted {
            button(text("Job Aborted!").size(18))
                .style(button::danger)
                .width(Length::Fixed(300.0))
                .on_press_maybe(if can_run { Some(Message::TriggerImportJob) } else { None })
        } else if is_aborting {
            button(text("Aborting Job...").size(18))
                .style(button::danger)
                .width(Length::Fixed(300.0))
        } else if is_running {
            button(text("Abort Job").size(18))
                .style(button::danger)
                .width(Length::Fixed(300.0))
                .on_press(Message::AbortImportJob)
        } else {
            button(text(button_text).size(18))
                .style(if can_run { button::primary } else { button::secondary })
                .width(Length::Fixed(300.0))
                .on_press_maybe(if can_run { Some(Message::TriggerImportJob) } else { None })
        };

        let action_row = container(action_btn).width(Length::Fill).center_x(Length::Fill);

        column![sections_row, Space::new().height(20), action_row].into()
    }

    fn view_export(&self, settings: &Settings) -> Element<'_, Message> {
        let current_status = self.config.export_job_status.load(Ordering::Relaxed);
        let is_running = current_status == 1;

        let title = text("Package database into a ZST archive").size(16);

        let toggle_raw = row![
            checkbox(self.config.include_raw).on_toggle_maybe(if !is_running {
                Some(Message::ToggleIncludeRaw)
            } else {
                None
            }),
            text("Include \"raw\" Folder")
        ]
            .spacing(10)
            .align_y(Alignment::Center);

        let file_input = text_input("battlecats", &self.config.export_filename)
            .on_input(Message::ExportFilenameChanged)
            .width(Length::Fixed(150.0));
        let file_row = row![text("Filename: "), file_input, text(".tar.zst")]
            .align_y(Alignment::Center)
            .spacing(10);

        let max_compression = if settings.game_data.enable_ultra_compression { 21 } else { 15 };
        let current_compression = if self.config.compression_level == 0 {
            settings.game_data.last_compression_level
        } else if self.config.compression_level > max_compression {
            max_compression
        } else {
            self.config.compression_level
        };

        let comp_slider = slider(
            1..=max_compression,
            current_compression,
            Message::CompressionLevelChanged,
        )
            .width(Length::Fixed(200.0));

        let comp_row = row![text("Compression Level: "), comp_slider]
            .align_y(Alignment::Center)
            .spacing(10);

        let (desc_text, is_success) = match current_compression {
            1..=9 => ("Best compression balance", true),
            10..=15 => ("Slow compression for low archive size", false),
            _ => ("Ultra compression granting minimal returns", false),
        };

        let desc_label = text(desc_text)
            .size(14)
            .style(if is_success { text::success } else { text::danger });

        let show_success = self
            .config
            .export_job_completed_time
            .is_some_and(|time| time.elapsed().as_secs() < 2);
        let show_aborted = self
            .config
            .export_job_aborted_time
            .is_some_and(|time| time.elapsed().as_secs() < 2);
        let is_aborting = is_running && self.config.export_abort_flag.load(Ordering::Relaxed);

        let base_filename = if self.config.export_filename.trim().is_empty() {
            "battlecats"
        } else {
            &self.config.export_filename
        };
        let full_filename = format!("{}.tar.zst", base_filename);

        let action_btn = if show_success {
            button(text("Job Complete!").size(18))
                .style(button::success)
                .width(Length::Fixed(300.0))
                .on_press(Message::TriggerExportJob)
        } else if show_aborted {
            button(text("Job Aborted!").size(18))
                .style(button::danger)
                .width(Length::Fixed(300.0))
                .on_press(Message::TriggerExportJob)
        } else if is_aborting {
            button(text("Aborting Job...").size(18))
                .style(button::danger)
                .width(Length::Fixed(300.0))
        } else if is_running {
            button(text("Abort Job").size(18))
                .style(button::danger)
                .width(Length::Fixed(300.0))
                .on_press(Message::AbortExportJob)
        } else {
            button(text(format!("Create {}", full_filename)).size(18))
                .style(button::primary)
                .width(Length::Fixed(300.0))
                .on_press(Message::TriggerExportJob)
        };

        let action_row = container(action_btn).width(Length::Fill).center_x(Length::Fill);

        let controls = column![
            title,
            Space::new().height(10),
            toggle_raw,
            Space::new().height(10),
            file_row,
            Space::new().height(10),
            comp_row,
            desc_label,
            Space::new().height(20),
            action_row
        ];

        controls.into()
    }

    fn view_progress_and_console(&self) -> Element<'_, Message> {
        let (is_running, log_content, cur, max) = match self.config.active_tab {
            DataTab::Import => (
                self.config.import_job_status.load(Ordering::Relaxed) == 1,
                &self.config.import_log_content,
                self.config.import_progress_current.load(Ordering::Relaxed),
                self.config.import_progress_maximum.load(Ordering::Relaxed),
            ),
            DataTab::Export => (
                self.config.export_job_status.load(Ordering::Relaxed) == 1,
                &self.config.export_log_content,
                self.config.export_progress_current.load(Ordering::Relaxed),
                self.config.export_progress_maximum.load(Ordering::Relaxed),
            ),
        };

        let progress_fraction = if is_running {
            if max > 0 {
                cur as f32 / max as f32
            } else {
                1.0
            }
        } else {
            1.0
        };

        let progress = progress_bar(0.0..=1.0, progress_fraction);

        let console_area = scrollable(
            container(text(log_content).size(12).font(Font::MONOSPACE))
                .width(Length::Fill)
                .padding(5),
        )
            .height(Length::Fill);

        column![progress, Space::new().height(10), console_area].into()
    }

    fn trigger_import_job(&mut self, settings: &Settings) {
        self.config.import_job_status.store(1, Ordering::Relaxed);
        self.config.import_abort_flag.store(false, Ordering::Relaxed);
        self.config.import_progress_current.store(0, Ordering::Relaxed);
        self.config.import_progress_maximum.store(0, Ordering::Relaxed);
        self.config.import_log_content.clear();
        self.config.import_job_completed_time = None;
        self.config.import_job_aborted_time = None;

        let (sender, receiver) = mpsc::channel();
        self.config.import_rx = Some(receiver);

        let abort = self.config.import_abort_flag.clone();
        let status = self.config.import_job_status.clone();
        let progress_current = self.config.import_progress_current.clone();
        let progress_max = self.config.import_progress_maximum.clone();
        let enforce_val = settings.game_data.enforce_key_validation;

        match self.config.selected_job {
            Some(ImportSubTab::Emulator) => {
                let mode = if settings.game_data.adb_import_type_idx == 1 {
                    AdbImportType::Update
                } else {
                    AdbImportType::All
                };
                let region = match settings.game_data.adb_region_idx {
                    0 => AdbTarget::Specific(Region::En),
                    1 => AdbTarget::Specific(Region::Ja),
                    2 => AdbTarget::Specific(Region::Tw),
                    3 => AdbTarget::Specific(Region::Ko),
                    _ => AdbTarget::All,
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
                    progress_max,
                );
            }
            Some(ImportSubTab::Decrypt) => {
                let folder_path = self.config.decrypt_path.clone();
                let mode = ImportMode::Folder;
                let region = self.config.adb_target.clone();

                thread::spawn(move || {
                    let result = pack::run(
                        &folder_path,
                        mode,
                        region,
                        enforce_val,
                        sender,
                        abort,
                        progress_current,
                        progress_max,
                    );

                    if result.is_err() {
                        status.store(3, Ordering::Relaxed);
                    } else {
                        status.store(2, Ordering::Relaxed);
                    }
                });
            }
            Some(ImportSubTab::Sort) => {
                let data_path = self.config.import_path.clone();
                let lang_priority = settings.general.language_priority.clone();

                thread::spawn(move || {
                    let result = raw::run(
                        &data_path,
                        sender,
                        abort,
                        progress_current,
                        progress_max,
                        &lang_priority,
                    );

                    if result.is_err() {
                        status.store(3, Ordering::Relaxed);
                    } else {
                        status.store(2, Ordering::Relaxed);
                    }
                });
            }
            None => {}
        }
    }

    fn trigger_export_job(&mut self) {
        self.config.export_job_status.store(1, Ordering::Relaxed);
        self.config.export_abort_flag.store(false, Ordering::Relaxed);
        self.config.export_progress_current.store(0, Ordering::Relaxed);
        self.config.export_progress_maximum.store(0, Ordering::Relaxed);
        self.config.export_log_content.clear();
        self.config.export_job_completed_time = None;
        self.config.export_job_aborted_time = None;

        let (sender, receiver) = mpsc::channel();
        self.config.export_rx = Some(receiver);

        let compression_level = self.config.compression_level;
        let include_raw = self.config.include_raw;
        let status = self.config.export_job_status.clone();
        let abort = self.config.export_abort_flag.clone();
        let progress_current = self.config.export_progress_current.clone();
        let progress_maximum = self.config.export_progress_maximum.clone();

        let base_filename = if self.config.export_filename.trim().is_empty() {
            "battlecats"
        } else {
            &self.config.export_filename
        };
        let full_filename = format!("{}.tar.zst", base_filename);

        thread::spawn(move || {
            let result = export::create_game_archive(
                sender.clone(),
                abort.clone(),
                progress_current,
                progress_maximum,
                compression_level,
                full_filename,
                include_raw,
            );

            if let Err(error) = result {
                let _ = sender.send(format!("Error Packing: {}", error));
                status.store(3, Ordering::Relaxed);
            } else if !abort.load(Ordering::Relaxed) {
                status.store(2, Ordering::Relaxed);
            } else {
                status.store(3, Ordering::Relaxed);
            }
        });
    }
}

pub fn censor_path(path_string: &str) -> String {
    if path_string.is_empty() || path_string == "No source selected" {
        return String::new();
    }

    let mut clean_string = path_string.to_string();
    if let Ok(username) = env::var("USERNAME").or_else(|_| env::var("USER")) {
        if !username.is_empty() {
            clean_string = clean_string.replace(&username, "***");
        }
    }

    let path_object = Path::new(&clean_string);
    let path_components: Vec<_> = path_object
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect();

    if path_components.len() < 2 {
        if clean_string.chars().count() > 20 {
            return format!(
                "...{}",
                clean_string
                    .chars()
                    .skip(clean_string.chars().count() - 20)
                    .collect::<String>()
            );
        }
        return clean_string;
    }

    let mut parent_folder = path_components[path_components.len() - 2].to_string();
    let mut target_file = path_components[path_components.len() - 1].to_string();

    let total_length = parent_folder.chars().count() + target_file.chars().count();

    if total_length > 20 {
        if target_file.chars().count() >= 20 {
            target_file = format!("{}...", target_file.chars().take(18).collect::<String>());
            parent_folder = String::new();
        } else {
            let allowed_parent_length = 20 - target_file.chars().count();
            if allowed_parent_length > 2 {
                parent_folder = format!(
                    "{}...",
                    parent_folder
                        .chars()
                        .take(allowed_parent_length - 2)
                        .collect::<String>()
                );
            } else {
                parent_folder = String::new();
            }
        }
    }

    let ellipsis_prefix = if path_components.len() > 2 { "...\\" } else { "" };

    if parent_folder.is_empty() {
        format!("{}{}", ellipsis_prefix, target_file)
    } else {
        format!("{}{}\\{}", ellipsis_prefix, parent_folder, target_file)
    }
}