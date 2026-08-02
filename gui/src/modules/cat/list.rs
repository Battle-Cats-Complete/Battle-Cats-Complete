use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use iced::futures::channel::mpsc::UnboundedReceiver;
use iced::widget::image::Handle;
use iced::widget::{button, column, container, image as iced_image, responsive, row, scrollable, space, text, tooltip, Column, Id};
use iced::{Border, Color, Element, Length, Size, Task, Theme};
use image::{imageops, RgbaImage};
use tracing::{info, warn};

use core::common::{assets, gfx};
use core::modules::cat::filter::{evaluation, CatFilterState};
use core::modules::cat::scanner::CatEntry;

use crate::common::icon_loader::{self, Dispatcher, LoadRequest, LoadResult};
use crate::common::row_window::{self, RowWindow};
use crate::common::smooth_scroll::smooth_scroll;
const BANNER_ASPECT: f32 = 318.0 / 133.0;
const SCROLLBAR_WIDTH: f32 = 16.0;
pub(super) const LIST_WIDTH: f32 = row_window::ROW_HEIGHT * BANNER_ASPECT + SCROLLBAR_WIDTH;

#[derive(Clone)]
pub enum Message {
    IconLoaded(LoadResult),
    SelectCat(u32),
    Scrolled(f32),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IconLoaded(result) => write!(f, "IconLoaded({})", result.id),
            Self::SelectCat(id) => write!(f, "SelectCat({})", id),
            Self::Scrolled(offset) => write!(f, "Scrolled({})", offset),
        }
    }
}

pub struct State {
    texture_cache: HashMap<u32, Handle>,
    placeholder: Handle,
    background: Option<Arc<RgbaImage>>,
    pending_requests: HashSet<u32>,
    missing_ids: HashSet<u32>,
    last_search_query: String,
    last_unit_count: usize,
    last_filter_state: CatFilterState,
    cached_indices: Vec<usize>,
    scroll_offset: f32,
    last_focus_row: usize,
    generation: u64,
    loader: Dispatcher,
    rx_result: Option<UnboundedReceiver<LoadResult>>,
}

impl Default for State {
    fn default() -> Self {
        let background = image::load_from_memory(assets::UDI_F).ok().map(|img| Arc::new(img.to_rgba8()));

        let placeholder = match &background {
            Some(bg) => Handle::from_rgba(bg.width(), bg.height(), bg.as_raw().clone()),
            None => {
                warn!("Failed to decode embedded banner background asset");
                Handle::from_rgba(1, 1, vec![80, 80, 80, 255])
            }
        };

        let (loader, rx_result) = icon_loader::spawn(composite_banner);

        Self {
            texture_cache: HashMap::new(),
            placeholder,
            background,
            pending_requests: HashSet::new(),
            missing_ids: HashSet::new(),
            last_search_query: String::new(),
            last_unit_count: usize::MAX,
            last_filter_state: CatFilterState::default(),
            cached_indices: Vec::new(),
            scroll_offset: 0.0,
            last_focus_row: 0,
            generation: 0,
            loader,
            rx_result: Some(rx_result),
        }
    }
}

pub(super) fn scrollable_id() -> Id {
    Id::new("cat-banner-list")
}

impl State {
    pub(super) fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    pub(super) fn set_scroll_offset(&mut self, offset: f32) {
        self.scroll_offset = offset;
    }

    pub fn result_stream(&mut self) -> Task<Message> {
        match self.rx_result.take() {
            Some(rx) => Task::stream(rx).map(Message::IconLoaded),
            None => Task::none(),
        }
    }

    pub fn update(&mut self, message: Message) {
        if let Message::Scrolled(offset) = message {
            self.scroll_offset = offset;

            let row = (offset / row_window::ROW_PITCH) as usize;
            if row != self.last_focus_row {
                self.last_focus_row = row;
                self.loader.set_focus(row);
            }
            return;
        }

        let Message::IconLoaded(result) = message else { return };

        if result.generation != self.generation {
            return;
        }

        self.pending_requests.remove(&result.id);
        match result.payload {
            Some((width, height, pixels)) => {
                self.texture_cache.insert(result.id, Handle::from_rgba(width, height, pixels));
            }
            None => {
                self.missing_ids.insert(result.id);
            }
        }
    }

    pub fn invalidate(&mut self) {
        self.generation += 1;
        self.texture_cache.clear();
        self.missing_ids.clear();
        self.pending_requests.clear();
        self.last_unit_count = usize::MAX;
    }

    pub fn refresh(&mut self, cats: &[CatEntry], query: &str, filter_state: &CatFilterState) {
        if query == self.last_search_query && cats.len() == self.last_unit_count && filter_state == &self.last_filter_state {
            return;
        }

        self.last_search_query = query.to_string();
        self.last_unit_count = cats.len();
        self.last_filter_state = filter_state.clone();
        self.cached_indices.clear();

        let query_lower = query.to_lowercase();

        for (index, cat) in cats.iter().enumerate() {
            if !evaluation::entity_passes_filter(cat, filter_state) {
                continue;
            }

            if query_lower.is_empty() || cat.base_id_str().contains(&query_lower) {
                self.cached_indices.push(index);
                continue;
            }

            if cat.names.iter().flatten().any(|name| name.to_lowercase().contains(&query_lower)) {
                self.cached_indices.push(index);
            }
        }

        info!("Visible cats: {} (of {} total)", self.cached_indices.len(), cats.len());

        self.dispatch_requests(cats);
    }

