use std::fs;
use std::process::Command;
use std::thread;

use eframe::egui;
use self_update::backends::github::{ReleaseList, Update as GithubUpdate};
use self_update::cargo_crate_version;
use self_update::update::Release;
use self_update::version;
use tracing::{error, info};

use core::modules::settings::{Settings, UpdateMode};

use crate::common::DragGuard;

use super::{UpdateStatus, Updater, UpdaterMsg};

const REPO_OWNER: &str = "omochikaeri15";
const REPO_NAME: &str = "battle-cats-complete";
const BIN_NAME: &str = "Battle Cats Complete";

const PROMPT_BUTTON_SIZE: [f32; 2] = [80.0, 40.0];
const RESTART_BUTTON_SIZE: [f32; 2] = [80.0, 40.0];

pub(super) fn cleanup_temp_files() {
    let temp_files = [
        "tmp_update.zip",
        "tmp_new_version.exe",
        "tmp_new_version",
    ];

    for file in temp_files {
        let _ = fs::remove_file(file);
    }
}

#[cfg(unix)]
fn restart_app() {
    info!("Executing unix restart sequence");
    let Ok(exe) = std::env::current_exe() else { return; };
    let path = exe.to_string_lossy();
    let clean_path = path.trim_end_matches(" (deleted)");
    let _ = Command::new("sh")
        .arg("-c")
        .arg(format!("sleep 1 && \"{}\" &", clean_path))
        .spawn();

    std::process::exit(0);
}

#[cfg(not(unix))]
fn restart_app() {
    info!("Executing non-unix restart sequence");
    let Ok(exe) = std::env::current_exe() else { return; };
    let _ = Command::new(exe).spawn();
    std::process::exit(0);
}

impl Updater {
    pub(crate) fn check_for_updates(&mut self, ctx: egui::Context, is_manual: bool) {
        let is_valid_state = matches!(self.status, UpdateStatus::Idle | UpdateStatus::UpToDate | UpdateStatus::CheckFailed);
        if !is_valid_state { return; }

        info!("Checking Github for releases...");

        let tx = self.tx.clone();
        self.status = UpdateStatus::Checking;

        thread::spawn(move || {
            match check_remote() {
                Ok(Some(release)) => {
                    info!("Found new release: {}", release.version);
                    let _ = tx.send(UpdaterMsg::UpdateFound(release));
                },
                Ok(None) if is_manual => {
                    info!("Software is up to date");
                    let _ = tx.send(UpdaterMsg::UpToDate);
                },
                Ok(None) => { let _ = tx.send(UpdaterMsg::SilentFail); },
                Err(err) if is_manual => {
                    error!("Update check failed: {}", err);
                    let _ = tx.send(UpdaterMsg::CheckFailed);
                }
                Err(_) => { let _ = tx.send(UpdaterMsg::SilentFail); }
            }

            ctx.request_repaint();
        });
    }

    pub(crate) fn download_and_install(&mut self, release: Release) {
        let tx = self.tx.clone();
        let target_version = release.version.clone();
        self.status = UpdateStatus::Downloading(target_version.clone());

        info!("Initializing download process for version: {}", target_version);

        thread::spawn(move || {
            cleanup_temp_files();
            let _ = tx.send(UpdaterMsg::DownloadStarted(target_version.clone()));

            let target_tag = if target_version.starts_with('v') { target_version.clone() } else { format!("v{}", target_version) };

            let target_asset_name = match () {
                _ if cfg!(target_os = "windows") => "bcc_windows.zip",
                _ if cfg!(target_os = "macos") => "bcc_mac.zip",
                _ => "bcc_linux.zip",
            };

            let Ok(update_box) = GithubUpdate::configure()
                .repo_owner(REPO_OWNER)
                .repo_name(REPO_NAME)
                .bin_name(BIN_NAME)
                .show_download_progress(false)
                .show_output(false)
                .no_confirm(true)
                .current_version(cargo_crate_version!())
                .target_version_tag(&target_tag)
                .target(target_asset_name)
                .build() else {
                cleanup_temp_files();
                error!("Failed to build download configurator");
                let _ = tx.send(UpdaterMsg::CheckFailed);
                return;
            };

            if update_box.update().is_err() {
                cleanup_temp_files();
                error!("Failed during update installation sequence");
                let _ = tx.send(UpdaterMsg::CheckFailed);
                return;
            }

            info!("Download and extraction finished");
            cleanup_temp_files();
            let _ = tx.send(UpdaterMsg::DownloadFinished(target_version));
        });
    }

