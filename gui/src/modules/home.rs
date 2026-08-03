use std::env;
use std::fs;
use std::path::Path;

use iced::widget::{button, column, container, pick_list, row, scrollable, stack, text, Space};
use iced::{Alignment, Background, Color, Element, Length, Task, Theme};
use self_update::backends::github::ReleaseList;
use tracing::{debug, error, info};

use crate::app::Page;

const SPACE_TOP: f32 = 20.0;
const SPACE_TITLE_SUBTITLE: f32 = 2.0;
const SPACE_SUBTITLE_SECTION: f32 = 50.0;
const SPACE_BETWEEN_SECTIONS: f32 = 20.0;
const BUTTON_WIDTH: f32 = 120.0;
const BUTTON_SPACING: f32 = 10.0;

#[derive(Default)]
pub struct State {
    is_game_empty: Option<bool>,
    changelog_open: bool,
    changelog_loading: bool,
    changelog_error: bool,
    releases: Vec<(String, String)>,
    selected_version: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CheckInit,
    InitChecked(bool),
    OpenChangelog,
    CloseChangelog,
    ChangelogsFetched(Result<Vec<(String, String)>, String>),
    SelectChangelogVersion(String),
    Navigate(Page),
    NavigateSettingsKeys,
    NavigateSettingsAddOns,
}

