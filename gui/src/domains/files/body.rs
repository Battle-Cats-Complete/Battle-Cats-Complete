use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, operation, responsive, scrollable, space, stack, text, text_editor};
use iced::{widget, Element, Font, Length, Padding, Size, Task, Theme};
use tracing::warn;

use kore::common::preview::{self, Preview, Stamp};
use kore::domains::settings::EditorMode;
use kore::Vfs;

use crate::app::theme;
use crate::common::fonts;

use super::picture;
use super::{both_ways, EMPTY_TEXT_SIZE, TEXT_SIZE};

const OVERSIZED_LABEL: &str = "File Too Large to Preview";
const BINARY_LABEL: &str = "No Preview for Binary Files";

const UPLOAD_GLYPH_SIZE: f32 = 16.0;
const UPLOAD_INSET: f32 = 2.0;

const CHAR_WIDTH: f32 = TEXT_SIZE * 0.6;
const TAB_COLUMNS: usize = 8;
const LINE_HEIGHT: f32 = TEXT_SIZE * 1.3;
const DOCUMENT_PADDING: f32 = 4.0;
const DOCUMENT_CLEARANCE: f32 = 14.0;

const OPENING_LINES: usize = 80;
const WINDOW_FACTOR: usize = 3;
const BUFFER_LINES: usize = 12;

#[derive(Debug, Clone)]
pub enum Message {
    Picture(picture::Message),
    Edit(text_editor::Action),
    Scrolled(scrollable::Viewport),
    Upload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Outcome {
    Idle,
    Refused,
    Upload,
}

struct Draft {
    mode: EditorMode,
    body: String,
    starts: Vec<usize>,
    window: Range<usize>,
    editor: text_editor::Content,
    stamp: Stamp,
    staged: bool,
    dirty: bool,
    widest: f32,
}

impl Draft {
    fn new(body: String, stamp: Stamp, mode: EditorMode) -> Self {
        let starts = Self::index(&body);
        let widest = Self::measure(&body);

        let opening = match mode {
            EditorMode::Virtual => OPENING_LINES.min(starts.len()),
            EditorMode::Fill => starts.len(),
        };

        let window = 0..opening;

        let mut draft = Self {
            mode,
            body,
            starts,
            window,
            editor: text_editor::Content::new(),
            stamp,
            staged: false,
            dirty: false,
            widest,
        };

        draft.reload();
        draft
    }

    fn index(body: &str) -> Vec<usize> {
        let mut starts = Vec::with_capacity(body.len() / 32 + 1);
        starts.push(0);
        starts.extend(body.match_indices('\n').map(|(offset, _)| offset + 1));
        starts
    }

    fn bounds(&self, window: &Range<usize>) -> (usize, usize) {
        let from = self.starts.get(window.start).copied().unwrap_or(self.body.len());
        let to = self.starts.get(window.end).map_or(self.body.len(), |next| next.saturating_sub(1));

        (from, to.max(from))
    }

    fn reach(&self) -> usize {
        self.window.start + self.editor.line_count()
    }

    fn total(&self) -> usize {
        if self.staged {
            return self.starts.len() + self.editor.line_count() - self.window.len();
        }

        self.starts.len()
    }

    fn flush(&mut self) {
        if !self.staged {
            return;
        }

        let (from, to) = self.bounds(&self.window);
        let text = self.editor.text();

        self.body.replace_range(from..to, &text);
        self.starts = Self::index(&self.body);
        self.window = self.window.start..self.window.start + self.editor.line_count();
        self.staged = false;
    }

    fn reload(&mut self) {
        let (from, to) = self.bounds(&self.window);

        self.editor = text_editor::Content::with_text(&self.body[from..to]);
    }

    fn retarget(&mut self, window: Range<usize>) {
        if window == self.window {
            return;
        }

        let anchor = self.editor.cursor();
        let absolute = self.window.start + anchor.position.line;

        self.flush();

        let limit = self.starts.len();
        self.window = window.start.min(limit)..window.end.min(limit);
        self.reload();

        if self.window.contains(&absolute) {
            self.editor.move_to(text_editor::Cursor {
                position: text_editor::Position {
                    line: absolute - self.window.start,
                    column: anchor.position.column,
                },
                selection: None,
            });
        }
    }

    fn reopen(&mut self, anchor: usize) {
        if self.mode == EditorMode::Fill {
            return;
        }

        let span = self.window.len().max(OPENING_LINES);
        let start = anchor.min(self.starts.len().saturating_sub(1));

        self.retarget(start..(start + span).min(self.starts.len()));
    }

    fn scrolled(&mut self, offset: f32, height: f32) {
        if self.mode == EditorMode::Fill {
            return;
        }

        let visible = ((height / LINE_HEIGHT).ceil() as usize).max(1);
        let first = (offset / LINE_HEIGHT).floor() as usize;
        let last = (first + visible).min(self.total());

        let reach = self.reach();
        let room_above = self.window.start > 0 && first < self.window.start + BUFFER_LINES;
        let room_below = reach < self.total() && last + BUFFER_LINES > reach;

        if !room_above && !room_below {
            return;
        }

        let span = (visible * WINDOW_FACTOR).max(OPENING_LINES);
        let start = first.saturating_sub(span.saturating_sub(visible) / 2);
        let end = (start + span).min(self.total());

        self.retarget(start.min(end)..end);
    }

