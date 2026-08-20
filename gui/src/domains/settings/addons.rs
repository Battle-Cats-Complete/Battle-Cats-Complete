use std::thread;

use iced::futures::channel::mpsc;
use iced::widget::{column, text};
#[cfg(target_os = "windows")]
use iced::widget::{button, row};
#[cfg(target_os = "windows")]
use iced::Alignment;
use iced::{Element, Task, Theme};
use tracing::error;

use kore::systems::addons::adb::AdbManager;
use kore::systems::addons::apkeditor::ApkeditorManager;
use kore::systems::addons::avifenc::AvifManager;
use kore::systems::addons::ffmpeg::FfmpegManager;
#[cfg(target_os = "windows")]
use kore::systems::addons::oem::{OemDriver, OemManager};
use kore::systems::addons::{manager, AddonStatus};

use crate::app::theme;
use crate::common::feedback::{Slot as Confirm, CONFIRM_LABEL};

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
            Self::Adb => "Android Bridge",
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
    ConfirmExpired,
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
    confirm_delete: Confirm<Addon>,
}

impl State {
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
            Message::ConfirmExpired => {
                self.confirm_delete.expire();
                Task::none()
            }
            Message::RequestDelete(addon) => {
                if !self.confirm_delete.take(&addon) {
                    return self.confirm_delete.set(addon, Message::ConfirmExpired);
                }

                match addon {
                    Addon::Adb => self.adb.uninstall(),
                    Addon::Apkeditor => self.apkeditor.uninstall(),
                    Addon::Ffmpeg => self.ffmpeg.uninstall(),
                    Addon::Avif => self.avif.uninstall(),
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
                let label = if self.confirm_delete.armed_for(&addon) {
                    CONFIRM_LABEL.to_string()
                } else {
                    format!("Delete {}", addon.label())
                };

                theme::sized_button(label, theme::ACTION_BUTTON_WIDTH, theme::danger_button)
                    .on_press(Message::RequestDelete(addon))
                    .into()
            }
            AddonStatus::Downloading(progress, stage) => {
                column![
                    theme::sized_button(format!("Downloading {}...", addon.label()), theme::ACTION_BUTTON_WIDTH, theme::neutral_button),
                    text(format!("{} ({:.0}%)", stage, progress * 100.0)).size(12),
                ].spacing(4).into()
            }
            AddonStatus::NotInstalled | AddonStatus::Error(_) => {
                let mut col = column![
                    theme::sized_button(format!("Download {}", addon.label()), theme::ACTION_BUTTON_WIDTH, theme::success_button)
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

}
