mod body;
mod picture;
mod tree;

use std::ffi::OsStr;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, mouse_area, pick_list, row, scrollable, space, stack, text_input};
use iced::{font, Alignment, Element, Font, Length, Padding, Task, Theme};
use serde::{Deserialize, Serialize};
use tracing::info;

use kore::domains::import::architecture;
use kore::domains::settings::{EditorMode, FilesSettings};
use kore::Vfs;

use crate::app::state::FilesState;
use crate::app::theme;
use crate::common::feedback::LOCKED_NOTICE;
use crate::common::feedback::Slot;
use crate::common::fonts;
use crate::common::watcher;
use crate::widget::{slide, Slide};

const PANEL_WIDTH: f32 = 320.0;
const PANEL_PADDING: f32 = 8.0;

const TOGGLE_BUTTON_SIZE: f32 = 30.0;
const TOGGLE_BUTTON_GAP: f32 = 5.0;
const TOGGLE_ARROW_SIZE: f32 = 16.0;

const ARROW_OPEN: &str = "\u{25c0}";
const ARROW_SHUT: &str = "\u{25b6}";

const HEADER_HEIGHT: f32 = 38.0;
const HEADER_PADDING: f32 = 10.0;
const HEADER_TEXT_SIZE: f32 = 13.0;
const HEADER_TOP_GAP: f32 = 3.0;
const HEADER_BODY_GAP: f32 = 3.0;
const HEADER_EMPTY: &str = "Please select a file";

const PICKER_GAP: f32 = 4.0;
const PICKER_TREE_GAP: f32 = 8.0;
const PICKER_TEXT_SIZE: f32 = 13.0;
const PICKER_PADDING: [u16; 2] = [4, 8];
const MODE_WIDTH: f32 = 60.0;

const BODY_PADDING: f32 = 8.0;

const TEXT_SIZE: f32 = 13.0;
const EMPTY_TEXT_SIZE: f32 = 14.0;

const SCROLLBAR_WIDTH: f32 = 6.0;
const SCROLLBAR_MARGIN: f32 = 2.0;
const SCROLLBAR_ALLOWANCE: f32 = 14.0;

const NOTICE_PADDING_X: f32 = 7.0;
const NOTICE_PADDING_Y: f32 = 7.0;
const NOTICE_OVERHANG: f32 = 4.0;
const NOTICE_TEXT_SIZE: f32 = 13.0;
const NOTICE_EXPIRY: Duration = Duration::from_secs(3);

const EMPTY_LABEL: &str = "No Files Found on Mount";
const MISSING_LABEL: &str = "Selected Mount Missing from Memory";
const NO_MOUNTS_LABEL: &str = "No Mounts Available";

fn both_ways() -> scrollable::Direction {
    let bar = || scrollable::Scrollbar::new().width(SCROLLBAR_WIDTH).margin(SCROLLBAR_MARGIN);

    scrollable::Direction::Both { vertical: bar(), horizontal: bar() }
}

#[derive(Debug, Clone)]
pub enum Message {
    MountSelected(String),
    ModeSelected(Mode),
    SearchChanged(String),
    ToggleSidebar,
    Deselect,
    NoticeExpired,
    Tree(tree::Message),
    Body(body::Message),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Tree,
    Flat,
}

impl Mode {
    const ALL: [Self; 2] = [Self::Tree, Self::Flat];
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Tree => "Tree",
            Self::Flat => "Flat",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Content {
    Rows,
    NoMounts,
    MountMissing,
    NoFiles,
}

impl Content {
    fn label(self) -> Option<&'static str> {
        match self {
            Self::Rows => None,
            Self::NoMounts => Some(NO_MOUNTS_LABEL),
            Self::MountMissing => Some(MISSING_LABEL),
            Self::NoFiles => Some(EMPTY_LABEL),
        }
    }
}

pub struct State {
    mounts: Vec<String>,
    mount: Option<String>,
    mode: Mode,
    search_query: String,
    selected: Option<PathBuf>,
    content: Content,
    sidebar_open: bool,
    entered: bool,
    verify: bool,
    unlocked: bool,
    editor_mode: EditorMode,
    tree: tree::State,
    body: body::State,
    notice: Slot<()>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            mounts: Vec::new(),
            mount: None,
            mode: Mode::default(),
            search_query: String::new(),
            selected: None,
            content: Content::NoMounts,
            sidebar_open: true,
            entered: true,
            verify: false,
            unlocked: false,
            editor_mode: EditorMode::default(),
            tree: tree::State::default(),
            body: body::State::default(),
            notice: Slot::default(),
        }
    }
}