    fn dispatch_requests(&mut self, cats: &[CatEntry]) {
        let Some(background) = self.background.clone() else { return; };

        let ranked = self.cached_indices.iter().filter_map(|&index| cats.get(index).map(|cat| cat.id)).collect();
        self.loader.set_rank(ranked);

        self.last_focus_row = (self.scroll_offset / row_window::ROW_PITCH) as usize;
        self.loader.set_focus(self.last_focus_row);

        for &index in &self.cached_indices {
            let Some(cat) = cats.get(index) else { continue; };
            let id = cat.id;

            if self.texture_cache.contains_key(&id) || self.missing_ids.contains(&id) || self.pending_requests.contains(&id) {
                continue;
            }

            let Some(path) = cat.image_path.clone() else {
                self.missing_ids.insert(id);
                continue;
            };

            self.pending_requests.insert(id);

            self.loader.request(LoadRequest { id, path, background: background.clone(), generation: self.generation });
        }
    }

    pub fn view<'a>(&'a self, cats: &'a [CatEntry], selected_id: Option<u32>) -> Element<'a, Message> {
        responsive(move |size: Size| {
            let RowWindow { range, pad_before, pad_after } =
                row_window::compute(self.cached_indices.len(), size.height, self.scroll_offset);

            let mut list_col = Column::with_capacity(range.len() + 2)
                .spacing(row_window::ROW_SPACING)
                .width(Length::Fill);

            if pad_before > 0.0 {
                list_col = list_col.push(space().height(Length::Fixed(pad_before)));
            }

            for &index in &self.cached_indices[range] {
                let Some(cat) = cats.get(index) else { continue; };
                list_col = list_col.push(self.view_row(cat, selected_id == Some(cat.id)));
            }

            if pad_after > 0.0 {
                list_col = list_col.push(space().height(Length::Fixed(pad_after)));
            }

            smooth_scroll(
                scrollable(list_col)
                    .id(scrollable_id())
                    .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
                    .height(Length::Fill)
                    .width(Length::Fill),
            )
        })
            .into()
    }

    fn view_row<'a>(&'a self, cat: &'a CatEntry, is_selected: bool) -> Element<'a, Message> {
        let handle = self.texture_cache.get(&cat.id).cloned().unwrap_or_else(|| self.placeholder.clone());
        let banner = iced_image(handle).height(Length::Fixed(row_window::ROW_HEIGHT));

        let banner_button = button(row![banner].width(Length::Fill))
            .on_press(Message::SelectCat(cat.id))
            .width(Length::Fill)
            .padding(0)
            .style(move |theme: &Theme, status| {
                let palette = theme.palette();
                let background = if is_selected {
                    palette.primary
                } else if status == button::Status::Hovered {
                    Color { a: 0.15, ..palette.text }
                } else {
                    Color::TRANSPARENT
                };

                button::Style {
                    background: Some(background.into()),
                    text_color: palette.text,
                    border: Border::default().rounded(4.0).width(if is_selected { 2.0 } else { 0.0 }).color(palette.primary),
                    ..Default::default()
                }
            });

        tooltip(banner_button, self.view_tooltip(cat), tooltip::Position::Right).into()
    }

    fn view_tooltip<'a>(&self, cat: &CatEntry) -> Element<'a, Message> {
        let mut content = column![
            row![text("[ID]").size(11), text(cat.base_id_str())].spacing(4)
        ].spacing(2);

        let labels = ["Normal", "Evolved", "True", "Ultra"];
        for (i, label) in labels.iter().enumerate() {
            if !cat.forms[i] { continue; }
            content = content.push(row![text(format!("[{}]", label)).size(11), text(cat.display_name(i))].spacing(4));
        }

        container(content).padding(8).style(container::bordered_box).into()
    }
}

fn composite_banner(path: &PathBuf, background: &RgbaImage) -> Option<(u32, u32, Vec<u8>)> {
    for _ in 0..3 {
        if !path.exists() {
            return None;
        }

        let Ok(opened) = image::open(path) else {
            thread::sleep(Duration::from_millis(50));
            continue;
        };

        let mut unit_img = opened.to_rgba8();
        let mut final_image = background.clone();
        let bg_w = final_image.width() as i64;
        let bg_h = final_image.height() as i64;
        let (w, h) = unit_img.dimensions();
        let is_transparent_unit = w > 311 && h > 2 && unit_img.get_pixel(311, 2)[3] == 0;

        let (x, y) = if is_transparent_unit {
            (-3, 9)
        } else {
            unit_img = gfx::autocrop(unit_img);
            let unit_w = unit_img.width() as i64;
            let unit_h = unit_img.height() as i64;
            ((bg_w - unit_w) / 2, (bg_h - unit_h) / 2)
        };

        imageops::overlay(&mut final_image, &unit_img, x, y);

        const TARGET_H: u32 = 100;
        let ratio = TARGET_H as f32 / final_image.height() as f32;
        let target_w = (final_image.width() as f32 * ratio) as u32;
        let resized = imageops::resize(&final_image, target_w, TARGET_H, imageops::FilterType::Lanczos3);

        return Some((resized.width(), resized.height(), resized.into_raw()));
    }
    None
}