    fn columns(line: &str) -> usize {
        line.chars().fold(0, |column, character| match character {
            '\t' => (column / TAB_COLUMNS + 1) * TAB_COLUMNS,
            _ => column + 1,
        })
    }

    fn span(columns: usize) -> f32 {
        DOCUMENT_PADDING * 2.0 + CHAR_WIDTH * columns as f32
    }

    fn extent(lines: usize) -> f32 {
        DOCUMENT_PADDING * 2.0 + LINE_HEIGHT * lines as f32
    }

    fn measure(body: &str) -> f32 {
        Self::span(body.lines().map(Self::columns).max().unwrap_or(0))
    }

    fn grow(&mut self) {
        let row = self.editor.cursor().position.line;

        let Some(line) = self.editor.line(row) else {
            return;
        };

        self.widest = self.widest.max(Self::span(Self::columns(&line.text)));
    }
}

#[derive(Default)]
enum Content {
    #[default]
    Empty,
    Image { source: picture::Source, stamp: Stamp },
    Text(Draft),
    Notice(&'static str),
}

pub(super) struct State {
    loaded: Option<PathBuf>,
    framed: Option<PathBuf>,
    content: Content,
    picture: picture::State,
    writable: bool,
    stale: bool,
    scroll_id: widget::Id,
}

impl Default for State {
    fn default() -> Self {
        Self {
            loaded: None,
            framed: None,
            content: Content::default(),
            picture: picture::State::default(),
            writable: false,
            stale: false,
            scroll_id: widget::Id::unique(),
        }
    }
}

impl State {
    pub(super) fn invalidate(&mut self) {
        self.stale = true;
    }

    fn resume(&self) -> Option<usize> {
        match &self.content {
            Content::Text(draft) => Some(draft.window.start),
            _ => None,
        }
    }

    pub(super) fn recenter(&mut self) -> Task<Message> {
        self.picture.reset();

        let Content::Text(draft) = &mut self.content else {
            return Task::none();
        };

        draft.reopen(0);

        operation::scroll_to(self.scroll_id.clone(), scrollable::AbsoluteOffset { x: 0.0, y: 0.0 })
    }

    pub(super) fn update(&mut self, message: Message) -> (Outcome, Task<Message>) {
        match message {
            Message::Picture(msg) => {
                self.picture.update(msg);
                (Outcome::Idle, Task::none())
            }
            Message::Scrolled(viewport) => {
                if let Content::Text(draft) = &mut self.content {
                    draft.scrolled(viewport.absolute_offset().y, viewport.bounds().height);
                }

                (Outcome::Idle, Task::none())
            }
            Message::Upload => {
                let outcome = if self.writable { Outcome::Upload } else { Outcome::Refused };

                (outcome, Task::none())
            }
            Message::Edit(text_editor::Action::Scroll { lines }) => {
                let offset = scrollable::AbsoluteOffset { x: 0.0, y: lines as f32 * LINE_HEIGHT };

                (Outcome::Idle, operation::scroll_by(self.scroll_id.clone(), offset))
            }
            Message::Edit(action) => {
                let Content::Text(draft) = &mut self.content else {
                    return (Outcome::Idle, Task::none());
                };

                if self.writable {
                    let edited = action.is_edit();

                    draft.dirty |= edited;
                    draft.staged |= edited;
                    draft.editor.perform(action);

                    if edited {
                        draft.grow();
                    }

                    return (Outcome::Idle, Task::none());
                }

                if !action.is_edit() {
                    draft.editor.perform(action);

                    return (Outcome::Idle, Task::none());
                }

                (Outcome::Refused, Task::none())
            }
        }
    }

    pub(super) fn replace(&mut self, vfs: &Vfs, mount: Option<&str>, path: Option<&Path>, source: &Path) -> bool {
        let Content::Image { stamp, .. } = &self.content else {
            return false;
        };

        if !self.writable {
            return false;
        }

        let Ok(bytes) = fs::read(source).inspect_err(|err| warn!(path = %source.display(), "upload read failed: {}", err))
        else {
            return false;
        };

        if !preview::is_png(&bytes) {
            warn!(path = %source.display(), "refusing to upload a file that is not a PNG");
            return false;
        }

        let resolved = mount
            .and_then(|mount| vfs.root(mount))
            .zip(path)
            .map(|(root, relative)| root.join(relative));

        let Some(target) = resolved else {
            return false;
        };

        match preview::save(&target, &bytes, *stamp) {
            Ok(_) => true,
            Err(err) => {
                warn!(path = %target.display(), "upload failed: {}", err);
                false
            }
        }
    }

