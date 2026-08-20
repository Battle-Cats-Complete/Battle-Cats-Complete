use std::env;
use std::fs;
#[cfg(unix)]
use std::path::Path;
use std::thread;
use std::time::Duration;

use iced::futures::channel::mpsc;
use iced::widget::{column, container, progress_bar, row, text, Space};
use iced::{Alignment, Color, Element, Length, Size, Task, Theme};
use self_update::{cargo_crate_version, version};
use self_update::backends::github::{ReleaseList, Update as GithubUpdate};
use self_update::update::Release;
use tracing::{error, info};

use kore::common::process;

use crate::widget::popup;

use super::{theme, BattleCatsApp, Message, UpdateStatus, UpdaterAction, UpdaterMsg};

const REPO_OWNER: &str = "omochikaeri15";
const REPO_NAME: &str = "battle-cats-complete";
const BIN_NAME: &str = "Battle Cats Complete";
const STATUS_EXPIRY: Duration = Duration::from_secs(2);
#[cfg(unix)]
const RESTART_DELAY_SECS: u8 = 1;
const MAX_RELEASE_LOOKBACK: usize = 8;

const POPUP: popup::Spec = popup::Spec::new(popup::Kind::Updater, Size::new(440.0, 250.0));
const POPUP_PADDING: f32 = 20.0;
const CONTENT_SPACING: f32 = 14.0;
const BUTTON_SPACING: f32 = 12.0;
const PROGRESS_WIDTH: f32 = 300.0;
const PROGRESS_HEIGHT: f32 = 12.0;
const TITLE_SIZE: f32 = 18.0;
const BODY_SIZE: f32 = 14.0;
const SUBTLE_ALPHA: f32 = 0.6;

impl BattleCatsApp {
    pub(crate) fn schedule_updater_status_expiry(&mut self) -> Task<Message> {
        if let Some(handle) = self.updater_status_handle.take() {
            handle.abort();
        }

        let (task, handle) = Task::perform(
            async { smol::Timer::after(STATUS_EXPIRY).await; },
            |_| Message::UpdaterStatusExpired,
        )
        .abortable();
        self.updater_status_handle = Some(handle);
        task
    }

    pub(crate) fn check_for_updates(&mut self, is_manual: bool) -> Task<Message> {
        let is_valid_state = matches!(self.updater_status, UpdateStatus::Idle | UpdateStatus::UpToDate | UpdateStatus::CheckFailed);
        if !is_valid_state { return Task::none(); }

        info!("Checking Github for releases...");
        self.updater_status = UpdateStatus::Checking;

        let (tx, rx) = mpsc::unbounded();

        thread::spawn(move || {
            match check_remote() {
                Ok(Some(release)) => {
                    info!("Found new release: {}", release.version);
                    let _ = tx.unbounded_send(UpdaterMsg::UpdateFound(release));
                },
                Ok(None) if is_manual => {
                    info!("Software is up to date");
                    let _ = tx.unbounded_send(UpdaterMsg::UpToDate);
                },
                Ok(None) => { let _ = tx.unbounded_send(UpdaterMsg::SilentFail); },
                Err(err) if is_manual => {
                    error!("Update check failed: {}", err);
                    let _ = tx.unbounded_send(UpdaterMsg::CheckFailed);
                }
                Err(_) => { let _ = tx.unbounded_send(UpdaterMsg::SilentFail); }
            }
        });

        let (task, handle) = Task::stream(rx).abortable();
        self.updater_handle = Some(handle);
        task.map(Message::Updater)
    }

