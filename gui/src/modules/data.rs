use std::env;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use iced::futures::channel::mpsc;
use iced::task;
use iced::widget::{
    button, checkbox, column, container, pick_list, progress_bar, row, scrollable, slider, text,
    text_input, Space,
};
use iced::{Alignment, Element, Font, Length, Task, Theme};
use smol::Timer;
use tracing::{info, trace, warn};

use core::addons::paths::{self, Presence};
use core::common::job::{JobEvent, JobOutcome};
use core::common::region::Region;
use core::modules::data::{
    android, export, pack, raw, AdbImportType, AdbTarget, DataConfigState, DataTab, ImportMode,
    ImportSubTab,
};
use core::modules::settings::Settings;

use crate::app::state::AppState;
use crate::app::theme;

#[derive(Debug, Clone)]
pub enum Message {
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
    ImportJob(JobEvent),
    ImportBannerExpired,
    ToggleIncludeRaw(bool),
    ExportFilenameChanged(String),
    CompressionLevelChanged(i32),
    TriggerExportJob,
    AbortExportJob,
    ExportJob(JobEvent),
    ExportBannerExpired,
}

#[derive(Clone, Copy, PartialEq)]
enum Banner {
    Completed,
    Aborted,
}

#[derive(Default)]
struct JobSlot {
    running: bool,
    aborting: bool,
    log: String,
    progress: Option<(usize, usize)>,
    banner: Option<Banner>,
    abort: Arc<AtomicBool>,
    job_handle: Option<task::Handle>,
    banner_handle: Option<task::Handle>,
}

impl JobSlot {
    fn begin(&mut self) {
        self.running = true;
        self.aborting = false;
        self.log.clear();
        self.progress = None;
        self.banner = None;
        self.abort.store(false, Ordering::Relaxed);
        if let Some(handle) = self.banner_handle.take() {
            handle.abort();
        }
    }

    fn request_abort(&mut self) {
        self.aborting = true;
        self.abort.store(true, Ordering::Relaxed);
    }

    fn apply(&mut self, event: JobEvent, label: &str, expired: Message) -> Task<Message> {
        match event {
            JobEvent::Log(line) => {
                self.log.push_str(&line);
                self.log.push('\n');
                Task::none()
            }
            JobEvent::Progress { current, total } => {
                self.progress = Some((current, total));
                Task::none()
            }
            JobEvent::Finished(outcome) => {
                self.running = false;
                self.aborting = false;
                self.abort.store(false, Ordering::Relaxed);
                self.progress = None;
                self.job_handle = None;

                match outcome {
                    JobOutcome::Completed => {
                        info!("{} job completed.", label);
                        self.show_banner(Banner::Completed, expired)
                    }
                    JobOutcome::Aborted => {
                        info!("{} job aborted.", label);
                        self.show_banner(Banner::Aborted, expired)
                    }
                    JobOutcome::Failed(message) => {
                        warn!("{} job failed: {}", label, message);
                        self.log.push_str(&format!("Error: {}\n", message));
                        Task::none()
                    }
                }
            }
        }
    }

    fn show_banner(&mut self, banner: Banner, expired: Message) -> Task<Message> {
        self.banner = Some(banner);
        if let Some(handle) = self.banner_handle.take() {
            handle.abort();
        }

        let (banner_task, handle) = Task::perform(
            async {
                Timer::after(Duration::from_secs(2)).await;
            },
            move |_| expired,
        )
        .abortable();

        self.banner_handle = Some(handle);
        banner_task
    }
}

fn job_outcome(result: Result<(), String>, abort: &AtomicBool) -> JobOutcome {
    if abort.load(Ordering::Relaxed) {
        return JobOutcome::Aborted;
    }

    result.map_or_else(JobOutcome::Failed, |()| JobOutcome::Completed)
}

#[derive(Default)]
pub struct State {
    pub config: DataConfigState,
    pub import_censored: String,
    pub decrypt_censored: String,
    import: JobSlot,
    export: JobSlot,
}

