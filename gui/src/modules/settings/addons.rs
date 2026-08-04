use std::thread;

use iced::futures::channel::mpsc;
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Task, Theme};
use tracing::error;

use core::addons::adb::AdbManager;
use core::addons::apkeditor::ApkeditorManager;
use core::addons::avifenc::AvifManager;
use core::addons::ffmpeg::FfmpegManager;
#[cfg(target_os = "windows")]
use core::addons::oem::{OemDriver, OemManager};
use core::addons::{manager, AddonStatus};

use crate::app::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Addon {
    Adb,
    Apkeditor,
    Ffmpeg,
    Avif,
}

impl Addon {
    fn label(self) -> &'static str {
        match self {
            Self::Adb => "ADB",
            Self::Apkeditor => "APKEditor",
            Self::Ffmpeg => "FFMPEG",
            Self::Avif => "AVIFENC",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Install(Addon),
    Download(Addon, AddonStatus),
    RequestDelete(Addon),
    ConfirmDelete,
    CancelDelete,
    #[cfg(target_os = "windows")]
    OemDriverSelected(OemDriver),
    #[cfg(target_os = "windows")]
    OemAction,
}

#[derive(Default)]
pub struct State {
    adb: AdbManager,
    apkeditor: ApkeditorManager,
    avif: AvifManager,
    ffmpeg: FfmpegManager,
    #[cfg(target_os = "windows")]
    oem: OemManager,
    pending_delete: Option<Addon>,
}

impl State {
    pub fn is_modal_open(&self) -> bool {
        self.pending_delete.is_some()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Install(addon) => {
                let config = match addon {
                    Addon::Adb => self.adb.install(),
                    Addon::Apkeditor => self.apkeditor.install(),
                    Addon::Ffmpeg => self.ffmpeg.install(),
                    Addon::Avif => self.avif.install(),
                };

                let (tx, rx) = mpsc::unbounded();

                thread::spawn(move || {
                    let result = manager::download(config, |status| {
                        let _ = tx.unbounded_send(status);
                    });

                    let terminal = result.map_or_else(
                        |err| {
                            error!("Addon download failed: {}", err);
                            AddonStatus::Error(err)
                        },
                        |()| AddonStatus::Installed,
                    );
                    let _ = tx.unbounded_send(terminal);
                });

                Task::stream(rx).map(move |status| Message::Download(addon, status))
            }
            Message::Download(addon, status) => {
                let slot = match addon {
                    Addon::Adb => &mut self.adb.status,
                    Addon::Apkeditor => &mut self.apkeditor.status,
                    Addon::Ffmpeg => &mut self.ffmpeg.status,
                    Addon::Avif => &mut self.avif.status,
                };
                *slot = status;
                Task::none()
            }
            Message::RequestDelete(addon) => {
                self.pending_delete = Some(addon);
                Task::none()
            }
            Message::CancelDelete => {
                self.pending_delete = None;
                Task::none()
            }
            Message::ConfirmDelete => {
                if let Some(addon) = self.pending_delete.take() {
                    match addon {
                        Addon::Adb => self.adb.uninstall(),
                        Addon::Apkeditor => self.apkeditor.uninstall(),
                        Addon::Ffmpeg => self.ffmpeg.uninstall(),
                        Addon::Avif => self.avif.uninstall(),
                    }
                }
                Task::none()
            }
            #[cfg(target_os = "windows")]
            Message::OemDriverSelected(driver) => {
                self.oem.selected = driver;
                Task::none()
            }
            #[cfg(target_os = "windows")]
            Message::OemAction => {
                self.oem.execute_action();
                Task::none()
            }
        }
    }

    fn addon_section<'a>(&'a self, addon: Addon, status: &'a AddonStatus, description: &'a str) -> Element<'a, Message> {
        let controls: Element<'a, Message> = match status {
            AddonStatus::Installed => {
                button(text(format!("Delete {}", addon.label())))
                    .padding([6, 14])
                    .style(button::danger)
                    .on_press(Message::RequestDelete(addon))
                    .into()
            }
            AddonStatus::Downloading(progress, stage) => {
                column![
                    button(text(format!("Downloading {}...", addon.label())))
                        .padding([6, 14])
                        .style(button::secondary),
                    text(format!("{} ({:.0}%)", stage, progress * 100.0)).size(12),
                ].spacing(4).into()
            }
            AddonStatus::NotInstalled | AddonStatus::Error(_) => {
                let mut col = column![
                    button(text(format!("Download {}", addon.label())))
                        .padding([6, 14])
                        .style(button::success)
                        .on_press(Message::Install(addon))
                ].spacing(4);
                if let AddonStatus::Error(err) = status {
                    col = col.push(text(format!("Error: {}", err)).size(12).style(|theme: &Theme| {
                        text::Style { color: Some(theme.palette().danger) }
                    }));
                }
                col.into()
            }
        };

        column![
            text(addon.label()).size(20),
            text(description).size(13),
            controls,
        ].spacing(6).into()
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let mut content = column![
            self.addon_section(
                Addon::Adb, &self.adb.status,
                "Enables \"Android\" option for Game Data Import allowing Android Device & Emulator imports.\nMake sure you have \"USB Debugging\" or \"Wireless Debugging\" enabled on your Android device."
            ),
        ].spacing(20);

        #[cfg(target_os = "windows")]
        {
            let drivers = OemManager::all_drivers();
            let btn_text = if self.oem.selected == OemDriver::Universal { "Download Installer" } else { "Open Download Page" };
            content = content.push(
                column![
                    text("ADB OEM Drivers").size(20),
                    text("Allows Windows devices to connect to a real Android device for game files during \"Android\" export method.\nWindows only, requires Android Bridge Add-On, and manual set-up.").size(13),
                    row![
                        iced::widget::pick_list(
                            drivers.iter().map(|d| OemManager::label(*d)).collect::<Vec<_>>(),
                            Some(OemManager::label(self.oem.selected)),
                            |label| {
                                let driver = OemManager::all_drivers().into_iter()
                                    .find(|d| OemManager::label(*d) == label)
                                    .unwrap_or_default();
                                Message::OemDriverSelected(driver)
                            }
                        ).style(theme::combo_box).menu_style(theme::combo_box_menu),
                        button(text(btn_text)).on_press(Message::OemAction),
                    ].spacing(10).align_y(Alignment::Center),
                ].spacing(6)
            );
        }

        content = content.push(self.addon_section(
            Addon::Apkeditor, &self.apkeditor.status,
            "Allows mod export to convert XAPK/APKM/APKS files into an APK.\nDownloads a portable JRE for you, falling back to system JRE upon failure."
        ));
        content = content.push(self.addon_section(
            Addon::Ffmpeg, &self.ffmpeg.status,
            "Optimizes encoding speed for most file formats.\nEnables most export formats."
        ));
        content = content.push(self.addon_section(
            Addon::Avif, &self.avif.status,
            "Optimizes encoding for the AVIF format specifically.\nEnables AVIF export format."
        ));

        content.into()
    }

    pub fn view_modal<'a>(&'a self) -> Element<'a, Message> {
        let name = self.pending_delete.map(Addon::label).unwrap_or_default();

        container(
            column![
                text(format!("Are you sure you want to delete {}?", name)).size(16),
                row![
                    button("Yes").on_press(Message::ConfirmDelete).style(button::danger),
                    button("No").on_press(Message::CancelDelete),
                ].spacing(10)
            ].spacing(20).padding(25).align_x(Alignment::Center)
        )
            .style(theme::confirm_modal_container)
            .into()
    }
}
