use std::env;
use std::fs;
use std::process::Command;
use std::thread;

use iced::futures::channel::mpsc;
use iced::Task;
use self_update::{cargo_crate_version, version};
use self_update::backends::github::{ReleaseList, Update as GithubUpdate};
use self_update::update::Release;
use tracing::{error, info};

use super::{BattleCatsApp, Message, UpdateStatus, UpdaterMsg};

const REPO_OWNER: &str = "omochikaeri15";
const REPO_NAME: &str = "battle-cats-complete";
const BIN_NAME: &str = "Battle Cats Complete";

impl BattleCatsApp {
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

        info!("Initializing download process for version: {}", target_version);

        let (tx, rx) = mpsc::unbounded();

        thread::spawn(move || {
            cleanup_temp_files();
            let _ = tx.unbounded_send(UpdaterMsg::DownloadStarted(target_version.clone()));

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
    info!("Executing unix restart sequence");
    let Ok(exe) = env::current_exe() else { return; };
    let path = exe.to_string_lossy();
    let clean_path = path.trim_end_matches(" (deleted)");
    let _ = Command::new("sh")
        .arg("-c")
        .arg(format!("sleep 1 && \"{}\" &", clean_path))
        .spawn();

    std::process::exit(0);
}

#[cfg(not(unix))]
pub(crate) fn restart_app() {
    info!("Executing non-unix restart sequence");
    let Ok(exe) = env::current_exe() else { return; };
    let _ = Command::new(exe).spawn();
    std::process::exit(0);
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