    pub(crate) fn update_state(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                UpdaterMsg::UpdateFound(release) => {
                    self.status = UpdateStatus::UpdateFound(release.version.clone(), release);
                },
                UpdaterMsg::UpToDate => {
                    self.status = UpdateStatus::UpToDate;
                    self.clear_time = Some(ctx.input(|i| i.time) + 2.0);
                },
                UpdaterMsg::CheckFailed => {
                    self.status = UpdateStatus::CheckFailed;
                    self.clear_time = Some(ctx.input(|i| i.time) + 2.0);
                },
                UpdaterMsg::DownloadStarted(version) => {
                    self.status = UpdateStatus::Downloading(version);
                },
                UpdaterMsg::DownloadFinished(version) => {
                    self.status = UpdateStatus::RestartPending(version);
                },
                UpdaterMsg::SilentFail => {
                    self.status = UpdateStatus::Idle;
                }
            }
        }

        let Some(clear_time) = self.clear_time else { return; };

        if ctx.input(|i| i.time) >= clear_time {
            self.status = UpdateStatus::Idle;
            self.clear_time = None;
        }

        ctx.request_repaint();
    }

    pub(crate) fn show_ui(&mut self, ctx: &egui::Context, settings: &mut Settings, drag_guard: &mut DragGuard) {
        let status = self.status.clone();
        match status {
            UpdateStatus::UpdateFound(tag, release) => {
                self.show_update_found_window(ctx, settings, drag_guard, tag, release);
            }
            UpdateStatus::Downloading(tag) => {
                self.show_downloading_window(ctx, drag_guard, tag);
            }
            UpdateStatus::RestartPending(tag) => {
                self.show_restart_pending_window(ctx, settings, drag_guard, tag);
            }
            _ => {}
        }
    }

    fn show_update_found_window(&mut self, ctx: &egui::Context, settings: &mut Settings, drag_guard: &mut DragGuard, tag: String, release: Release) {
        if matches!(settings.general.update_mode, UpdateMode::AutoReset | UpdateMode::AutoLoad) {
            self.download_and_install(release);
            return;
        }

        ctx.request_repaint();
        let mut start_download = false;
        let mut close_modal = false;
        let mut disable_future = false;
        let display_version = if tag.starts_with('v') { tag.clone() } else { format!("v{}", tag) };
        let screen_rect = ctx.screen_rect();

        let window_id = egui::Id::new(format!("Update_Available_{}", tag));
        let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);

        let mut window = egui::Window::new("Update Available")
            .id(window_id)
            .collapsible(false).resizable(false).order(egui::Order::Tooltip)
            .constrain(false).movable(allow_drag)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen_rect.center());

        if let Some(pos) = fixed_pos { window = window.current_pos(pos); }

        window.show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(format!("New Battle Cats Complete update found: {}", display_version));
                ui.add_space(10.0);
                ui.label("Would you like to download the update now?");
            });
            ui.add_space(20.0);

            let mut style: egui::Style = (**ui.style()).clone();
            style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
            style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
            style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
            ui.set_style(style);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;

                let button_width = PROMPT_BUTTON_SIZE[0];
                let button_count = 3.0;
                let spacing = 10.0;
                let total_width = (button_width * button_count) + (spacing * (button_count - 1.0));

                let available_width = ui.available_width();
                let margin_left = (available_width - total_width) / 2.0;

                ui.add_space(margin_left.max(0.0));

                if ui.add_sized(PROMPT_BUTTON_SIZE, egui::Button::new("Yes")).clicked() { start_download = true; }
                if ui.add_sized(PROMPT_BUTTON_SIZE, egui::Button::new("No")).clicked() { close_modal = true; }
                if ui.add_sized(PROMPT_BUTTON_SIZE, egui::Button::new("Never")).clicked() {
                    close_modal = true;
                    disable_future = true;
                }
            });
        });

        if start_download { self.download_and_install(release); }
        if disable_future {
            info!("User selected Never update, changing mode to Ignore");
            settings.general.update_mode = UpdateMode::Ignore;
            close_modal = true;
        }
        if close_modal { self.status = UpdateStatus::Idle; }
    }

    fn show_downloading_window(&self, ctx: &egui::Context, drag_guard: &mut DragGuard, tag: String) {
        ctx.request_repaint();
        let screen_rect = ctx.screen_rect();

        let window_id = egui::Id::new(format!("Downloading_Update_{}", tag));
        let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);

        let mut window = egui::Window::new("Downloading Update")
            .id(window_id)
            .collapsible(false).resizable(false).title_bar(false)
            .order(egui::Order::Tooltip).constrain(false).movable(allow_drag)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen_rect.center());

        if let Some(pos) = fixed_pos { window = window.current_pos(pos); }

        window.show(ctx, |ui| {
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                let display_tag = if tag.starts_with('v') { tag.clone() } else { format!("v{}", tag) };
                ui.label(format!("Downloading {}...", display_tag));
                ui.add_space(10.0);
                let progress = (ctx.input(|i| i.time) % 1.0) as f32;
                ui.add(egui::ProgressBar::new(progress).animate(false));
            });
        });
    }

    fn show_restart_pending_window(&mut self, ctx: &egui::Context, settings: &Settings, drag_guard: &mut DragGuard, tag: String) {
        if matches!(settings.general.update_mode, UpdateMode::AutoReset) {
            restart_app();
            return;
        }
        if matches!(settings.general.update_mode, UpdateMode::AutoLoad) {
            self.status = UpdateStatus::Idle;
            return;
        }

        ctx.request_repaint();
        let mut should_restart = false;
        let mut close_modal = false;
        let display_tag = if tag.starts_with('v') { tag.clone() } else { format!("v{}", tag) };
        let screen_rect = ctx.screen_rect();

        let window_id = egui::Id::new(format!("Update_Complete_{}", tag));
        let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);

        let mut window = egui::Window::new("Update Complete")
            .id(window_id)
            .collapsible(false).resizable(false).order(egui::Order::Tooltip)
            .constrain(false).movable(allow_drag)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen_rect.center());

        if let Some(pos) = fixed_pos { window = window.current_pos(pos); }

        window.show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(format!("{} update complete!", display_tag));
                ui.add_space(5.0);
                ui.label("Would you like to restart and apply the update now?");
            });
            ui.add_space(20.0);

            let mut style: egui::Style = (**ui.style()).clone();
            style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
            style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);
            style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
            ui.set_style(style);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;

                let button_width = RESTART_BUTTON_SIZE[0];
                let button_count = 2.0;
                let spacing = 10.0;
                let total_width = (button_width * button_count) + (spacing * (button_count - 1.0));

                let available_width = ui.available_width();
                let margin_left = (available_width - total_width) / 2.0;

                ui.add_space(margin_left.max(0.0));

                if ui.add_sized(RESTART_BUTTON_SIZE, egui::Button::new("Yes")).clicked() { should_restart = true; }
                if ui.add_sized(RESTART_BUTTON_SIZE, egui::Button::new("No")).clicked() { close_modal = true; }
            });
        });

        if should_restart { restart_app(); }
        if close_modal { self.status = UpdateStatus::Idle; }
    }
}

fn check_remote() -> Result<Option<Release>, Box<dyn std::error::Error>> {
    let current_version = cargo_crate_version!();
    let releases = ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()?
        .fetch()?;

    let Some(latest_release) = releases.first() else { return Ok(None); };

    if !version::bump_is_greater(current_version, &latest_release.version)? {
        return Ok(None);
    }

    Ok(Some(latest_release.clone()))
}