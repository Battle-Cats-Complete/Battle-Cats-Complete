use std::path::{Path, PathBuf};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{container, image, operation, responsive, scrollable, space, text};
use iced::{widget, ContentFit, Element, Font, Length, Size, Task};

use core::common::preview::{self, Preview};
use core::Vfs;

use crate::app::theme;
use crate::widget::smooth_scroll;

use super::{both_ways, EMPTY_TEXT_SIZE, SCROLLBAR_ALLOWANCE, TEXT_SIZE};

const OVERSIZED_LABEL: &str = "File Too Large to Preview";
const BINARY_LABEL: &str = "No Preview for Binary Files";

enum Content {
    Empty,
    Image { handle: image::Handle, width: u32, height: u32 },
    Text(String),
    Notice(&'static str),
}

pub(super) struct State {
    loaded: Option<PathBuf>,
    content: Content,
    scroll_id: widget::Id,
}

impl Default for State {
    fn default() -> Self {
        Self { loaded: None, content: Content::Empty, scroll_id: widget::Id::unique() }
    }
}

impl State {
    pub(super) fn invalidate(&mut self) {
        self.loaded = None;
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
                self.content = Content::Image { handle: image::Handle::from_bytes(bytes), width, height };
            }
            Preview::Text(text) => self.content = Content::Text(text),
            Preview::Oversized => self.content = Content::Notice(OVERSIZED_LABEL),
            Preview::Binary => self.content = Content::Notice(BINARY_LABEL),
            Preview::Unavailable => {}
        }
    }

    pub(super) fn view<M: 'static>(&self) -> Element<'_, M> {
        match &self.content {
            Content::Empty => space().into(),
            Content::Notice(label) => container(theme::centered_text(*label).size(EMPTY_TEXT_SIZE))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .into(),
            Content::Text(body) => self.view_document(body),
            Content::Image { handle, width, height } => self.view_picture(handle, *width, *height),
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

    fn view_picture<'a, M: 'a>(&self, handle: &image::Handle, width: u32, height: u32) -> Element<'a, M> {
        let handle = handle.clone();
        let id = self.scroll_id.clone();

        responsive(move |size: Size| {
            let frame_width = (width as f32).max(size.width - SCROLLBAR_ALLOWANCE);
            let frame_height = (height as f32).max(size.height - SCROLLBAR_ALLOWANCE);

            let framed = container(image(handle.clone()).content_fit(ContentFit::None))
                .width(Length::Fixed(frame_width))
                .height(Length::Fixed(frame_height))
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            smooth_scroll(
                scrollable(framed)
                    .id(id.clone())
                    .direction(both_ways())
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
                .into()
        })
            .into()
    }
}