impl State {
    pub fn update(&mut self, message: Message, settings: &mut Settings, app_state: &mut AppState) -> Task<Message> {
        match message {
            Message::TabSelected(tab) => {
                trace!("Switching data tab");
                self.config.active_tab = tab;
            }
            Message::ImportJobSelected(job) => {
                trace!("Selected import sub-job");
                self.config.selected_job = Some(job);
            }
            Message::AdbImportTypeChanged(idx) => {
                app_state.game_data.adb_import_type_idx = idx;
            }
            Message::AdbRegionChangedEmu(idx) => {
                app_state.game_data.adb_region_idx = idx;
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
                };

                if let Some(file_path) = dialog_result {
                    self.config.import_path = file_path.to_string_lossy().to_string();
                    self.import_censored = censor_path(&self.config.import_path);
                    info!("Selected import data path: {}", self.import_censored);
                }
            }
            Message::TriggerImportJob => {
                info!("Starting import job.");
                return self.trigger_import_job(settings, app_state);
            }
            Message::AbortImportJob => {
                warn!("Aborting import job.");
                self.import.request_abort();
            }
            Message::ImportJob(event) => {
                return self.import.apply(event, "Import", Message::ImportBannerExpired);
            }
            Message::ImportBannerExpired => {
                self.import.banner = None;
                self.import.banner_handle = None;
            }
            Message::ToggleIncludeRaw(include) => {
                self.config.include_raw = include;
            }
            Message::ExportFilenameChanged(name) => {
                self.config.export_filename = name;
            }
            Message::CompressionLevelChanged(level) => {
                self.config.compression_level = level;
            }
            Message::TriggerExportJob => {
                info!("Starting export job.");
                return self.trigger_export_job();
            }
            Message::AbortExportJob => {
                warn!("Aborting export job.");
                self.export.request_abort();
            }
            Message::ExportJob(event) => {
                return self.export.apply(event, "Export", Message::ExportBannerExpired);
            }
            Message::ExportBannerExpired => {
                self.export.banner = None;
                self.export.banner_handle = None;
            }
        }
        Task::none()
    }

    pub fn view(&self, app_state: &AppState) -> Element<'_, Message> {
        let is_import = self.config.active_tab == DataTab::Import;
        let is_export = self.config.active_tab == DataTab::Export;

        let tabs_row = row![
            button(text("Import").size(16))
                .style(move |t: &Theme, status| theme::toggle_button(t, status, is_import))
                .width(Length::Fixed(120.0))
                .on_press(Message::TabSelected(DataTab::Import)),
            button(text("Export").size(16))
                .style(move |t: &Theme, status| theme::toggle_button(t, status, is_export))
                .width(Length::Fixed(120.0))
                .on_press(Message::TabSelected(DataTab::Export)),
        ]
            .spacing(10);

        let content = if is_import {
            self.view_import(app_state)
        } else {
            self.view_export()
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

    fn view_import(&self, app_state: &AppState) -> Element<'_, Message> {
        let is_running = self.import.running;
        let adb_installed = paths::adb_status() == Presence::Installed;

        let android_btn = button(
            text("Android")
                .size(16)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        )
            .style(move |t: &Theme, status| theme::toggle_button(t, status, self.config.selected_job == Some(ImportSubTab::Emulator)))
            .on_press_maybe(if !is_running && adb_installed {
                Some(Message::ImportJobSelected(ImportSubTab::Emulator))
            } else {
                None
            });

        let import_types = vec!["All Content", "Update Only"];
        let current_type = if app_state.game_data.adb_import_type_idx == 1 {
            "Update Only"
        } else {
            "All Content"
        };
        let type_picker = pick_list(import_types, Some(current_type), |sel| {
            Message::AdbImportTypeChanged(if sel == "Update Only" { 1 } else { 0 })
        }).style(theme::combo_box).menu_style(theme::combo_box_menu);

        let emu_regions = vec!["Global", "Japan", "Taiwan", "Korea", "All Regions"];
        let emu_selected = emu_regions
            .get(app_state.game_data.adb_region_idx)
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
        }).style(theme::combo_box).menu_style(theme::combo_box_menu);

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
            .style(move |t: &Theme, status| theme::toggle_button(t, status, self.config.selected_job == Some(ImportSubTab::Decrypt)))
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
        }).style(theme::combo_box).menu_style(theme::combo_box_menu);

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
            .style(move |t: &Theme, status| theme::toggle_button(t, status, self.config.selected_job == Some(ImportSubTab::Sort)))
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
        }).style(theme::combo_box).menu_style(theme::combo_box_menu);

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

        let show_success = self.import.banner == Some(Banner::Completed);
        let show_aborted = self.import.banner == Some(Banner::Aborted);
        let is_aborting = is_running && self.import.aborting;

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
                .style(move |t: &Theme, status| theme::toggle_button(t, status, can_run))
                .width(Length::Fixed(300.0))
                .on_press_maybe(if can_run { Some(Message::TriggerImportJob) } else { None })
        };

        let action_row = container(action_btn).width(Length::Fill).center_x(Length::Fill);

        column![sections_row, Space::new().height(20), action_row].into()
    }

    fn view_export(&self) -> Element<'_, Message> {
        let is_running = self.export.running;

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

        let comp_slider = slider(
            1..=15,
            self.config.compression_level,
            Message::CompressionLevelChanged,
        )
            .width(Length::Fixed(200.0));

        let comp_row = row![text("Compression Level: "), comp_slider]
            .align_y(Alignment::Center)
            .spacing(10);

        let (desc_text, is_success) = match self.config.compression_level {
            1..=9 => ("Best compression balance", true),
            _ => ("Slow compression for low archive size", false),
        };

        let desc_label = text(desc_text)
            .size(14)
            .style(if is_success { text::success } else { text::danger });

        let show_success = self.export.banner == Some(Banner::Completed);
        let show_aborted = self.export.banner == Some(Banner::Aborted);
        let is_aborting = is_running && self.export.aborting;

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
        let slot = match self.config.active_tab {
            DataTab::Import => &self.import,
            DataTab::Export => &self.export,
        };

        let progress_fraction = if slot.running {
            match slot.progress {
                Some((current, total)) if total > 0 => current as f32 / total as f32,
                _ => 1.0,
            }
        } else {
            1.0
        };

        let progress = progress_bar(0.0..=1.0, progress_fraction);

        let console_area = scrollable(
            container(text(&slot.log).size(12).font(Font::MONOSPACE))
                .width(Length::Fill)
                .padding(5),
        )
            .height(Length::Fill);

        column![progress, Space::new().height(10), console_area].into()
    }

    fn trigger_import_job(&mut self, settings: &Settings, app_state: &AppState) -> Task<Message> {
        if self.import.running {
            return Task::none();
        }
        let Some(job) = self.config.selected_job else {
            return Task::none();
        };

        self.import.begin();

        let (tx, rx) = mpsc::unbounded();
        let abort = self.import.abort.clone();
        let enforce_val = settings.game_data.enforce_key_validation;

        match job {
            ImportSubTab::Emulator => {
                let mode = if app_state.game_data.adb_import_type_idx == 1 {
                    AdbImportType::Update
                } else {
                    AdbImportType::All
                };
                let region = match app_state.game_data.adb_region_idx {
                    0 => AdbTarget::Specific(Region::En),
                    1 => AdbTarget::Specific(Region::Ja),
                    2 => AdbTarget::Specific(Region::Tw),
                    3 => AdbTarget::Specific(Region::Ko),
                    _ => AdbTarget::All,
                };
                let emulator_config = settings.emulator_config();

                let thread_tx = tx.clone();
                let spawn_result = thread::Builder::new()
                    .name("android_import_worker".to_string())
                    .stack_size(8 * 1024 * 1024)
                    .spawn(move || {
                        let emit = |event: JobEvent| {
                            let _ = thread_tx.unbounded_send(event);
                        };
                        let result =
                            android::run(mode, region, emulator_config, enforce_val, emit, &abort);
                        emit(JobEvent::Finished(job_outcome(result, &abort)));
                    });

                if spawn_result.is_err() {
                    warn!("Failed to spawn android import worker thread.");
                    let _ = tx.unbounded_send(JobEvent::Finished(JobOutcome::Failed(
                        "Failed to spawn worker thread".to_string(),
                    )));
                }
            }
            ImportSubTab::Decrypt => {
                let folder_path = self.config.decrypt_path.clone();

                thread::spawn(move || {
                    let emit = |event: JobEvent| {
                        let _ = tx.unbounded_send(event);
                    };
                    let result = pack::run(
                        &folder_path,
                        ImportMode::Folder,
                        enforce_val,
                        emit,
                        &abort,
                    );
                    emit(JobEvent::Finished(job_outcome(result, &abort)));
                });
            }
            ImportSubTab::Sort => {
                let data_path = self.config.import_path.clone();
                let lang_priority = settings.general.language_priority.clone();

                thread::spawn(move || {
                    let emit = |event: JobEvent| {
                        let _ = tx.unbounded_send(event);
                    };
                    let result = raw::run(&data_path, emit, &abort, &lang_priority);
                    emit(JobEvent::Finished(job_outcome(result, &abort)));
                });
            }
        }

        let (stream_task, handle) = Task::stream(rx).abortable();
        self.import.job_handle = Some(handle);
        stream_task.map(Message::ImportJob)
    }

    fn trigger_export_job(&mut self) -> Task<Message> {
        if self.export.running {
            return Task::none();
        }

        self.export.begin();

        let (tx, rx) = mpsc::unbounded();
        let abort = self.export.abort.clone();
        let compression_level = self.config.compression_level;
        let include_raw = self.config.include_raw;

        let base_filename = if self.config.export_filename.trim().is_empty() {
            "battlecats"
        } else {
            &self.config.export_filename
        };
        let full_filename = format!("{}.tar.zst", base_filename);

        thread::spawn(move || {
            let emit = |event: JobEvent| {
                let _ = tx.unbounded_send(event);
            };
            let result =
                export::create_game_archive(emit, &abort, compression_level, full_filename, include_raw);
            emit(JobEvent::Finished(job_outcome(result, &abort)));
        });

        let (stream_task, handle) = Task::stream(rx).abortable();
        self.export.job_handle = Some(handle);
        stream_task.map(Message::ExportJob)
    }
}

fn censor_path(path_string: &str) -> String {
    if path_string.is_empty() || path_string == "No source selected" {
        return String::new();
    }

    let mut clean_string = path_string.to_string();
    if let Ok(username) = env::var("USERNAME").or_else(|_| env::var("USER"))
        && !username.is_empty() {
        clean_string = clean_string.replace(&username, "***");
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