impl State {
    pub(crate) fn restore_state(&mut self, state: &FilesState) {
        self.mount = state.mount.clone();
        self.mode = state.mode;
        self.search_query = state.search_query.clone();
        self.selected = state.selected_file.clone();
        self.verify = true;
    }

    pub(crate) fn sync_state(&self, state: &mut FilesState) {
        let mount = if self.stale() { None } else { self.mount.clone() };

        if state.mount != mount {
            state.mount = mount;
        }

        if state.mode != self.mode {
            state.mode = self.mode;
        }

        if state.search_query != self.search_query {
            state.search_query = self.search_query.clone();
        }

        if state.selected_file != self.selected {
            state.selected_file = self.selected.clone();
        }
    }

    pub(crate) fn mount(&self) -> Option<&str> {
        self.mount.as_deref()
    }

    pub(crate) fn entry_at(&self, vfs: &Vfs, index: usize) -> Option<(bool, PathBuf)> {
        self.tree.entry(vfs, self.mount.as_deref()?, self.mode, index)
    }

    pub(crate) fn sync(&mut self, vfs: &Vfs, files: &FilesSettings, validated: bool) -> Task<Message> {
        self.adopt(files);
        self.entered = true;
        self.body.invalidate();

        let keys: Vec<String> = vfs.mount_keys().iter().map(|key| key.to_string()).collect();

        if keys != self.mounts {
            self.mounts = keys;
        }

        if self.verify && validated && !self.mounts.is_empty() {
            self.verify = false;

            if let Some(missing) = self.stale().then(|| self.mount.take()).flatten() {
                info!(mount = %missing, "Persisted mount is no longer indexed, falling back to the first mount");
                self.selected = None;
            } else if let Some(path) = self.selected.as_deref() {
                self.tree.reveal(path);
            }
        }

        if self.mount.is_none() {
            self.mount = self.mounts.first().cloned();
            self.reset();
        }

        self.reindex(vfs)
    }

    pub(crate) fn apply_changes(&mut self, vfs: &Vfs, paths: &[PathBuf], files: &FilesSettings) -> Task<Message> {
        self.adopt(files);
        let Some(mount) = self.mount.as_deref() else {
            return Task::none();
        };

        let mut mounted = false;
        let mut touched = false;

        for path in paths {
            if watcher::mount_of(path).is_none_or(|key| key != mount) {
                continue;
            }

            mounted = true;

            if vfs.relative(mount, path).is_some_and(|relative| Some(&relative) == self.selected.as_ref()) {
                touched = true;
            }
        }

        if touched {
            self.body.invalidate();
        }

        if !mounted {
            return Task::none();
        }

        self.reindex(vfs)
    }

    pub(crate) fn update(&mut self, message: Message, vfs: &Vfs, files: &FilesSettings) -> Task<Message> {
        self.adopt(files);
        match message {
            Message::MountSelected(mount) => {
                if self.mount.as_deref() == Some(mount.as_str()) {
                    return Task::none();
                }

                self.mount = Some(mount);
                self.reset();

                Task::batch([self.reindex(vfs), self.tree.snap_to_top().map(Message::Tree)])
            }
            Message::ModeSelected(mode) => {
                if self.mode == mode {
                    return Task::none();
                }

                self.mode = mode;
                self.tree.rewind();

                Task::batch([self.reindex(vfs), self.tree.snap_to_top().map(Message::Tree)])
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                self.tree.rewind();

                Task::batch([self.refresh(vfs), self.tree.snap_to_top().map(Message::Tree)])
            }
            Message::ToggleSidebar => {
                self.sidebar_open = !self.sidebar_open;
                self.entered = false;
                Task::none()
            }
            Message::Deselect => {
                if self.selected.is_none() {
                    return Task::none();
                }

                self.selected = None;

                self.refresh(vfs)
            }
            Message::NoticeExpired => {
                self.notice.expire();
                Task::none()
            }
            Message::Tree(msg) => {
                let Some(index) = self.tree.update(msg) else {
                    return Task::none();
                };

                self.activate(vfs, index)
            }
            Message::Body(msg) => {
                let (outcome, task) = self.body.update(msg);

                match outcome {
                    body::Outcome::Idle => task.map(Message::Body),
                    body::Outcome::Refused => self.notice.set_after((), Message::NoticeExpired, NOTICE_EXPIRY),
                    body::Outcome::Upload => {
                        self.upload(vfs);
                        Task::none()
                    }
                }
            }
        }
    }

