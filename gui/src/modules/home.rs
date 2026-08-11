use std::env;

use iced::widget::{button, column, container, markdown, pick_list, row, scrollable, text, Space};
use iced::{Alignment, Color, Element, Length, Size, Task, Theme};
use self_update::backends::github::ReleaseList;
use tracing::{debug, error, info, warn};

use crate::app::{theme, Page};
use crate::widget::{nightly_label, popup, smooth_scroll};

const SPACE_TOP: f32 = 20.0;
const SPACE_TITLE_SUBTITLE: f32 = 2.0;
const SPACE_SUBTITLE_SECTION: f32 = 50.0;
const SPACE_BETWEEN_SECTIONS: f32 = 20.0;
const BUTTON_WIDTH: f32 = 120.0;
const BUTTON_SPACING: f32 = 10.0;
const BUTTON_TEXT_SIZE: f32 = 15.0;
const CHANGELOG_POPUP_SIZE: Size = Size::new(600.0, 430.0);
const SCROLLBAR_GAP: f32 = 8.0;

#[derive(Default)]
pub struct State {
    is_game_empty: Option<bool>,
    changelog_open: bool,
    changelog_loading: bool,
    changelog_error: bool,
    releases: Vec<(String, String)>,
    selected_version: Option<String>,
    changelog_items: Vec<markdown::Item>,
    changelog_popup: popup::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    InitChecked(bool),
    OpenChangelog,
    Popup(popup::Message),
    OpenUrl(String),
    ChangelogsFetched(Result<Vec<(String, String)>, String>),
    SelectChangelogVersion(String),
    Navigate(Page),
    NavigateSettingsKeys,
    NavigateSettingsAddOns,
}