    pub(crate) fn download_and_install(&mut self, release: Release) -> Task<Message> {
        let target_version = release.version.clone();
        self.updater_status = UpdateStatus::Downloading(target_version.clone());
        self.download_progress = 0.0;
        self.set_updater_popup(true);

        info!("Initializing download process for version: {}", target_version);

        let (tx, rx) = mpsc::unbounded();

        thread::spawn(move || {
            cleanup_temp_files();
            let _ = tx.unbounded_send(UpdaterMsg::DownloadStarted(target_version.clone()));

            let Some((target_tag, target_asset_name)) = resolve_download(&release) else {
                cleanup_temp_files();
                error!("No release within the last {} carries a usable asset for this platform", MAX_RELEASE_LOOKBACK);
                let _ = tx.unbounded_send(UpdaterMsg::CheckFailed);
                return;
            };

            info!("Installing asset {} from {}", target_asset_name, target_tag);

            let Ok(update_box) = GithubUpdate::configure()
                .repo_owner(REPO_OWNER)
                .repo_name(REPO_NAME)
                .bin_name(BIN_NAME)
                .show_download_progress(false)
                .show_output(false)
                .no_confirm(true)
                .current_version(cargo_crate_version!())
                .target_version_tag(&target_tag)
                .target(&target_asset_name)
                .build() else {
                cleanup_temp_files();
                error!("Failed to build download configurator");
                let _ = tx.unbounded_send(UpdaterMsg::CheckFailed);
                return;
            };

            if update_box.update().is_err() {
                cleanup_temp_files();
                error!("Failed during update installation sequence");
                let _ = tx.unbounded_send(UpdaterMsg::CheckFailed);
                return;
            }

            info!("Download and extraction finished");
            cleanup_temp_files();
            let _ = tx.unbounded_send(UpdaterMsg::DownloadFinished(target_version));
        });

        let (task, handle) = Task::stream(rx).abortable();
        self.updater_handle = Some(handle);
        task.map(Message::Updater)
    }
}

pub(super) fn update_popup(state: &mut popup::State, message: popup::Message) -> bool {
    state.update(message, POPUP)
}

pub(super) fn view<'a>(state: &'a popup::State, status: &'a UpdateStatus, window: Size, progress: f32) -> Option<Element<'a, Message>> {
    let title = match status {
        UpdateStatus::UpdateFound(..) => "Update Available",
        UpdateStatus::Downloading(_) => "Downloading Update",
        UpdateStatus::RestartPending(_) => "Update Complete",
        _ => return None,
    };

    Some(state.view(title, POPUP, window, Message::UpdaterPopup, move || {
        let body = match status {
            UpdateStatus::UpdateFound(tag, release) => view_update_found(tag, release),
            UpdateStatus::Downloading(tag) => view_downloading(tag, progress),
            UpdateStatus::RestartPending(tag) => view_restart_pending(tag),
            _ => Space::new().into(),
        };

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(POPUP_PADDING)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
    }, None))
}

fn display_version(tag: &str) -> String {
    if tag.starts_with('v') { tag.to_string() } else { format!("v{}", tag) }
}

fn subtle_text<'a>(content: impl ToString) -> Element<'a, Message> {
    text(content.to_string())
        .size(BODY_SIZE)
        .style(|theme: &Theme| text::Style { color: Some(Color { a: SUBTLE_ALPHA, ..theme.palette().text }) })
        .into()
}

fn view_update_found<'a>(tag: &str, release: &Release) -> Element<'a, Message> {
    let actions = row![
        theme::sized_button("Yes", theme::POPUP_ACTION_BUTTON_WIDTH, theme::success_button)
            .on_press(Message::UpdaterAction(UpdaterAction::StartDownload(release.clone()))),
        theme::sized_button("No", theme::POPUP_ACTION_BUTTON_WIDTH, theme::neutral_button)
            .on_press(Message::UpdaterAction(UpdaterAction::DismissUpdate)),
        theme::sized_button("Never", theme::POPUP_ACTION_BUTTON_WIDTH, theme::danger_button)
            .on_press(Message::UpdaterAction(UpdaterAction::NeverUpdate)),
    ]
    .spacing(BUTTON_SPACING);

    column![
        theme::bold_text(format!("Battle Cats Complete {}", display_version(&release.version))).size(TITLE_SIZE),
        subtle_text(format!("You are running v{} · found {}", cargo_crate_version!(), display_version(tag))),
        text("Would you like to download the update now?").size(BODY_SIZE),
        actions,
        subtle_text("\"Never\" stops all future update checks."),
    ]
    .spacing(CONTENT_SPACING)
    .align_x(Alignment::Center)
    .into()
}