    pub(super) fn commit(&mut self, vfs: &Vfs, mount: Option<&str>, path: Option<&Path>) {
        let Content::Text(draft) = &mut self.content else {
            return;
        };

        if !draft.dirty || !self.writable {
            return;
        }

        let resolved = mount
            .and_then(|mount| vfs.root(mount))
            .zip(path)
            .map(|(root, relative)| root.join(relative));

        let Some(target) = resolved else {
            return;
        };

        draft.flush();

        match preview::save(&target, draft.body.as_bytes(), draft.stamp) {
            Ok(stamp) => {
                draft.stamp = stamp;
                draft.dirty = false;
            }
            Err(err) => warn!(path = %target.display(), "discarding edits: {}", err),
        }
    }

    pub(super) fn refresh(
        &mut self,
        vfs: &Vfs,
        mount: Option<&str>,
        selected: Option<&Path>,
        writable: bool,
        mode: EditorMode,
    ) {
        let Some(relative) = selected else {
            let previous = self.loaded.take();

            self.commit(vfs, mount, previous.as_deref());
            self.writable = writable;
            self.framed = None;
            self.content = Content::Empty;
            self.picture.reset();

            return;
        };

        let reloaded = self.loaded.as_deref() == selected;
        let settled = matches!(&self.content, Content::Text(draft) if draft.mode == mode);

        if reloaded && self.writable == writable && !self.stale && settled {
            return;
        }

        if !reloaded {
            let previous = self.loaded.clone();
            self.commit(vfs, mount, previous.as_deref());
        }

        let resume = reloaded.then(|| self.resume()).flatten();

        self.stale = false;
        self.loaded = Some(relative.to_path_buf());
        self.writable = writable;

        let resolved = mount.and_then(|mount| vfs.root(mount)).map(|root| root.join(relative));

        let Some(path) = resolved else {
            return;
        };

        match preview::load(&path) {
            Preview::Image { bytes, width, height, stamp } => {
                if self.framed.as_deref() != Some(relative) {
                    self.framed = Some(relative.to_path_buf());
                    self.picture.reset();
                }

                self.content = Content::Image { source: picture::Source::new(bytes, width, height), stamp };
            }
            Preview::Text { body, stamp } => {
                let mut draft = Draft::new(body, stamp, mode);

                if let Some(anchor) = resume {
                    draft.reopen(anchor);
                }

                self.content = Content::Text(draft);
            }
            Preview::Oversized => self.content = Content::Notice(OVERSIZED_LABEL),
            Preview::Binary => self.content = Content::Notice(BINARY_LABEL),
            Preview::Unavailable => {}
        }
    }

    pub(super) fn view(&self) -> Element<'_, Message> {
        match &self.content {
            Content::Empty => space().into(),
            Content::Notice(label) => container(theme::centered_text(*label).size(EMPTY_TEXT_SIZE))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .into(),
            Content::Text(draft) => self.view_document(draft),
            Content::Image { source, .. } => stack![
                self.picture.view(source).map(Message::Picture),
                self.view_upload(),
            ]
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        }
    }

    fn view_upload(&self) -> Element<'_, Message> {
        let writable = self.writable;

        let upload = button(
            theme::centered_text(fonts::UPLOAD)
                .font(fonts::MISC_SYMBOLS)
                .size(UPLOAD_GLYPH_SIZE)
                .line_height(fonts::MISC_SYMBOLS_LINE_HEIGHT),
        )
            .width(Length::Fixed(theme::OVERLAY_BUTTON_SIZE))
            .height(Length::Fixed(theme::OVERLAY_BUTTON_SIZE))
            .padding(0)
            .on_press(Message::Upload)
            .style(move |t: &Theme, status| theme::overlay_button(t, status, writable));

        container(upload)
            .width(Length::Fill)
            .align_x(Horizontal::Right)
            .padding(UPLOAD_INSET)
            .into()
    }

    fn view_document<'a>(&'a self, draft: &'a Draft) -> Element<'a, Message> {
        responsive(move |size: Size| {
            let width = draft.widest.max(size.width - DOCUMENT_CLEARANCE);
            let total = draft.total();

            let page = text_editor(&draft.editor)
                .on_action(Message::Edit)
                .font(Font::MONOSPACE)
                .size(TEXT_SIZE)
                .wrapping(text::Wrapping::None)
                .width(width)
                .height(Length::Fixed(Draft::extent(draft.editor.line_count())))
                .padding(DOCUMENT_PADDING)
                .style(theme::plain_editor);

            let above = draft.window.start as f32 * LINE_HEIGHT;
            let below = total.saturating_sub(draft.reach()) as f32 * LINE_HEIGHT;

            let document = column![space().height(above), page, space().height(below)].width(width);

            scrollable(container(document).padding(Padding::default().right(DOCUMENT_CLEARANCE).bottom(DOCUMENT_CLEARANCE)))
                .id(self.scroll_id.clone())
                .direction(both_ways())
                .on_scroll(Message::Scrolled)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        })
            .into()
    }
}
