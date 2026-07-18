use iced::alignment;
use iced::widget::{button, column, container, progress_bar, row, scrollable, stack, text};
use iced::{Color, Element, Length, Theme};

use super::{BattleCatsApp, Message, UpdateStatus, UpdaterAction, ALL_PAGES};

impl BattleCatsApp {
    pub fn view(&self) -> Element<Message> {
        let sidebar = if self.sidebar_open {
            let mut tabs = column![].spacing(10);
            for page in ALL_PAGES {
                tabs = tabs.push(
                    button(text(page.tab_name()))
                        .width(Length::Fill)
                        .on_press(Message::Navigate(*page))
                );
            }

            container(tabs)
                .width(Length::Fixed(200.0))
                .height(Length::Fill)
                .padding(10)
                .style(|theme: &Theme| {
                    let palette = theme.palette();
                    container::Style {
                        background: Some(palette.background.into()),
                        border: iced::border::rounded(0).color(palette.text).width(1),
                        ..Default::default()
                    }
                })
        } else {
            container(
                button(text(">"))
                    .on_press(Message::ToggleSidebar)
            )
                .height(Length::Fill)
                .padding(10)
        };

        let content = container(
            column![
                button(text(if self.sidebar_open { "Close Sidebar" } else { "Open Sidebar" }))
                    .on_press(Message::ToggleSidebar),
                text(format!("Welcome to the {} page!", self.current_page.tab_name())).size(30)
            ]
                .spacing(20)
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        let base_ui = row![sidebar, content]
            .width(Length::Fill)
            .height(Length::Fill);

        if let Some(modal) = self.build_modal() {
            stack![base_ui, modal].into()
        } else {
            base_ui.into()
        }
    }

    fn build_modal(&self) -> Option<Element<Message>> {
        let modal_content: Element<Message> = match &self.updater_status {
            UpdateStatus::UpdateFound(tag, release) => {
                let display_version = if tag.starts_with('v') { tag.clone() } else { format!("v{}", tag) };

                column![
                    text("Update Available").size(24),
                    text(format!("New Battle Cats Complete update found: {}", display_version)),
                    text("Would you like to download the update now?"),
                    row![
                        button("Yes").on_press(Message::UpdaterAction(UpdaterAction::StartDownload(release.clone()))),
                        button("No").on_press(Message::UpdaterAction(UpdaterAction::DismissUpdate)),
                        button("Never").on_press(Message::UpdaterAction(UpdaterAction::NeverUpdate)),
                    ].spacing(15)
                ]
                    .spacing(20)
                    .align_x(alignment::Horizontal::Center)
                    .into()
            }
            UpdateStatus::Downloading(tag) => {
                let display_tag = if tag.starts_with('v') { tag.clone() } else { format!("v{}", tag) };

                column![
                    text("Downloading Update").size(24),
                    text(format!("Downloading {}...", display_tag)),
                    progress_bar(0.0..=1.0, self.download_progress)
                ]
                    .spacing(20)
                    .align_x(alignment::Horizontal::Center)
                    .into()
            }
            UpdateStatus::RestartPending(tag) => {
                let display_tag = if tag.starts_with('v') { tag.clone() } else { format!("v{}", tag) };

                column![
                    text("Update Complete").size(24),
                    text(format!("{} update complete!", display_tag)),
                    text("Would you like to restart and apply the update now?"),
                    row![
                        button("Yes").on_press(Message::UpdaterAction(UpdaterAction::RestartApp)),
                        button("No").on_press(Message::UpdaterAction(UpdaterAction::DismissUpdate)),
                    ].spacing(15)
                ]
                    .spacing(20)
                    .align_x(alignment::Horizontal::Center)
                    .into()
            }
            _ => return None,
        };

        let modal_card = container(
            scrollable(modal_content)
                .width(Length::Fill)
                .height(Length::Shrink)
        )
            .padding(30)
            .width(Length::Fixed(400.0))
            .style(|theme: &Theme| {
                let palette = theme.palette();
                container::Style {
                    background: Some(palette.background.into()),
                    border: iced::border::rounded(10).color(palette.text).width(2),
                    ..Default::default()
                }
            });

        let overlay = container(modal_card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme| {
                container::Style {
                    background: Some(Color::from_rgba8(0, 0, 0, 0.7).into()),
                    ..Default::default()
                }
            });

        Some(overlay.into())
    }
}