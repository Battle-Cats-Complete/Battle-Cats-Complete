use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::widget::{button, column, container, row, text};
use iced::{task, Alignment, Element, Length, Task, Theme};
use tracing::{debug, error};

use crate::app::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Game,
    Raw,
    Cache,
}

#[derive(Debug, Clone)]
pub enum Message {
    RequestDelete(Target),
    ConfirmDelete,
    CancelDelete,
    DeleteFinished(Target),
    DoneExpired(Target),
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    Deleting,
    Done,
}

#[derive(Default)]
struct Slot {
    phase: Phase,
    delete_handle: Option<task::Handle>,
    banner_handle: Option<task::Handle>,
}

impl Slot {
    fn reset(&mut self) {
        if let Some(handle) = self.delete_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.banner_handle.take() {
            handle.abort();
        }
        self.phase = Phase::Idle;
    }
}

#[derive(Default)]
pub struct State {
    game: Slot,
    raw: Slot,
    cache: Slot,
    pending: Option<Target>,
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

    fn slot_mut(&mut self, target: Target) -> &mut Slot {
        match target {
            Target::Game => &mut self.game,
            Target::Raw => &mut self.raw,
            Target::Cache => &mut self.cache,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RequestDelete(target) => {
                self.pending = Some(target);
                Task::none()
            }
            Message::CancelDelete => {
                self.pending = None;
                Task::none()
            }
            Message::ConfirmDelete => {
                let Some(target) = self.pending.take() else {
                    return Task::none();
                };

                let path = match target {
                    Target::Game => {
                        self.raw.reset();
                        PathBuf::from("game")
                    }
                    Target::Raw => PathBuf::from("game/raw"),
                    Target::Cache => {
                        let Some(cache_dir) = core::common::io::cache::get_cache_dir() else {
                            return Task::none();
                        };
                        cache_dir
                    }
                };

                self.start_delete(target, path)
            }
            Message::DeleteFinished(target) => {
                let slot = self.slot_mut(target);
                slot.phase = Phase::Done;
                slot.delete_handle = None;

                let (banner_task, handle) = Task::perform(
                    async { smol::Timer::after(Duration::from_secs(2)).await },
                    move |_| Message::DoneExpired(target),
                )
                .abortable();
                slot.banner_handle = Some(handle);
                banner_task
            }
            Message::DoneExpired(target) => {
                let slot = self.slot_mut(target);
                slot.phase = Phase::Idle;
                slot.banner_handle = None;
                Task::none()
            }
        }
    }

    fn start_delete(&mut self, target: Target, path: PathBuf) -> Task<Message> {
        debug!("Initializing folder deletion for path: {:?}", path);

        let (delete_task, handle) = Task::perform(
            smol::unblock(move || {
                if let Err(delete_error) = fs::remove_dir_all(&path) {
                    error!("Failed to delete folder {:?}: {}", path, delete_error);
                } else {
                    debug!("Folder deletion completed successfully.");
                }
            }),
            move |_| Message::DeleteFinished(target),
        )
        .abortable();

        let slot = self.slot_mut(target);
        slot.phase = Phase::Deleting;
        slot.delete_handle = Some(handle);
        delete_task
    }

    fn disk_button<'a>(&'a self, label_idle: String, phase: Phase, target: Target, can_delete: bool) -> Element<'a, Message> {
        match phase {
            Phase::Deleting => button(text(format!("Deleting \"{}\"...", label_idle)).size(14))
                .padding([8, 16])
                .style(|_theme: &Theme, _status| button::Style {
                    background: Some(iced::Color::from_rgb8(200, 180, 50).into()),
                    text_color: iced::Color::WHITE,
                    ..Default::default()
                })
                .into(),
            Phase::Done => button(text(format!("Deleted \"{}\"!", label_idle)).size(14))
                .padding([8, 16])
                .style(button::success)
                .into(),
            Phase::Idle if can_delete => button(text(format!("Delete \"{}\"", label_idle)).size(14))
                .padding([8, 16])
                .style(button::danger)
                .on_press(Message::RequestDelete(target))
                .into(),
            Phase::Idle => button(text(format!("No \"{}\"", label_idle)).size(14))
                .padding([8, 16])
                .style(button::secondary)
                .into(),
        }
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let game_exists = Path::new("game").exists();
        let raw_exists = Path::new("game/raw").exists();
        let cache_size = core::common::io::cache::get_cache_dir()
            .map(|dir| folder_size(&dir))
            .unwrap_or(0);

        let raw_can_delete = raw_exists && self.game.phase != Phase::Deleting;

        column![
            self.disk_button("game".to_string(), self.game.phase, Target::Game, game_exists),
            self.disk_button("raw".to_string(), self.raw.phase, Target::Raw, raw_can_delete),
            self.disk_button(
                if cache_size > 0 { "Clear Cache".to_string() } else { "Cache Empty".to_string() },
                self.cache.phase, Target::Cache, cache_size > 0
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
            .style(theme::confirm_modal_container)
            .width(Length::Shrink)
            .into()
    }
}
