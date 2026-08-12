use std::fs;
use std::path::{Path, PathBuf};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, container, operation, responsive, scrollable, space, stack, text, text_editor};
use iced::{widget, Element, Font, Length, Padding, Size, Task, Theme};
use tracing::warn;

use core::common::preview::{self, Preview, Stamp};
use core::Vfs;

use crate::app::theme;
use crate::common::fonts;

use super::picture;
use super::{both_ways, EMPTY_TEXT_SIZE, TEXT_SIZE};

const OVERSIZED_LABEL: &str = "File Too Large to Preview";
const BINARY_LABEL: &str = "No Preview for Binary Files";

const UPLOAD_GLYPH_SIZE: f32 = 16.0;
const UPLOAD_INSET: f32 = 2.0;

const CHAR_WIDTH: f32 = TEXT_SIZE * 0.6;
const LINE_HEIGHT: f32 = TEXT_SIZE * 1.3;
const DOCUMENT_PADDING: f32 = 4.0;
const DOCUMENT_CLEARANCE: f32 = 14.0;

#[derive(Debug, Clone)]
pub enum Message {
    Picture(picture::Message),
    Edit(text_editor::Action),
    Upload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Outcome {
    Idle,
    Refused,
    Upload,
}

struct Draft {
    editor: text_editor::Content,
    stamp: Stamp,
    dirty: bool,
    widest: f32,
}

impl Draft {
    fn span(columns: usize) -> f32 {
        DOCUMENT_PADDING * 2.0 + CHAR_WIDTH * columns as f32
    }

    fn measure(body: &str) -> f32 {
        Self::span(body.lines().map(|line| line.chars().count()).max().unwrap_or(0))
    }

    fn grow(&mut self) {
        let row = self.editor.cursor().position.line;

        let Some(line) = self.editor.line(row) else {
            return;
        };

        self.widest = self.widest.max(Self::span(line.text.chars().count()));
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
            scroll_id: widget::Id::unique(),
        }
    }
}

impl State {
    pub(super) fn invalidate(&mut self) {
        self.loaded = None;
    }

    pub(super) fn recenter(&mut self) {
        self.picture.reset();
    }

    pub(super) fn update(&mut self, message: Message) -> (Outcome, Task<Message>) {
        match message {
            Message::Picture(msg) => {
                self.picture.update(msg);
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

        match preview::save(&target, draft.editor.text().as_bytes(), draft.stamp) {
            Ok(stamp) => {
                draft.stamp = stamp;
                draft.dirty = false;
            }
            Err(err) => warn!(path = %target.display(), "discarding edits: {}", err),
        }
    }

    pub(super) fn refresh(&mut self, vfs: &Vfs, mount: Option<&str>, selected: Option<&Path>, writable: bool) {
        let Some(relative) = selected else {
            let previous = self.loaded.take();

            self.commit(vfs, mount, previous.as_deref());
            self.writable = writable;
            self.framed = None;
            self.content = Content::Empty;
            self.picture.reset();

            return;
        };

        if self.loaded.as_deref() == selected && self.writable == writable {
            return;
        }

        if self.loaded.as_deref() != selected {
            let previous = self.loaded.clone();
            self.commit(vfs, mount, previous.as_deref());
        }

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
                self.content = Content::Text(Draft {
                    widest: Draft::measure(&body),
                    editor: text_editor::Content::with_text(&body),
                    stamp,
                    dirty: false,
                });
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

            let page = text_editor(&draft.editor)
                .on_action(Message::Edit)
                .font(Font::MONOSPACE)
                .size(TEXT_SIZE)
                .wrapping(text::Wrapping::None)
                .width(width)
                .height(Length::Shrink)
                .padding(DOCUMENT_PADDING)
                .style(theme::plain_editor);

            scrollable(container(page).padding(Padding::default().right(DOCUMENT_CLEARANCE).bottom(DOCUMENT_CLEARANCE)))
                .id(self.scroll_id.clone())
                .direction(both_ways())
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        })
            .into()
    }
}