    fn activate(&mut self, vfs: &Vfs, index: usize) -> Task<Message> {
        let Some(mount) = self.mount.as_deref() else {
            return Task::none();
        };

        let Some((folder, path)) = self.tree.entry(vfs, mount, self.mode, index) else {
            return Task::none();
        };

        if folder {
            self.tree.toggle(path);

            return self.refresh(vfs);
        }

        let repeat = self.selected.as_deref() == Some(path.as_path());

        self.selected = Some(path);
        let queued = self.refresh(vfs);

        if !repeat {
            return queued;
        }

        Task::batch([queued, self.body.recenter().map(Message::Body)])
    }

    fn reset(&mut self) {
        self.tree.reset();
        self.selected = None;
    }

    fn reindex(&mut self, vfs: &Vfs) -> Task<Message> {
        self.tree.refresh_keys(vfs, self.mount.as_deref(), self.mode);
        self.refresh(vfs)
    }

    fn refresh(&mut self, vfs: &Vfs) -> Task<Message> {
        self.content = self.rebuild(vfs);

        self.body.refresh(vfs, self.mount.as_deref(), self.selected.as_deref(), self.writable(), self.editor_mode);

        Task::none()
    }

    fn rebuild(&mut self, vfs: &Vfs) -> Content {
        if self.mounts.is_empty() {
            self.tree.clear();
            return Content::NoMounts;
        }

        if self.stale() {
            self.tree.clear();
            self.selected = None;
            return Content::MountMissing;
        }

        let Some(mount) = self.mount.as_deref() else {
            self.tree.clear();
            return Content::NoFiles;
        };

        let dropped = self.selected.as_deref().is_some_and(|path| {
            let Some(parent) = path.parent() else {
                return true;
            };

            let Some(name) = path.file_name().and_then(OsStr::to_str) else {
                return true;
            };

            !vfs.contains(mount, parent, name)
        });

        if dropped {
            self.selected = None;
        }

        if vfs.count(mount) == 0 {
            self.tree.clear();
            return Content::NoFiles;
        }

        let populated = self.tree.rebuild(vfs, mount, self.mode, &self.search_query, self.selected.as_deref());

        if populated { Content::Rows } else { Content::NoFiles }
    }

    fn upload(&mut self, vfs: &Vfs) {
        let Some(source) = rfd::FileDialog::new().add_filter("PNG Image", &["png"]).pick_file() else {
            return;
        };

        if !self.body.replace(vfs, self.mount.as_deref(), self.selected.as_deref(), &source) {
            return;
        }

        self.body.invalidate();
        drop(self.refresh(vfs));
    }

    pub(crate) fn leave(&mut self, vfs: &Vfs) {
        self.body.commit(vfs, self.mount.as_deref(), self.selected.as_deref());
    }

    fn adopt(&mut self, files: &FilesSettings) {
        self.unlocked = files.unlock_game_mount;
        self.editor_mode = files.editor_mode;
    }

    fn writable(&self) -> bool {
        self.mount.as_deref().is_some_and(|mount| self.unlocked || mount != architecture::GAME)
    }

    fn stale(&self) -> bool {
        !self.mounts.is_empty() && self.mount.as_ref().is_some_and(|mount| !self.mounts.contains(mount))
    }

    fn selection_label(&self) -> Option<String> {
        let mount = self.mount.as_deref()?;
        let name = self.selected.as_deref()?.file_name()?.to_str()?;

        Some(format!("{}{}{}", mount, theme::HEADER_SEPARATOR, name))
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let sidebar = column![
            self.view_controls(),
            space().height(Length::Fixed(PICKER_GAP)),
            self.view_search(),
            space().height(Length::Fixed(PICKER_TREE_GAP)),
            mouse_area(self.tree.view(self.content.label()).map(Message::Tree)).on_press(Message::Deselect),
        ]
            .spacing(0)
            .height(Length::Fill);

        let panel = container(sidebar)
            .width(Length::Fixed(PANEL_WIDTH))
            .height(Length::Fill)
            .padding(PANEL_PADDING)
            .style(theme::dark_panel_container);

        let base = row![slide(panel, self.sidebar_open, Slide::Left).snap(self.entered), self.view_workspace()]
            .width(Length::Fill)
            .height(Length::Fill);

        let hover = row![
            slide(space().width(Length::Fixed(PANEL_WIDTH)), self.sidebar_open, Slide::Left).snap(self.entered),
            self.view_toggle(),
        ]
            .height(Length::Fill)
            .align_y(Alignment::Start);

        stack![base, hover, self.view_notice()].width(Length::Fill).height(Length::Fill).into()
    }

