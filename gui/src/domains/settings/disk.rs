use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::widget::{column, container, text, tooltip};
use iced::{task, Element, Task};
use tracing::{debug, error};

use core::domains::import::architecture;

use crate::app::theme;
use crate::common::feedback::{Slot as Confirm, CONFIRM_LABEL};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Game,
    Raw,
    Cache,
}

#[derive(Debug, Clone)]
pub enum Message {
    RequestDelete(Target),
    Refresh,
    SizesLoaded(Sizes),
    ConfirmExpired,
    DeleteFinished(Target),
    DoneExpired(Target),
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Sizes {
    game: u64,
    raw: u64,
    cache: u64,
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
    confirm: Confirm<Target>,
    sizes: Sizes,
}

fn size_hint<'a>(content: impl Into<Element<'a, Message>>, size: u64) -> Element<'a, Message> {
    tooltip(
        content,
        container(text(format_size(size))).padding(6).style(container::bordered_box),
        tooltip::Position::Right,
    )
    .into()
}

fn measure() -> Sizes {
    Sizes {
        game: folder_size(Path::new(architecture::GAME)),
        raw: folder_size(Path::new(architecture::RAW)),
        cache: core::common::dirs::cache_path().map_or(0, |dir| folder_size(&dir)),
    }
}

fn format_size(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let size = size as f64;

    if size >= GB {
        format!("{:.1} GB", size / GB)
    } else if size >= MB {
        format!("{:.1} MB", size / MB)
    } else if size >= KB {
        format!("{:.1} KB", size / KB)
    } else {
        format!("{:.1} B", size)
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


impl State {
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
                if !self.confirm.take(&target) {
                    return self.confirm.set(target, Message::ConfirmExpired);
                }

                let path = match target {
                    Target::Game => {
                        self.raw.reset();
                        PathBuf::from(architecture::GAME)
                    }
                    Target::Raw => PathBuf::from(architecture::RAW),
                    Target::Cache => {
                        let Some(cache_dir) = core::common::dirs::cache_path() else {
                            return Task::none();
                        };
                        cache_dir
                    }
                };

                self.start_delete(target, path)
            }
            Message::Refresh => Task::perform(smol::unblock(measure), Message::SizesLoaded),
            Message::SizesLoaded(sizes) => {
                self.sizes = sizes;
                Task::none()
            }
            Message::ConfirmExpired => {
                self.confirm.expire();
                Task::none()
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

                Task::batch([banner_task, Task::done(Message::Refresh)])
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

    fn disk_button<'a>(&'a self, name: &str, phase: Phase, target: Target, can_delete: bool, size: u64) -> Element<'a, Message> {
        match phase {
            Phase::Deleting => theme::sized_button(format!("Deleting \"{}\"...", name), theme::ACTION_BUTTON_WIDTH, theme::warning_button).into(),
            Phase::Done => theme::sized_button(format!("Deleted \"{}\"!", name), theme::ACTION_BUTTON_WIDTH, theme::success_button).into(),
            Phase::Idle if can_delete => {
                let label = if self.confirm.armed_for(&target) {
                    CONFIRM_LABEL.to_string()
                } else {
                    format!("Delete \"{}\"", name)
                };

                let button = theme::sized_button(label, theme::ACTION_BUTTON_WIDTH, theme::danger_button)
                    .on_press(Message::RequestDelete(target));

                size_hint(button, size)
            }
            Phase::Idle => theme::sized_button(format!("No \"{}\"", name), theme::ACTION_BUTTON_WIDTH, theme::neutral_button).into(),
        }
    }

    fn cache_button<'a>(&'a self, phase: Phase, can_delete: bool) -> Element<'a, Message> {
        match phase {
            Phase::Deleting => theme::sized_button("Clearing Cache...", theme::ACTION_BUTTON_WIDTH, theme::warning_button).into(),
            Phase::Done => theme::sized_button("Cache Cleared!", theme::ACTION_BUTTON_WIDTH, theme::success_button).into(),
            Phase::Idle if can_delete => {
                let label = self.confirm.confirm_label("Clear Cache");

                let button = theme::sized_button(label, theme::ACTION_BUTTON_WIDTH, theme::danger_button)
                    .on_press(Message::RequestDelete(Target::Cache));

                size_hint(button, self.sizes.cache)
            }
            Phase::Idle => theme::sized_button("Cache Empty", theme::ACTION_BUTTON_WIDTH, theme::neutral_button).into(),
        }
    }

    pub fn view<'a>(&'a self) -> Element<'a, Message> {
        let game_exists = architecture::game_present();
        let raw_exists = architecture::has_content(Path::new(architecture::RAW));
        let cache_present = core::common::dirs::cache_path()
            .is_some_and(|dir| architecture::has_content(&dir));

        let raw_can_delete = raw_exists && self.game.phase != Phase::Deleting;

        column![
            self.disk_button("game", self.game.phase, Target::Game, game_exists, self.sizes.game),
            self.disk_button("raw", self.raw.phase, Target::Raw, raw_can_delete, self.sizes.raw),
            self.cache_button(self.cache.phase, cache_present),
        ].spacing(8).into()
    }

}
