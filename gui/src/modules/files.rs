use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    button, column, container, operation, pick_list, responsive, row, scrollable, space, stack, text, text_input,
    Column,
};
use iced::{font, widget, Alignment, Element, Font, Length, Padding, Size, Task, Theme};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use tracing::info;

use core::modules::settings::nightly;
use core::Vfs;

use crate::app::state::FilesState;
use crate::app::theme;
use crate::common::fonts;
use crate::common::row_window::{self, RowWindow};
use crate::common::watcher;
use crate::widget::{list_row, slide, smooth_scroll, Slide};

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
const HEADER_SEPARATOR: &str = " :: ";
const HEADER_EMPTY: &str = "Please select a file";
const PICKER_GAP: f32 = 4.0;
const PICKER_TREE_GAP: f32 = 8.0;
const MODE_WIDTH: f32 = 60.0;

const PICKER_TEXT_SIZE: f32 = 13.0;
const PICKER_PADDING: [u16; 2] = [4, 8];
const EMPTY_TEXT_SIZE: f32 = 14.0;

const TEXT_SIZE: f32 = 13.0;
const CHAR_WIDTH: f32 = TEXT_SIZE * 0.65;
const ROW_HEIGHT: f32 = 24.0;
const ROW_SPACING: f32 = 0.0;
const ROW_PADDING: f32 = 6.0;
const INDENT: f32 = 12.0;

const MARKER_SIZE: f32 = 22.0;
const MARKER_LINE_HEIGHT: f32 = ROW_HEIGHT / MARKER_SIZE;
const MARKER_WIDTH: f32 = 16.0;

const SCROLLBAR_WIDTH: f32 = 6.0;
const SCROLLBAR_MARGIN: f32 = 2.0;
const SCROLLBAR_ALLOWANCE: f32 = 14.0;
const SCROLL_TAIL: f32 = 14.0;

const FOLDER_OPEN: &str = "\u{25be}";
const FOLDER_SHUT: &str = "\u{25b8}";

const EMPTY_LABEL: &str = "No Files Found on Mount";
const MISSING_LABEL: &str = "Selected Mount Missing from Memory";

pub(crate) fn register_nightly() {
    nightly::register_nightly_usage();
}

