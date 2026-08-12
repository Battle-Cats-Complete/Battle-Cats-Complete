use std::path::{Path, PathBuf};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{container, operation, scrollable, space, text};
use iced::{widget, Element, Font, Length, Task};

use core::common::preview::{self, Preview};
use core::Vfs;

use crate::app::theme;
use crate::widget::smooth_scroll;

use super::picture::{self, Message};
use super::{both_ways, EMPTY_TEXT_SIZE, TEXT_SIZE};

const OVERSIZED_LABEL: &str = "File Too Large to Preview";
const BINARY_LABEL: &str = "No Preview for Binary Files";

enum Content {
    Empty,
    Image(picture::Source),
    Text(String),
    Notice(&'static str),
}

pub(super) struct State {
    loaded: Option<PathBuf>,
    framed: Option<PathBuf>,
    content: Content,
    picture: picture::State,
    scroll_id: widget::Id,
}

impl Default for State {
    fn default() -> Self {
        Self {
            loaded: None,
            framed: None,
            content: Content::Empty,
            picture: picture::State::default(),
            scroll_id: widget::Id::unique(),
        }
    }
}

impl State {
    pub(super) fn invalidate(&mut self) {
        self.loaded = None;
    }

    pub(super) fn update(&mut self, message: Message) {
        self.picture.update(message);
    }

    pub(super) fn recenter(&mut self) {
        self.picture.reset();
    }

    pub(super) fn snap_to_top<M: Send + 'static>(&self) -> Task<M> {
        operation::scroll_to(self.scroll_id.clone(), scrollable::AbsoluteOffset { x: 0.0, y: 0.0 })
    }

    pub(super) fn refresh(&mut self, vfs: &Vfs, mount: Option<&str>, selected: Option<&Path>) {
        if self.loaded.as_deref() == selected {
            return;
        }

        self.loaded = selected.map(Path::to_path_buf);

        let Some(relative) = selected else {
            self.content = Content::Empty;
            return;
        };

        let resolved = mount.and_then(|mount| vfs.root(mount)).map(|root| root.join(relative));

        let Some(path) = resolved else {
            return;
        };

        match preview::load(&path) {
            Preview::Image { bytes, width, height } => {
                if self.framed.as_deref() != Some(relative) {
                    self.framed = Some(relative.to_path_buf());
                    self.picture.reset();
                }

                self.content = Content::Image(picture::Source::new(bytes, width, height));
            }
            Preview::Text(text) => self.content = Content::Text(text),
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
            Content::Text(body) => self.view_document(body),
            Content::Image(source) => self.picture.view(source),
        }
    }

    fn view_document<'a, M: 'a>(&self, body: &'a str) -> Element<'a, M> {
        let page = text(body).font(Font::MONOSPACE).size(TEXT_SIZE).wrapping(text::Wrapping::None);

        smooth_scroll(
            scrollable(page)
                .id(self.scroll_id.clone())
                .direction(both_ways())
                .width(Length::Fill)
                .height(Length::Fill),
        )
            .into()
    }

}
