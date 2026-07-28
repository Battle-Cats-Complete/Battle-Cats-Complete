use std::fs;
use std::path::Path;

use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Border, Element, Length, Theme};

use core::modules::settings::delete::FolderDeleter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Game,
    Raw,
    Cache,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    RequestDelete(Target),
    ConfirmDelete,
    CancelDelete,
}

pub struct State {
    game: FolderDeleter,
    raw: FolderDeleter,
    cache: FolderDeleter,
    pending: Option<Target>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            game: FolderDeleter::default(),
            raw: FolderDeleter::default(),
            cache: FolderDeleter::default(),
            pending: None,
        }
    }
}

fn folder_size(path: &Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    size += folder_size(&entry.path());
                } else {
                    size += metadata.len();
                }
            }
        }
    }
    size
}

fn format_size(size: u64) -> String {
    let kb = 1024.0;
    let mb = kb * 1024.0;
    let gb = mb * 1024.0;
    let size = size as f64;

    if size >= gb {
        format!("{:.2} GB", size / gb)
    } else if size >= mb {
        format!("{:.2} MB", size / mb)
    } else if size >= kb {
        format!("{:.2} KB", size / kb)
    } else {
        format!("{} B", size)
    }
}

impl State {
    pub fn is_modal_open(&self) -> bool {
        self.pending.is_some()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Tick => {
                self.game.update();
                self.raw.update();
                self.cache.update();
            }
            Message::RequestDelete(target) => self.pending = Some(target),
            Message::CancelDelete => self.pending = None,
            Message::ConfirmDelete => {
                if let Some(target) = self.pending.take() {
                    match target {
                        Target::Game => {
                            if self.raw.is_active() {
                                self.raw = FolderDeleter::default();
                            }
                            self.game.start("game");
                        }
                        Target::Raw => self.raw.start("game/raw"),
                        Target::Cache => {
                            if let Some(cache_dir) = core::common::io::cache::get_cache_dir() {
                                self.cache.start(cache_dir);
                            }
                        }
                    }
                }
            }
        }
    }

    fn disk_button<'a>(&'a self, label_idle: String, deleter: &FolderDeleter, target: Target, can_delete: bool) -> Element<'a, Message> {
        if deleter.is_deleting() {
            button(text(format!("Deleting \"{}\"...", label_idle)).size(14))
                .padding([8, 16])
                .style(|_theme: &Theme, _status| button::Style {
                    background: Some(iced::Color::from_rgb8(200, 180, 50).into()),
                    text_color: iced::Color::WHITE,
                    ..Default::default()
                })
                .into()
        } else if deleter.is_done() {
            button(text(format!("Deleted \"{}\"!", label_idle)).size(14))
                .padding([8, 16])
                .style(button::success)
                .into()
        } else if can_delete {
            button(text(format!("Delete \"{}\"", label_idle)).size(14))
                .padding([8, 16])
                .style(button::danger)
                .on_press(Message::RequestDelete(target))
                .into()
        } else {
            button(text(format!("No \"{}\"", label_idle)).size(14))
                .padding([8, 16])
                .style(button::secondary)
                .into()
        }
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let game_exists = Path::new("game").exists();
        let raw_exists = Path::new("game/raw").exists();
        let cache_size = core::common::io::cache::get_cache_dir()
            .map(|dir| folder_size(&dir))
            .unwrap_or(0);

        let raw_can_delete = raw_exists && !self.game.is_deleting();

        column![
            self.disk_button("game".to_string(), &self.game, Target::Game, game_exists),
            self.disk_button("raw".to_string(), &self.raw, Target::Raw, raw_can_delete),
            self.disk_button(
                if cache_size > 0 { "Clear Cache".to_string() } else { "Cache Empty".to_string() },
                &self.cache, Target::Cache, cache_size > 0
            ),
        ].spacing(8).into()
    }

    pub fn view_modal<'a>(&'a self) -> Element<'a, Message> {
        let (message, size_str) = match self.pending {
            Some(Target::Game) => ("Are you sure you want to delete the \"game\" folder?\nMost app function will be lost.".to_string(), None),
            Some(Target::Raw) => (
                "Are you sure you want to delete the \"raw\" folder?\nYou may need to import again.".to_string(),
                Some(format_size(folder_size(Path::new("game/raw")))),
            ),
            Some(Target::Cache) => (
                "Are you sure you want to clear the Cache?\nIt will automatically rebuild the next time the app loads.".to_string(),
                core::common::io::cache::get_cache_dir().map(|dir| format_size(folder_size(&dir))),
            ),
            None => (String::new(), None),
        };

        let mut content = column![text(message)].spacing(10).align_x(Alignment::Center);

        if let Some(size) = size_str {
            content = content.push(text(format!("Folder size: {}", size)).size(13));
        }

        content = content.push(
            row![
                button("Yes").on_press(Message::ConfirmDelete).style(button::danger),
                button("No").on_press(Message::CancelDelete),
            ].spacing(10)
        );

        container(content.padding(25).align_x(Alignment::Center))
            .style(|theme: &Theme| {
                container::background(theme.palette().background)
                    .border(Border { color: theme.palette().text, width: 1.0, radius: 8.0.into() })
            })
            .width(Length::Shrink)
            .into()
    }
}
