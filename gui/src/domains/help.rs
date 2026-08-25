use iced::widget::{column, container, markdown, operation, row, scrollable, text, Id};
use iced::{Element, Length, Padding, Task, Theme};
use tracing::warn;

use crate::app::state::HelpState;
use crate::app::theme;
use crate::widget::{list_row, smooth_scroll};

include!(concat!(env!("OUT_DIR"), "/help_pages.rs"));

const CONTENT_TEXT_SIZE: f32 = 16.0;
const SCROLLBAR_GAP: f32 = 8.0;

#[derive(Debug, Clone)]
pub enum Message {
    PageSelected(usize),
    Scrolled(f32),
    OpenUrl(String),
}

struct Page {
    label: String,
    items: Vec<markdown::Item>,
}

pub struct State {
    pub active_page: usize,
    scroll_offset: f32,
    pages: Vec<Page>,
}

impl Default for State {
    fn default() -> Self {
        let mut ordered: Vec<(Option<u32>, &str, &str)> = HELP_PAGES
            .iter()
            .map(|(stem, content)| {
                let (order, name) = strip_order_prefix(stem);
                (order, name, *content)
            })
            .collect();

        ordered.sort_by(|(order_a, name_a, _), (order_b, name_b, _)| match (order_a, order_b) {
            (Some(a), Some(b)) => a.cmp(b).then_with(|| name_a.cmp(name_b)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => name_a.cmp(name_b),
        });

        let pages = ordered
            .into_iter()
            .map(|(_, name, content)| Page { label: display_name(name), items: crate::common::markdown::parse(content) })
            .collect();

        Self { active_page: 0, scroll_offset: 0.0, pages }
    }
}

impl State {
    pub(crate) fn content_scrollable_id() -> Id {
        Id::new("help-content")
    }

    pub(crate) fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    pub(crate) fn restore_state(&mut self, state: &HelpState) {
        self.active_page = state.active_page.min(self.pages.len().saturating_sub(1));
        self.scroll_offset = state.scroll_offset;
    }

    pub(crate) fn sync_state(&self, state: &mut HelpState) {
        state.active_page = self.active_page;
        state.scroll_offset = self.scroll_offset;
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PageSelected(index) => {
                self.active_page = index;
                self.scroll_offset = 0.0;
                operation::scroll_to(Self::content_scrollable_id(), scrollable::AbsoluteOffset { x: 0.0, y: 0.0 })
            }
            Message::Scrolled(offset) => {
                self.scroll_offset = offset;
                Task::none()
            }
            Message::OpenUrl(url) => {
                if let Err(err) = open::that(&url) {
                    warn!("Failed to open URL {}: {}", url, err);
                }
                Task::none()
            }
        }
    }

    pub fn view<'a>(&'a self, ui_theme: &Theme) -> Element<'a, Message> {
        row![self.view_sidebar(), self.view_content(ui_theme)].height(Length::Fill).into()
    }

    fn view_sidebar<'a>(&'a self) -> Element<'a, Message> {
        const SIDEBAR_WIDTH: f32 = 110.0;

        let mut page_list = column![].spacing(4);

        for (index, page) in self.pages.iter().enumerate() {
            let is_active = self.active_page == index;
            let row_content = container(theme::button_label(&page.label).size(14)).padding([8, 12]).width(Length::Fill);

            page_list = page_list.push(list_row(row_content, is_active, true, Length::Fill, Message::PageSelected(index)));
        }

        container(smooth_scroll(scrollable(page_list).width(Length::Fill).height(Length::Fill)))
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill)
            .padding(8)
            .style(theme::list_panel_container)
            .into()
    }

    fn view_content<'a>(&'a self, ui_theme: &Theme) -> Element<'a, Message> {
        let Some(page) = self.pages.get(self.active_page) else {
            return container(text("No help pages found")).center_x(Length::Fill).center_y(Length::Fill).into();
        };

        let body = container(markdown::view(&page.items, markdown::Settings::with_text_size(CONTENT_TEXT_SIZE, theme::markdown_style(ui_theme))).map(Message::OpenUrl))
            .padding(Padding { top: 15.0, right: 0.0, bottom: 15.0, left: 15.0 });

        container(
            smooth_scroll(
                scrollable(body)
                    .id(Self::content_scrollable_id())
                    .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .spacing(SCROLLBAR_GAP),
            )
        )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn strip_order_prefix(stem: &str) -> (Option<u32>, &str) {
    let digits_end = stem.find(|c: char| !c.is_ascii_digit()).unwrap_or(stem.len());
    if digits_end == 0 {
        return (None, stem);
    }

    let Some(rest) = stem[digits_end..].strip_prefix('-') else {
        return (None, stem);
    };

    match stem[..digits_end].parse::<u32>() {
        Ok(order) => (Some(order), rest),
        Err(_) => (None, stem),
    }
}

fn display_name(stem: &str) -> String {
    stem.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
