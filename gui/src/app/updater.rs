use std::env;
use std::fs;
use std::iter;
#[cfg(unix)]
use std::path::Path;
use std::thread;
use std::time::Duration;

use iced::futures::channel::mpsc;
use iced::widget::{column, container, progress_bar, row, text, Space};
use iced::{Alignment, Color, Element, Length, Size, Task, Theme};
use self_update::backends::github::Update as GithubUpdate;
use self_update::{cargo_crate_version, version};
use tracing::{error, info, warn};

use kore::common::github;
use kore::common::process;

use crate::common::feedback::CONFIRM_SHORT_LABEL;
use crate::widget::popup;

use super::{theme, BattleCatsApp, CheckFailure, Message, UpdateStatus, UpdateTarget, UpdaterAction, UpdaterMsg};

const REPO_OWNER: &str = "omochikaeri15";
const REPO_NAME: &str = "battle-cats-complete";
const BIN_NAME: &str = "Battle Cats Complete";
const STATUS_EXPIRY: Duration = Duration::from_secs(2);
#[cfg(unix)]
const RESTART_DELAY_SECS: u8 = 1;

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
        let is_valid_state = matches!(self.updater_status, UpdateStatus::Idle | UpdateStatus::UpToDate | UpdateStatus::CheckFailed(_));
        if !is_valid_state { return Task::none(); }

        info!("Checking Github for releases...");
        self.updater_status = UpdateStatus::Checking;

        let (tx, rx) = mpsc::unbounded();

        thread::spawn(move || {
            match check_remote() {
                Ok(Some(target)) => {
                    info!("Found new release: {}", target.tag);
                    let _ = tx.unbounded_send(UpdaterMsg::UpdateFound(target));
                },
                Ok(None) if is_manual => {
                    info!("Software is up to date");
                    let _ = tx.unbounded_send(UpdaterMsg::UpToDate);
                },
                Ok(None) => { let _ = tx.unbounded_send(UpdaterMsg::SilentFail); },
                Err(err) if is_manual => {
                    error!("Update check failed: {}", err.detail);
                    let _ = tx.unbounded_send(UpdaterMsg::CheckFailed(err.reason));
                }
                Err(_) => { let _ = tx.unbounded_send(UpdaterMsg::SilentFail); }
            }
        });

        let (task, handle) = Task::stream(rx).abortable();
        self.updater_handle = Some(handle);
        task.map(Message::Updater)
    }

    pub(crate) fn download_and_install(&mut self, target: UpdateTarget) -> Task<Message> {
        let target_version = target.version.clone();
        self.updater_status = UpdateStatus::Downloading(target_version.clone());
        self.download_progress = 0.0;
        self.set_updater_popup(true);

        info!("Initializing download process for version: {}", target_version);

        let (tx, rx) = mpsc::unbounded();

        thread::spawn(move || {
            cleanup_temp_files();
            let _ = tx.unbounded_send(UpdaterMsg::DownloadStarted(target_version.clone()));

            let UpdateTarget { tag: target_tag, asset: target_asset_name, .. } = target;

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
                let _ = tx.unbounded_send(UpdaterMsg::CheckFailed(CheckFailure::Unknown));
                return;
            };

            if update_box.update().is_err() {
                cleanup_temp_files();
                error!("Failed during update installation sequence");
                let _ = tx.unbounded_send(UpdaterMsg::CheckFailed(CheckFailure::Unknown));
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

pub(super) fn view<'a>(state: &'a popup::State, status: &'a UpdateStatus, window: Size, progress: f32, confirming_never: bool) -> Option<Element<'a, Message>> {
    let title = match status {
        UpdateStatus::UpdateFound(..) => "Update Available",
        UpdateStatus::Downloading(_) => "Downloading Update",
        UpdateStatus::RestartPending(_) => "Update Complete",
        _ => return None,
    };

    Some(state.view(title, POPUP, window, Message::UpdaterPopup, move || {
        let body = match status {
            UpdateStatus::UpdateFound(target) => view_update_found(target, confirming_never),
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

fn view_update_found<'a>(target: &UpdateTarget, confirming_never: bool) -> Element<'a, Message> {
    let never_label = if confirming_never { CONFIRM_SHORT_LABEL } else { "Never" };

    let actions = row![
        theme::sized_button("Yes", theme::POPUP_ACTION_BUTTON_WIDTH, theme::success_button)
            .on_press(Message::UpdaterAction(UpdaterAction::StartDownload(target.clone()))),
        theme::sized_button("No", theme::POPUP_ACTION_BUTTON_WIDTH, theme::neutral_button)
            .on_press(Message::UpdaterAction(UpdaterAction::DismissUpdate)),
        theme::sized_button(never_label, theme::POPUP_ACTION_BUTTON_WIDTH, theme::danger_button)
            .on_press(Message::UpdaterAction(UpdaterAction::NeverUpdate)),
    ]
    .spacing(BUTTON_SPACING);

    let stepping = (target.version != target.latest).then(|| {
        subtle_text(format!(
            "{} is out, but this build has to install {} first",
            display_version(&target.latest),
            display_version(&target.version)
        ))
    });

    column![
        theme::bold_text(format!("Battle Cats Complete {}", display_version(&target.version))).size(TITLE_SIZE),
        subtle_text(format!("You are running v{}", cargo_crate_version!())),
    ]
    .extend(stepping)
    .push(text("Would you like to download the update now?").size(BODY_SIZE))
    .push(actions)
    .push(subtle_text("\"Never\" stops all future update checks."))
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

pub(crate) fn cleanup_replace_artifacts() {
    let Ok(exe) = env::current_exe() else {
        return;
    };

    let Some(stem) = exe.file_stem().and_then(|stem| stem.to_str()) else {
        return;
    };

    let Some(dir) = exe.parent() else {
        return;
    };

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    let prefix = format!(".{stem}.");

    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };

        if !name.starts_with(&prefix) {
            continue;
        }

        let path = entry.path();

        match fs::remove_file(&path) {
            Ok(()) => info!(path = %path.display(), "Removed a leftover update artifact"),
            Err(err) => warn!(path = %path.display(), "Failed to remove a leftover update artifact: {}", err),
        }
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

fn matching_asset(release: &github::Release) -> Option<String> {
    asset_candidates()
        .into_iter()
        .find(|candidate| release.has_asset(candidate))
}

#[derive(Debug)]
struct CheckError {
    reason: CheckFailure,
    detail: String,
}

impl From<github::Error> for CheckError {
    fn from(err: github::Error) -> Self {
        let reason = match err.kind {
            github::ErrorKind::RateLimited => CheckFailure::RateLimited,
            github::ErrorKind::InvalidUrl => CheckFailure::InvalidUrl,
            github::ErrorKind::Network | github::ErrorKind::Malformed => CheckFailure::Unknown,
        };

        Self { reason, detail: err.to_string() }
    }
}

fn check_remote() -> Result<Option<UpdateTarget>, CheckError> {
    let current_version = cargo_crate_version!();

    if let Some(latest) = github::latest_release(REPO_OWNER, REPO_NAME)? {
        if !is_upgrade(current_version, &latest) {
            return Ok(None);
        }

        if let Some(asset) = matching_asset(&latest) {
            return Ok(Some(UpdateTarget {
                latest: latest.version().to_string(),
                version: latest.version().to_string(),
                tag: latest.tag_name,
                asset,
            }));
        }

        warn!("{} carries no asset this build understands; listing every release to find a step", latest.tag_name);
    }

    let releases = github::list_releases(REPO_OWNER, REPO_NAME)?;
    select_target(&releases, current_version)
}

fn is_upgrade(current_version: &str, release: &github::Release) -> bool {
    release.is_versioned()
        && !release.prerelease
        && version::bump_is_greater(current_version, release.version())
            .inspect_err(|err| warn!("Skipping release {}: {}", release.tag_name, err))
            .unwrap_or(false)
}

fn select_target(releases: &[github::Release], current_version: &str) -> Result<Option<UpdateTarget>, CheckError> {
    let mut newer = releases.iter().filter(|release| is_upgrade(current_version, release));

    let Some(latest) = newer.next() else { return Ok(None); };

    let Some((installable, asset)) = iter::once(latest)
        .chain(newer)
        .find_map(|release| matching_asset(release).map(|asset| (release, asset)))
    else {
        return Err(CheckError {
            reason: CheckFailure::Unknown,
            detail: format!(
                "{} is newer than v{}, but no release above it ships an asset this build can install",
                latest.tag_name, current_version
            ),
        });
    };

    if installable.tag_name != latest.tag_name {
        warn!(
            "{} carries no asset this build understands; stepping through {} first",
            latest.tag_name, installable.tag_name
        );
    }

    Ok(Some(UpdateTarget {
        latest: latest.version().to_string(),
        version: installable.version().to_string(),
        tag: installable.tag_name.clone(),
        asset,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, assets: &[&str]) -> github::Release {
        github::Release {
            tag_name: tag.to_string(),
            body: None,
            prerelease: false,
            assets: assets
                .iter()
                .map(|name| github::Asset { name: name.to_string() })
                .collect(),
        }
    }

    fn known_asset() -> String {
        format!("bcc_{}.zip", platform())
    }

    #[test]
    fn picks_the_newest_installable_release() {
        let asset = known_asset();
        let releases = [release("v3.0.0", &[&asset]), release("v2.9.0", &[&asset])];

        let target = select_target(&releases, "2.8.0").unwrap().unwrap();

        assert_eq!(target.tag, "v3.0.0");
        assert_eq!(target.latest, "3.0.0");
        assert_eq!(target.asset, asset);
    }

    // The point of the fallback: a rename on the newest release must not strand old builds.
    #[test]
    fn steps_back_to_the_last_release_this_build_understands() {
        let asset = known_asset();
        let releases = [
            release("v3.0.0", &["bcc-3.0.0-x86_64.tar.zst"]),
            release("v2.9.1", &["bcc-2.9.1-x86_64.tar.zst"]),
            release("v2.9.0", &[&asset]),
            release("v2.7.0", &[&asset]),
        ];

        let target = select_target(&releases, "2.8.0").unwrap().unwrap();

        assert_eq!(target.tag, "v2.9.0");
        assert_eq!(target.latest, "3.0.0");
    }

    #[test]
    fn never_walks_back_past_the_running_version() {
        let releases = [
            release("v3.0.0", &["bcc-3.0.0-x86_64.tar.zst"]),
            release("v2.7.0", &[&known_asset()]),
        ];

        assert!(select_target(&releases, "2.8.0").is_err());
    }

    #[test]
    fn ignores_prereleases_and_unversioned_tags() {
        let asset = known_asset();
        let mut beta = release("v3.0.0", &[&asset]);
        beta.prerelease = true;
        let releases = [beta, release("tools", &[&asset]), release("v2.9.0", &[&asset])];

        let target = select_target(&releases, "2.8.0").unwrap().unwrap();

        assert_eq!(target.tag, "v2.9.0");
    }

    #[test]
    fn reports_nothing_when_already_current() {
        let releases = [release("v2.8.0", &[&known_asset()])];

        assert!(select_target(&releases, "2.8.0").unwrap().is_none());
    }
}