impl State {
    pub fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    pub fn set_game_empty(&mut self, is_empty: bool) {
        self.is_game_empty = Some(is_empty);
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
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

            Message::Popup(msg) => {
                if self.changelog_popup.update(msg, CHANGELOG_POPUP_SIZE) {
                    self.changelog_open = false;
                }
                Task::none()
            }

            Message::OpenUrl(url) => {
                if let Err(e) = open::that(&url) {
                    warn!("Failed to open URL {}: {}", url, e);
                }
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

                self.refresh_changelog_items();
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
                self.refresh_changelog_items();
                Task::none()
            }

            Message::Navigate(_) | Message::NavigateSettingsKeys | Message::NavigateSettingsAddOns => {
                debug!("Navigation requested, deferring to root app");
                Task::none()
            }
        }
    }

    pub fn view(&self, nightly: bool) -> Element<'_, Message> {
        let is_empty = self.is_game_empty.unwrap_or(false);

        let main_content = column![
            Space::new().height(SPACE_TOP),
            text("Battle Cats Complete")
                .size(40.0)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.palette().text),
                }),
            Space::new().height(SPACE_TITLE_SUBTITLE),
            text("All-In-One Battle Cats Toolkit")
                .size(16.0)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(Color { a: 0.4, ..theme.palette().text }),
                }),
            Space::new().height(SPACE_SUBTITLE_SECTION),
            if is_empty {
                self.view_setup_guide()
            } else {
                self.view_navigation(nightly)
            }
        ]
            .align_x(Alignment::Center)
            .width(Length::Fill);

        let version_tag = format!("v{}", env!("CARGO_PKG_VERSION"));
        let release_url = format!("https://github.com/omochikaeri15/battle-cats-complete/releases/tag/{}", version_tag);
        let footer = row![
            button(text(version_tag).size(13.0))
                .style(button::text)
                .padding(0.0)
                .on_press(Message::OpenUrl(release_url)),
            text(" | ").size(13.0),
            button(text("Changelogs").size(13.0))
                .style(button::text)
                .padding(0.0)
                .on_press(Message::OpenChangelog),
            Space::new().width(Length::Fill),
            button(text("Discord").size(13.0))
                .style(button::text)
                .padding(0.0)
                .on_press(Message::OpenUrl("https://discord.com/invite/SNSE8HNhmP".to_string())),
            text(" | ").size(13.0),
            button(text("GitHub").size(13.0))
                .style(button::text)
                .padding(0.0)
                .on_press(Message::OpenUrl("https://github.com/omochikaeri15/battle-cats-complete".to_string())),
        ]
            .width(Length::Fill)
            .padding(10.0)
            .align_y(Alignment::Center);

        column![
            container(main_content).height(Length::Fill),
            footer
        ].into()
    }

    pub fn changelog_popup_open(&self) -> bool {
        self.changelog_open
    }

    pub fn changelog_popup_view(&self, window: Size, theme: Theme) -> Option<Element<'_, Message>> {
        self.changelog_open.then(|| self.view_changelog_modal(window, theme))
    }

    fn refresh_changelog_items(&mut self) {
        let body_text = self.selected_version
            .as_ref()
            .and_then(|sel| self.releases.iter().find(|(v, _)| v == sel))
            .map(|(_, body)| body.as_str())
            .unwrap_or("No notes available.");

        self.changelog_items = markdown::parse(body_text).collect();
    }

    fn view_setup_guide(&self) -> Element<'_, Message> {
        column![
            Space::new().height(10.0),
            text("To get started, you will need to populate the \"game\" folder with game files using the \"Data\" page.\nAlternatively, you can put decrypted files in the \"game\" folder using any folder structure you desire.").size(15.0).align_x(Alignment::Center),
            Space::new().height(8.0),
            button(text("Import").size(15.0))
                .style(button::primary)
                .on_press(Message::Navigate(Page::Import)),

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

    fn view_navigation(&self, nightly: bool) -> Element<'_, Message> {
        let nav_row = |buttons: &[(&'static str, Page)]| -> Element<Message> {
            let mut row = row![].spacing(BUTTON_SPACING).align_y(Alignment::Center);
            for (label, page) in buttons {
                if page.nightly() && !nightly {
                    continue;
                }

                let content: Element<Message> = if page.nightly() {
                    nightly_label(label, BUTTON_TEXT_SIZE)
                } else {
                    text(*label).size(BUTTON_TEXT_SIZE).align_x(Alignment::Center).into()
                };

                row = row.push(
                    button(content)
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
            nav_row(&[("Mods", Page::Mods), ("Files", Page::Files), ("Import", Page::Import)]),
            Space::new().height(SPACE_BETWEEN_SECTIONS),

            text("Other").size(18.0),
            Space::new().height(10.0),
            nav_row(&[("Settings", Page::Settings)]),
        ]
            .align_x(Alignment::Center)
            .into()
    }

    fn view_changelog_modal(&self, window: Size, theme: Theme) -> Element<'_, Message> {
        self.changelog_popup.view("Changelogs", CHANGELOG_POPUP_SIZE, window, Message::Popup, move || self.changelog_content(&theme), None)
    }

    fn changelog_content(&self, theme: &Theme) -> Element<'_, Message> {
        if self.changelog_error {
            return container(text("Couldn't connect to GitHub").size(18.0))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        if self.changelog_loading {
            return container(text("Loading..."))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        let options: Vec<String> = self.releases.iter().map(|(v, _)| v.clone()).collect();

        column![
            row![
                text("Version:").size(16.0),
                Space::new().width(10.0),
                pick_list(
                    options,
                    self.selected_version.clone(),
                    Message::SelectChangelogVersion
                )
                    .style(theme::combo_box)
                    .menu_style(theme::combo_box_menu)
            ]
                .align_y(Alignment::Center),
            Space::new().height(15.0),
            smooth_scroll(
                scrollable(
                    markdown::view(&self.changelog_items, markdown::Settings::with_text_size(14.0, theme))
                        .map(Message::OpenUrl)
                )
                    .height(Length::Fill)
                    .spacing(SCROLLBAR_GAP)
            ),
        ]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20.0)
            .into()
    }
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