#[derive(Debug, Clone)]
pub enum Message {
    MountSelected(String),
    ModeSelected(Mode),
    SearchChanged(String),
    ToggleSidebar,
    Activate(usize),
    Scrolled(f32),
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

struct Row {
    name: Box<str>,
    depth: u16,
    folder: bool,
    expanded: bool,
}

pub struct State {
    mounts: Vec<String>,
    mount: Option<String>,
    mode: Mode,
    search_query: String,
    sidebar_open: bool,
    entered: bool,
    verify: bool,
    expanded: FxHashSet<PathBuf>,
    selected: Option<PathBuf>,
    flat_keys: Vec<Box<str>>,
    rows: Vec<Row>,
    selected_row: Option<usize>,
    populated: bool,
    has_folders: bool,
    widest: f32,
    scroll_offset: f32,
    scroll_id: widget::Id,
}

impl Default for State {
    fn default() -> Self {
        Self {
            mounts: Vec::new(),
            mount: None,
            mode: Mode::default(),
            search_query: String::new(),
            sidebar_open: true,
            entered: true,
            verify: false,
            expanded: FxHashSet::from_iter([PathBuf::new()]),
            selected: None,
            flat_keys: Vec::new(),
            rows: Vec::new(),
            selected_row: None,
            populated: false,
            has_folders: false,
            widest: 0.0,
            scroll_offset: 0.0,
            scroll_id: widget::Id::unique(),
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

    pub(crate) fn sync(&mut self, vfs: &Vfs) {
        self.entered = true;

        let keys: Vec<String> = vfs.mount_keys().iter().map(|key| key.to_string()).collect();

        if keys != self.mounts {
            self.mounts = keys;
        }

        if self.verify && !self.mounts.is_empty() {
            self.verify = false;

            if let Some(missing) = self.stale().then(|| self.mount.take()).flatten() {
                info!(mount = %missing, "Persisted mount is no longer indexed, falling back to the first mount");
                self.selected = None;
            }
        }

        if self.mount.is_none() {
            self.mount = self.mounts.first().cloned();
            self.reset();
        }

        self.refresh_keys(vfs);
        self.rebuild(vfs);
    }

    fn refresh_keys(&mut self, vfs: &Vfs) {
        self.flat_keys.clear();

        if self.mode != Mode::Flat {
            return;
        }

        let Some(mount) = self.mount.as_deref() else {
            return;
        };

        self.flat_keys = vfs.keys(mount);
        self.flat_keys.sort_unstable();
    }

    pub(crate) fn apply_changes(&mut self, vfs: &Vfs, paths: &[PathBuf]) {
        let Some(mount) = self.mount.as_deref() else {
            return;
        };

        let showing = paths.iter().any(|path| {
            if watcher::mount_of(path).is_none_or(|key| key != mount) {
                return false;
            }

            if self.mode == Mode::Flat {
                return true;
            }

            vfs.relative(mount, path)
                .as_deref()
                .and_then(Path::parent)
                .is_some_and(|parent| self.expanded.contains(parent))
        });

        if !showing {
            return;
        }

        self.refresh_keys(vfs);
        self.rebuild(vfs);
    }

    pub(crate) fn update(&mut self, message: Message, vfs: &Vfs) -> Task<Message> {
        match message {
            Message::MountSelected(mount) => {
                if self.mount.as_deref() == Some(mount.as_str()) {
                    return Task::none();
                }

                self.mount = Some(mount);
                self.reset();
                self.refresh_keys(vfs);
                self.rebuild(vfs);

                self.snap_to_top()
            }
            Message::ModeSelected(mode) => {
                if self.mode == mode {
                    return Task::none();
                }

                self.mode = mode;
                self.scroll_offset = 0.0;
                self.refresh_keys(vfs);
                self.rebuild(vfs);

                self.snap_to_top()
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
                self.scroll_offset = 0.0;
                self.rebuild(vfs);

                self.snap_to_top()
            }
            Message::Activate(index) => {
                let Some(folder) = self.rows.get(index).map(|row| row.folder) else {
                    return Task::none();
                };

                let path = if self.mode == Mode::Flat {
                    self.flat_path(vfs, index)
                } else {
                    self.path_of(index)
                };

                let Some(path) = path else {
                    return Task::none();
                };

                if !folder {
                    self.selected = Some(path);
                } else if !self.expanded.remove(&path) {
                    self.expanded.insert(path);
                }

                self.rebuild(vfs);

                Task::none()
            }
            Message::ToggleSidebar => {
                self.sidebar_open = !self.sidebar_open;
                self.entered = false;
                Task::none()
            }
            Message::Scrolled(offset) => {
                self.scroll_offset = offset;
                Task::none()
            }
        }
    }

    fn reset(&mut self) {
        self.expanded.clear();
        self.expanded.insert(PathBuf::new());
        self.selected = None;
        self.scroll_offset = 0.0;
    }

    fn snap_to_top(&self) -> Task<Message> {
        operation::scroll_to(self.scroll_id.clone(), scrollable::AbsoluteOffset { x: 0.0, y: 0.0 })
    }

    fn flat_path(&self, vfs: &Vfs, index: usize) -> Option<PathBuf> {
        let name = self.rows.get(index)?.name.as_ref();
        let mount = self.mount.as_deref()?;

        vfs.locate_in(mount, name)
    }

    fn rebuild(&mut self, vfs: &Vfs) {
        self.rows.clear();
        self.selected_row = None;
        self.widest = 0.0;
        self.populated = false;
        self.has_folders = false;

        let Some(mount) = self.mount.as_deref() else {
            return;
        };

        if vfs.count(mount) == 0 {
            return;
        }

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

        let anchor = self
            .selected
            .as_deref()
            .and_then(|path| Some((path.parent()?, path.file_name()?.to_str()?)));

        let mut flatten = Flatten {
            vfs,
            mount,
            expanded: &self.expanded,
            anchor,
            query: self.search_query.trim(),
            flat_keys: &self.flat_keys,
            rows: Vec::new(),
            selected_row: None,
            folders: false,
            widest: 0.0,
        };

        match self.mode {
            Mode::Tree => flatten.walk(Path::new(""), 0),
            Mode::Flat => flatten.flat(),
        }

        self.populated = !flatten.rows.is_empty();
        self.has_folders = flatten.folders;
        self.widest = flatten.widest + if flatten.folders { MARKER_WIDTH } else { 0.0 };
        self.rows = flatten.rows;
        self.selected_row = flatten.selected_row;
    }

    fn path_of(&self, index: usize) -> Option<PathBuf> {
        let target = self.rows.get(index)?;
        let mut parts: Vec<&str> = vec![target.name.as_ref()];
        let mut wanted = target.depth;

        for row in self.rows[..index].iter().rev() {
            if wanted == 0 {
                break;
            }

            if row.depth == wanted - 1 {
                wanted -= 1;
                parts.push(row.name.as_ref());
            }
        }

        let mut path = PathBuf::new();

        for part in parts.iter().rev() {
            path.push(part);
        }

        Some(path)
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        let sidebar = column![
            self.view_controls(),
            space().height(Length::Fixed(PICKER_GAP)),
            self.view_search(),
            space().height(Length::Fixed(PICKER_TREE_GAP)),
            self.view_tree(),
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

        stack![base, hover].width(Length::Fill).height(Length::Fill).into()
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
        let surface = container(space())
            .width(Length::Fill)
            .height(Length::Fill)
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
            text(label)
                .size(HEADER_TEXT_SIZE)
                .font(header_font)
                .align_x(Horizontal::Center)
                .wrapping(text::Wrapping::None),
        )
            .height(Length::Fixed(HEADER_HEIGHT))
            .align_y(Vertical::Center)
            .padding(Padding::default().left(HEADER_PADDING).right(HEADER_PADDING))
            .style(theme::workspace_header_container)
            .into()
    }

    fn selection_label(&self) -> Option<String> {
        let mount = self.mount.as_deref()?;
        let name = self.selected.as_deref()?.file_name()?.to_str()?;

        Some(format!("{}{}{}", mount, HEADER_SEPARATOR, name))
    }

    fn stale(&self) -> bool {
        !self.mounts.is_empty() && self.mount.as_ref().is_some_and(|mount| !self.mounts.contains(mount))
    }

    fn view_controls(&self) -> Element<'_, Message> {
        let style: fn(&Theme, pick_list::Status) -> pick_list::Style =
            if self.stale() { theme::combo_box_stale } else { theme::combo_box };

        let mount = pick_list(self.mounts.as_slice(), self.mount.as_ref(), Message::MountSelected)
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

    fn view_tree(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = if self.populated {
            responsive(move |size: Size| self.view_rows(size)).into()
        } else {
            let label = if self.stale() { MISSING_LABEL } else { EMPTY_LABEL };

            container(theme::centered_text(label).size(EMPTY_TEXT_SIZE).style(text::danger))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        };

        container(container(body).padding(theme::CONSOLE_BORDER_WIDTH))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::mock_console_container)
            .into()
    }

    fn view_rows(&self, size: Size) -> Element<'_, Message> {
        let tail = if self.widest > size.width { SCROLL_TAIL } else { 0.0 };

        let RowWindow { range, pad_before, pad_after } =
            row_window::compute_with(self.rows.len(), size.height - tail, self.scroll_offset, ROW_HEIGHT, ROW_SPACING);

        let width = self.widest.max(size.width - SCROLLBAR_ALLOWANCE);
        let mut list = Column::with_capacity(range.len() + 3).spacing(ROW_SPACING);

        if pad_before > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_before)));
        }

        for index in range {
            let Some(row) = self.rows.get(index) else {
                continue;
            };

            list = list.push(self.view_row(index, row, width));
        }

        if pad_after > 0.0 {
            list = list.push(space().height(Length::Fixed(pad_after)));
        }

        if tail > 0.0 {
            list = list.push(space().height(Length::Fixed(tail)));
        }

        let scrollbar = || scrollable::Scrollbar::new().width(SCROLLBAR_WIDTH).margin(SCROLLBAR_MARGIN);

        smooth_scroll(
            scrollable(list)
                .id(self.scroll_id.clone())
                .direction(scrollable::Direction::Both { vertical: scrollbar(), horizontal: scrollbar() })
                .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
                .width(Length::Fill)
                .height(Length::Fill),
        )
            .into()
    }

    fn view_row<'a>(&self, index: usize, row: &'a Row, width: f32) -> Element<'a, Message> {
        let marker = match (row.folder, row.expanded) {
            (true, true) => FOLDER_OPEN,
            (true, false) => FOLDER_SHUT,
            (false, _) => "",
        };

        let name = text(row.name.as_ref()).font(Font::MONOSPACE).size(TEXT_SIZE).wrapping(text::Wrapping::None);

        let label = if self.has_folders {
            row![
                text(marker)
                    .font(Font::MONOSPACE)
                    .size(MARKER_SIZE)
                    .line_height(MARKER_LINE_HEIGHT)
                    .width(Length::Fixed(MARKER_WIDTH)),
                name,
            ]
        } else {
            row![name]
        };

        let label = label.align_y(Vertical::Center);

        let content = container(label)
            .height(Length::Fixed(ROW_HEIGHT))
            .align_y(Vertical::Center)
            .padding(Padding::default().left(ROW_PADDING + INDENT * f32::from(row.depth)).right(ROW_PADDING));

        list_row(content, self.selected_row == Some(index), false, Length::Fixed(width), Message::Activate(index))
    }
}