    fn view_controls(&self) -> Element<'_, Message> {
        let style: fn(&Theme, pick_list::Status) -> pick_list::Style =
            if self.stale() { theme::combo_box_stale } else { theme::combo_box };

        let selected = self.mount.as_ref().filter(|_| !self.mounts.is_empty());

        let mount = pick_list(self.mounts.as_slice(), selected, Message::MountSelected)
            .placeholder("Mount")
            .text_size(PICKER_TEXT_SIZE)
            .padding(PICKER_PADDING)
            .width(Length::Fill)
            .style(style)
            .menu_style(theme::combo_box_menu);

        let mode = pick_list(&Mode::ALL[..], Some(self.mode), Message::ModeSelected)
            .text_size(PICKER_TEXT_SIZE)
            .padding(PICKER_PADDING)
            .width(Length::Fixed(MODE_WIDTH))
            .style(theme::combo_box)
            .menu_style(theme::combo_box_menu);

        row![mount, mode].spacing(PICKER_GAP).align_y(Vertical::Center).into()
    }

    fn view_search(&self) -> Element<'_, Message> {
        text_input("Search File...", &self.search_query)
            .on_input(Message::SearchChanged)
            .size(PICKER_TEXT_SIZE)
            .padding(PICKER_PADDING)
            .width(Length::Fill)
            .style(theme::rounded_input)
            .into()
    }

    fn view_notice(&self) -> Element<'_, Message> {
        let banner = container(theme::centered_text(LOCKED_NOTICE).size(NOTICE_TEXT_SIZE))
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .padding(Padding {
                top: NOTICE_OVERHANG + NOTICE_PADDING_Y,
                right: NOTICE_PADDING_X,
                bottom: NOTICE_PADDING_Y,
                left: NOTICE_PADDING_X,
            })
            .style(theme::notice_banner);

        container(slide(banner, self.notice.get().is_some(), Slide::Up))
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .padding(Padding::default().top(-NOTICE_OVERHANG))
            .into()
    }

    fn view_toggle(&self) -> Element<'_, Message> {
        let arrow = if self.sidebar_open { ARROW_OPEN } else { ARROW_SHUT };

        let toggle = button(
            theme::centered_text(arrow)
                .font(fonts::MISC_SYMBOLS)
                .size(TOGGLE_ARROW_SIZE)
                .width(Length::Fill)
                .height(Length::Fill),
        )
            .width(TOGGLE_BUTTON_SIZE)
            .height(TOGGLE_BUTTON_SIZE)
            .padding(0)
            .on_press(Message::ToggleSidebar)
            .style(theme::neutral_button);

        container(toggle)
            .padding(Padding { top: TOGGLE_BUTTON_GAP, left: TOGGLE_BUTTON_GAP, right: TOGGLE_BUTTON_GAP, ..Padding::ZERO })
            .into()
    }

    fn view_workspace(&self) -> Element<'_, Message> {
        let surface = container(self.body.view().map(Message::Body))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(BODY_PADDING)
            .style(theme::workspace_container);

        let reserve = (TOGGLE_BUTTON_GAP + TOGGLE_BUTTON_SIZE).max(theme::NAV_TOGGLE_RIGHT + theme::NAV_TOGGLE_SIZE);

        let strip = container(self.view_header())
            .width(Length::Fill)
            .height(Length::Fixed(HEADER_TOP_GAP + HEADER_HEIGHT))
            .align_x(Horizontal::Center)
            .align_y(Vertical::Bottom)
            .padding(Padding::default().left(reserve).right(reserve));

        let body = column![strip, surface].spacing(HEADER_BODY_GAP).height(Length::Fill);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(Padding {
                top: 0.0,
                right: TOGGLE_BUTTON_GAP,
                bottom: TOGGLE_BUTTON_GAP,
                left: TOGGLE_BUTTON_GAP,
            })
            .into()
    }

    fn view_header(&self) -> Element<'_, Message> {
        let label = self.selection_label().unwrap_or_else(|| HEADER_EMPTY.to_string());
        let header_font = Font { weight: font::Weight::Bold, ..Font::MONOSPACE };

        container(
            iced::widget::text(label)
                .size(HEADER_TEXT_SIZE)
                .font(header_font)
                .align_x(Horizontal::Center)
                .wrapping(iced::widget::text::Wrapping::None),
        )
            .height(Length::Fixed(HEADER_HEIGHT))
            .align_y(Vertical::Center)
            .padding(Padding::default().left(HEADER_PADDING).right(HEADER_PADDING))
            .style(theme::workspace_header_container)
            .into()
    }
}