fn view_downloading<'a>(tag: &str, progress: f32) -> Element<'a, Message> {
    column![
        theme::bold_text(format!("Downloading {}", display_version(tag))).size(TITLE_SIZE),
        progress_bar(0.0..=1.0, progress)
            .length(Length::Fixed(PROGRESS_WIDTH))
            .girth(Length::Fixed(PROGRESS_HEIGHT))
            .style(theme::progress_track),
        subtle_text("You will be prompted once it is ready to install."),
    ]
    .spacing(CONTENT_SPACING)
    .align_x(Alignment::Center)
    .into()
}

fn view_restart_pending<'a>(tag: &str) -> Element<'a, Message> {
    let actions = row![
        theme::sized_button("Yes", theme::POPUP_ACTION_BUTTON_WIDTH, theme::success_button)
            .on_press(Message::UpdaterAction(UpdaterAction::RestartApp)),
        theme::sized_button("No", theme::POPUP_ACTION_BUTTON_WIDTH, theme::neutral_button)
            .on_press(Message::UpdaterAction(UpdaterAction::DismissUpdate)),
    ]
    .spacing(BUTTON_SPACING);

    column![
        theme::bold_text(format!("{} is installed", display_version(tag))).size(TITLE_SIZE),
        text("Would you like to restart and apply the update now?").size(BODY_SIZE),
        actions,
        subtle_text("Declining keeps the update until the next launch."),
    ]
    .spacing(CONTENT_SPACING)
    .align_x(Alignment::Center)
    .into()
}

pub(crate) fn cleanup_temp_files() {
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
pub(crate) fn restart_app() {
    let Ok(exe) = env::current_exe() else {
        error!("Restart aborted: the running executable path could not be resolved");
        return;
    };

    let path = exe.to_string_lossy();
    let clean_path = path.trim_end_matches(" (deleted)");

    if !Path::new(clean_path).exists() {
        error!("Restart aborted: {} no longer exists on disk", clean_path);
        return;
    }

    info!("Executing unix restart sequence for {}", clean_path);

    let script = format!("sleep {}; exec \"$0\"", RESTART_DELAY_SECS);

    if let Err(err) = process::command("sh").arg("-c").arg(script).arg(clean_path).spawn() {
        error!("Restart aborted: could not spawn the relaunch helper: {}", err);
        return;
    }

    std::process::exit(0);
}

#[cfg(not(unix))]
pub(crate) fn restart_app() {
    let Ok(exe) = env::current_exe() else {
        error!("Restart aborted: the running executable path could not be resolved");
        return;
    };

    info!("Executing non-unix restart sequence for {}", exe.display());

    if let Err(err) = process::command(&exe).spawn() {
        error!("Restart aborted: could not spawn {}: {}", exe.display(), err);
        return;
    }

    std::process::exit(0);
}

fn platform() -> &'static str {
    match () {
        _ if cfg!(target_os = "windows") => "windows",
        _ if cfg!(target_os = "macos") => "mac",
        _ => "linux",
    }
}

fn asset_candidates() -> [String; 2] {
    [
        format!("bcc_{}.zip", platform()),
        format!("bcc_gui_{}={}_{}.zip", cargo_crate_version!(), kore::VERSION, platform()),
    ]
}

fn matching_asset(release: &Release) -> Option<String> {
    asset_candidates()
        .into_iter()
        .find(|candidate| release.assets.iter().any(|asset| asset.name == *candidate))
}

fn release_tag(version: &str) -> String {
    if version.starts_with('v') { version.to_string() } else { format!("v{}", version) }
}

fn resolve_download(selected: &Release) -> Option<(String, String)> {
    if let Some(target) = matching_asset(selected) {
        return Some((release_tag(&selected.version), target));
    }

    info!("Release {} carries no asset this build understands; walking back through older releases", selected.version);

    let releases = ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .and_then(|list| list.fetch())
        .inspect_err(|err| error!("Failed to list releases while resolving a download target: {}", err))
        .ok()?;

    let start = releases.iter().position(|entry| entry.version == selected.version).unwrap_or(0);

    releases
        .iter()
        .skip(start)
        .take(MAX_RELEASE_LOOKBACK)
        .find_map(|entry| matching_asset(entry).map(|target| (release_tag(&entry.version), target)))
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