struct Flatten<'a> {
    vfs: &'a Vfs,
    mount: &'a str,
    expanded: &'a FxHashSet<PathBuf>,
    anchor: Option<(&'a Path, &'a str)>,
    query: &'a str,
    flat_keys: &'a [Box<str>],
    rows: Vec<Row>,
    selected_row: Option<usize>,
    folders: bool,
    widest: f32,
}

fn matches(name: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let (name, query) = (name.as_bytes(), query.as_bytes());

    name.len() >= query.len() && name.windows(query.len()).any(|window| window.eq_ignore_ascii_case(query))
}

impl Flatten<'_> {
    fn push(&mut self, row: Row) {
        let span = ROW_PADDING * 2.0
            + INDENT * f32::from(row.depth)
            + CHAR_WIDTH * row.name.chars().count() as f32;

        self.widest = self.widest.max(span);
        self.rows.push(row);
    }

    fn flat(&mut self) {
        let marked = self.anchor.map(|(_, name)| name);

        for name in self.flat_keys {
            if !matches(name, self.query) {
                continue;
            }

            if marked == Some(name.as_ref()) {
                self.selected_row = Some(self.rows.len());
            }

            self.push(Row { name: name.clone(), depth: 0, folder: false, expanded: false });
        }
    }

    fn walk(&mut self, dir: &Path, depth: u16) {
        let Some(listing) = self.vfs.browse(self.mount, dir) else {
            return;
        };

        let marked = self.anchor.filter(|(parent, _)| *parent == dir).map(|(_, name)| name);
        let query = self.query;

        for folder in listing.folders {
            let path = dir.join(folder.as_ref());

            if !self.vfs.any_file(self.mount, &path, |name| matches(name, query)) {
                continue;
            }

            let open = self.expanded.contains(&path);

            self.folders = true;
            self.push(Row { name: folder, depth, folder: true, expanded: open });

            if open {
                self.walk(&path, depth + 1);
            }
        }

        for file in listing.files {
            if !matches(&file, self.query) {
                continue;
            }

            if marked == Some(file.as_ref()) {
                self.selected_row = Some(self.rows.len());
            }

            self.push(Row { name: file, depth, folder: false, expanded: false });
        }
    }
}
