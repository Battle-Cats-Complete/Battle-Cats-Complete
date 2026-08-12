use std::path::{Path, PathBuf};

use iced::alignment::Vertical;
use iced::widget::{column, container, operation, pick_list, responsive, row, scrollable, space, text, Column};
use iced::{widget, Element, Font, Length, Padding, Size, Task, Theme};
use rustc_hash::FxHashSet;

use core::modules::settings::nightly;
use core::Vfs;

use crate::app::theme;
use crate::common::row_window::{self, RowWindow};
use crate::common::watcher;
use crate::widget::{list_row, smooth_scroll};

const PANEL_WIDTH: f32 = 320.0;
const PANEL_PADDING: f32 = 8.0;
const PICKER_TREE_GAP: f32 = 8.0;

const PICKER_TEXT_SIZE: f32 = 13.0;
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
    Activate(usize),
    Scrolled(f32),
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
    expanded: FxHashSet<PathBuf>,
    selected: Option<PathBuf>,
    rows: Vec<Row>,
    selected_row: Option<usize>,
    populated: bool,
    widest: f32,
    scroll_offset: f32,
    scroll_id: widget::Id,
}

impl Default for State {
    fn default() -> Self {
        Self {
            mounts: Vec::new(),
            mount: None,
            expanded: FxHashSet::from_iter([PathBuf::new()]),
            selected: None,
            rows: Vec::new(),
            selected_row: None,
            populated: false,
            widest: 0.0,
            scroll_offset: 0.0,
            scroll_id: widget::Id::unique(),
        }
    }
}

impl State {
    pub(crate) fn sync(&mut self, vfs: &Vfs) {
        let keys: Vec<String> = vfs.mount_keys().iter().map(|key| key.to_string()).collect();

        if keys != self.mounts {
            self.mounts = keys;
        }

        if self.mount.is_none() {
            self.mount = self.mounts.first().cloned();
            self.reset();
        }

        self.rebuild(vfs);
    }

    pub(crate) fn apply_changes(&mut self, vfs: &Vfs, paths: &[PathBuf]) {
        let Some(mount) = self.mount.as_deref() else {
            return;
        };

        let showing = paths.iter().any(|path| {
            watcher::mount_of(path).is_some_and(|key| key == mount)
                && vfs
                    .relative(mount, path)
                    .as_deref()
                    .and_then(Path::parent)
                    .is_some_and(|parent| self.expanded.contains(parent))
        });

        if !showing {
            return;
        }

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
                self.rebuild(vfs);

                operation::scroll_to(self.scroll_id.clone(), scrollable::AbsoluteOffset { x: 0.0, y: 0.0 })
            }
            Message::Activate(index) => {
                let Some(folder) = self.rows.get(index).map(|row| row.folder) else {
                    return Task::none();
                };

                let Some(path) = self.path_of(index) else {
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

    fn rebuild(&mut self, vfs: &Vfs) {
        self.rows.clear();
        self.selected_row = None;
        self.widest = 0.0;
        self.populated = false;

        let Some(mount) = self.mount.as_deref() else {
            return;
        };

        if vfs.count(mount) == 0 {
            return;
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
            rows: Vec::new(),
            selected_row: None,
            widest: 0.0,
        };

        flatten.walk(Path::new(""), 0);

        self.populated = !flatten.rows.is_empty();
        self.rows = flatten.rows;
        self.selected_row = flatten.selected_row;
        self.widest = flatten.widest;
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
            self.view_picker(),
            space().height(Length::Fixed(PICKER_TREE_GAP)),
            self.view_tree(),
        ]
            .spacing(0)
            .height(Length::Fill);

        let panel = container(sidebar)
            .width(Length::Fixed(PANEL_WIDTH))
            .height(Length::Fill)
            .padding(PANEL_PADDING)
            .style(theme::list_panel_container);

        row![panel].width(Length::Fill).height(Length::Fill).into()
    }

    fn stale(&self) -> bool {
        self.mount.as_ref().is_some_and(|mount| !self.mounts.contains(mount))
    }

    fn view_picker(&self) -> Element<'_, Message> {
        let style: fn(&Theme, pick_list::Status) -> pick_list::Style =
            if self.stale() { theme::combo_box_stale } else { theme::combo_box };

        pick_list(self.mounts.as_slice(), self.mount.as_ref(), Message::MountSelected)
            .placeholder("Mount")
            .text_size(PICKER_TEXT_SIZE)
            .padding([4, 8])
            .width(Length::Fill)
            .style(style)
            .menu_style(theme::combo_box_menu)
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
        let RowWindow { range, pad_before, pad_after } =
            row_window::compute_with(self.rows.len(), size.height, self.scroll_offset, ROW_HEIGHT, ROW_SPACING);

        let width = self.widest.max(size.width - SCROLLBAR_ALLOWANCE);
        let mut list = Column::with_capacity(range.len() + 2).spacing(ROW_SPACING);

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

        let label = row![
            text(marker)
                .font(Font::MONOSPACE)
                .size(MARKER_SIZE)
                .line_height(MARKER_LINE_HEIGHT)
                .width(Length::Fixed(MARKER_WIDTH)),
            text(row.name.as_ref()).font(Font::MONOSPACE).size(TEXT_SIZE).wrapping(text::Wrapping::None),
        ]
            .align_y(Vertical::Center);

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
    rows: Vec<Row>,
    selected_row: Option<usize>,
    widest: f32,
}

impl Flatten<'_> {
    fn push(&mut self, row: Row) {
        let span = ROW_PADDING * 2.0
            + MARKER_WIDTH
            + INDENT * f32::from(row.depth)
            + CHAR_WIDTH * row.name.chars().count() as f32;

        self.widest = self.widest.max(span);
        self.rows.push(row);
    }

    fn walk(&mut self, dir: &Path, depth: u16) {
        let Some(listing) = self.vfs.browse(self.mount, dir) else {
            return;
        };

        let marked = self.anchor.filter(|(parent, _)| *parent == dir).map(|(_, name)| name);

        for folder in listing.folders {
            let path = dir.join(folder.as_ref());
            let open = self.expanded.contains(&path);

            self.push(Row { name: folder, depth, folder: true, expanded: open });

            if open {
                self.walk(&path, depth + 1);
            }
        }

        for file in listing.files {
            if marked == Some(file.as_ref()) {
                self.selected_row = Some(self.rows.len());
            }

            self.push(Row { name: file, depth, folder: false, expanded: false });
        }
    }
}