impl State {
    pub fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::perform(check_initialization(), Message::InitChecked))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CheckInit => Task::perform(check_initialization(), Message::InitChecked),

            Message::InitChecked(is_empty) => {
                self.is_game_empty = Some(is_empty);
                Task::none()
            }

            Message::OpenChangelog => {
                self.changelog_open = true;
                if self.releases.is_empty() && !self.changelog_loading {
                    self.changelog_loading = true;
                    self.changelog_error = false;
                    return Task::perform(smol::unblock(fetch_changelogs), Message::ChangelogsFetched);
                }
                Task::none()
            }

            Message::CloseChangelog => {
                self.changelog_open = false;
                Task::none()
            }

            Message::ChangelogsFetched(Ok(releases)) => {
                self.changelog_loading = false;
                self.changelog_error = false;
                self.releases = releases;

                let current_version = env!("CARGO_PKG_VERSION").trim_start_matches('v');
                if self.releases.iter().any(|(v, _)| v == current_version) {
                    self.selected_version = Some(current_version.to_string());
                } else if let Some(first) = self.releases.first() {
                    self.selected_version = Some(first.0.clone());
                } else {
                    self.releases.push(("Unknown".to_string(), "No releases found.".to_string()));
                    self.selected_version = Some("Unknown".to_string());
                }

                Task::none()
            }

            Message::ChangelogsFetched(Err(e)) => {
                error!("Failed to fetch changelogs: {}", e);
                self.changelog_loading = false;
                self.changelog_error = true;
                Task::none()
            }

            Message::SelectChangelogVersion(version) => {
                self.selected_version = Some(version);
                Task::none()
            }

            Message::Navigate(_) | Message::NavigateSettingsKeys | Message::NavigateSettingsAddOns => {
                debug!("Navigation requested, deferring to root app");
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let is_empty = self.is_game_empty.unwrap_or(true);

        let main_content = column![
            Space::new().height(SPACE_TOP),
            text("Battle Cats Complete")
                .size(40.0)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().background.strong.color),
                }),
            Space::new().height(SPACE_TITLE_SUBTITLE),
            text("All-In-One Battle Cats Toolkit").size(16.0),
            Space::new().height(SPACE_SUBTITLE_SECTION),
            if is_empty {
                self.view_setup_guide()
            } else {
                self.view_navigation()
            }
        ]
            .align_x(Alignment::Center)
            .width(Length::Fill);

        let version_tag = format!("v{}", env!("CARGO_PKG_VERSION"));
        let footer = row![
            text(version_tag).size(13.0),
            text(" | ").size(13.0),
            button(text("Changelogs").size(13.0))
                .style(button::text)
                .on_press(Message::OpenChangelog),
            Space::new().width(Length::Fill),
            text("Discord").size(13.0),
            text(" | ").size(13.0),
            text("GitHub").size(13.0),
        ]
            .width(Length::Fill)
            .padding(10.0)
            .align_y(Alignment::Center);

        let layout = column![
            container(main_content).height(Length::Fill),
            footer
        ];

        if self.changelog_open {
            stack![layout, self.view_changelog_modal()].into()
        } else {
            layout.into()
        }
    }

    fn view_setup_guide(&self) -> Element<'_, Message> {
        column![
            Space::new().height(10.0),
            text("To get started, you will need to populate the \"game\" folder with game files using the \"Data\" page.").size(15.0),
            Space::new().height(8.0),
            button(text("Data").size(15.0))
                .style(button::primary)
                .on_press(Message::Navigate(Page::Data)),

            Space::new().height(35.0),

            text("To import encrypted \"pack\" files, you will need to provide Decryption Keys and Initialization Vectors\nusing the \"Manage Keys\" button under the \"Data\" tab in the \"Settings\" page.").size(15.0).align_x(Alignment::Center),
            Space::new().height(8.0),
            button(text("Settings > Data > Manage Keys").size(15.0))
                .style(button::primary)
                .on_press(Message::NavigateSettingsKeys),

            Space::new().height(35.0),

            text("Importing from an Android device or Emulator requires Keys & IV, the Android bridge Add-on, and\nroot access. You can find the \"Android Bridge\" section under the \"Add-Ons\" tab in the \"Settings\" page.").size(15.0).align_x(Alignment::Center),
            Space::new().height(8.0),
            button(text("Settings > Add-Ons").size(15.0))
                .style(button::primary)
                .on_press(Message::NavigateSettingsAddOns),
        ]
            .align_x(Alignment::Center)
            .into()
    }

    fn view_navigation(&self) -> Element<'_, Message> {
        let nav_row = |buttons: &[(&'static str, Page)]| -> Element<Message> {
            let mut row = row![].spacing(BUTTON_SPACING).align_y(Alignment::Center);
            for (label, page) in buttons {
                row = row.push(
                    button(text(*label).size(15.0).align_x(Alignment::Center))
                        .width(BUTTON_WIDTH)
                        .style(button::primary)
                        .on_press(Message::Navigate(*page))
                );
            }
            row.into()
        };

        column![
            text("Information").size(18.0),
            Space::new().height(10.0),
            nav_row(&[("Cats", Page::Cats), ("Enemies", Page::Enemies), ("Stages", Page::Stages)]),
            Space::new().height(SPACE_BETWEEN_SECTIONS),

            text("Database").size(18.0),
            Space::new().height(10.0),
            nav_row(&[("Mods", Page::Mods), ("Data", Page::Data)]),
            Space::new().height(SPACE_BETWEEN_SECTIONS),

            text("Other").size(18.0),
            Space::new().height(10.0),
            nav_row(&[("Settings", Page::Settings)]),
        ]
            .align_x(Alignment::Center)
            .into()
    }

    fn view_changelog_modal(&self) -> Element<'_, Message> {
        let mut content = column![
            row![
                text("Changelogs").size(24.0),
                Space::new().width(Length::Fill),
                button("X").on_press(Message::CloseChangelog).style(button::danger)
            ]
            .align_y(Alignment::Center),
            Space::new().height(15.0),
        ];

        if self.changelog_error {
            content = content.push(
                container(text("Couldn't connect to GitHub").size(18.0))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
            );
        } else if self.changelog_loading {
            content = content.push(
                container(text("Loading..."))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
            );
        } else {
            let options: Vec<String> = self.releases.iter().map(|(v, _)| v.clone()).collect();

            content = content.push(
                row![
                    text("Version:").size(16.0),
                    Space::new().width(10.0),
                    pick_list(
                        options,
                        self.selected_version.clone(),
                        Message::SelectChangelogVersion
                    )
                ]
                    .align_y(Alignment::Center)
            );
            content = content.push(Space::new().height(15.0));

            let body_text = self.selected_version
                .as_ref()
                .and_then(|sel| self.releases.iter().find(|(v, _)| v == sel))
                .map(|(_, body)| body.as_str())
                .unwrap_or("No notes available.");

            content = content.push(
                scrollable(text(body_text).size(14.0)).height(Length::Fill)
            );
        }

        container(
            container(content)
                .width(600.0)
                .height(400.0)
                .style(container::bordered_box)
                .padding(20.0)
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.8,
                        ..palette.background.base.color
                    })),
                    ..Default::default()
                }
            })
            .into()
    }
}

async fn check_initialization() -> bool {
    let game_dir = Path::new("game");
    if !game_dir.exists() {
        return true;
    }

    fs::read_dir(game_dir).map_or(true, |mut iter| iter.next().is_none())
}

fn fetch_changelogs() -> Result<Vec<(String, String)>, String> {
    info!("Fetching GitHub releases...");

    let result = ReleaseList::configure()
        .repo_owner("omochikaeri15")
        .repo_name("battle-cats-complete")
        .build()
        .map_err(|e| e.to_string())?
        .fetch()
        .map_err(|e| e.to_string())?;

    let mut formatted = Vec::new();
    for r in result {
        let clean_version = r.version.trim_start_matches('v').to_string();
        if !clean_version.is_empty() && clean_version.chars().all(|c| c.is_ascii_digit() || c == '.') {
            let raw_body = r.body.unwrap_or_else(|| "No notes.".to_string());
            formatted.push((clean_version, raw_body));
        }
    }

    Ok(formatted)
}