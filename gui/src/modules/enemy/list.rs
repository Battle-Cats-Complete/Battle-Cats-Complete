use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use iced::futures::channel::mpsc::UnboundedReceiver;
use iced::widget::image::Handle;
use iced::widget::{button, column, container, image as iced_image, responsive, row, scrollable, space, text, tooltip, Column, Id};
use iced::{Border, Color, Element, Length, Size, Task, Theme};
use image::{imageops, Pixel, RgbaImage};
use tracing::warn;

use core::common::assets;
use core::modules::enemy::filter::evaluation::entity_passes_filter;
use core::modules::enemy::filter::EnemyFilterState;
use core::modules::enemy::scanner::EnemyEntry;

use crate::common::icon_loader::{self, Dispatcher, LoadRequest, LoadResult};
use crate::common::row_window::{self, RowWindow};

const ENEMY_ICON_SCALE_FACTOR: f32 = 2.6;
const ENEMY_ICON_OFFSET_X: i64 = 8;
const ENEMY_SHADOW_MARGIN: u32 = 8;

#[derive(Clone)]
pub enum Message {
    IconLoaded(LoadResult),
    SelectEnemy(u32),
    Scrolled(f32),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IconLoaded(result) => write!(f, "IconLoaded({})", result.id),
            Self::SelectEnemy(id) => write!(f, "SelectEnemy({})", id),
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
    last_filter_state: EnemyFilterState,
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

        let (loader, rx_result) = icon_loader::spawn(composite_icon);

        Self {
            texture_cache: HashMap::new(),
            placeholder,
            background,
            pending_requests: HashSet::new(),
            missing_ids: HashSet::new(),
            last_search_query: String::new(),
            last_unit_count: usize::MAX,
            last_filter_state: EnemyFilterState::default(),
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
    Id::new("enemy-banner-list")
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

    pub fn refresh(&mut self, entries: &[EnemyEntry], query: &str, filter_state: &EnemyFilterState) {
        if query == self.last_search_query && entries.len() == self.last_unit_count && filter_state == &self.last_filter_state {
            return;
        }

        self.last_search_query = query.to_string();
        self.last_unit_count = entries.len();
        self.last_filter_state = filter_state.clone();
        self.cached_indices.clear();

        let query_lower = query.to_lowercase();
        let is_empty = query_lower.is_empty();
        let is_id_search = query_lower.chars().next().is_some_and(|c| c.is_ascii_digit());

        for (index, entry) in entries.iter().enumerate() {
            if !entity_passes_filter(entry, filter_state) {
                continue;
            }

            let full_id = entry.id_str().to_lowercase();

            let matches = is_empty
                || (is_id_search && full_id.contains(&query_lower))
                || entry.name.to_lowercase().contains(&query_lower);

            if matches {
                self.cached_indices.push(index);
            }
        }

        self.dispatch_requests(entries);
    }

    fn dispatch_requests(&mut self, entries: &[EnemyEntry]) {
        let Some(background) = self.background.clone() else { return; };

        let ranked = self.cached_indices.iter().filter_map(|&index| entries.get(index).map(|entry| entry.id)).collect();
        self.loader.set_rank(ranked);

        self.last_focus_row = (self.scroll_offset / row_window::ROW_PITCH) as usize;
        self.loader.set_focus(self.last_focus_row);

        for &index in &self.cached_indices {
            let Some(entry) = entries.get(index) else { continue; };
            let id = entry.id;

            if self.texture_cache.contains_key(&id) || self.missing_ids.contains(&id) || self.pending_requests.contains(&id) {
                continue;
            }

            let Some(path) = entry.icon_path.clone() else {
                self.missing_ids.insert(id);
                continue;
            };

            self.pending_requests.insert(id);

            self.loader.request(LoadRequest { id, path, background: background.clone(), generation: self.generation });
        }
    }

    pub fn view<'a>(&'a self, entries: &'a [EnemyEntry], selected_id: Option<u32>) -> Element<'a, Message> {
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
                let Some(entry) = entries.get(index) else { continue; };
                list_col = list_col.push(self.view_row(entry, selected_id == Some(entry.id)));
            }

            if pad_after > 0.0 {
                list_col = list_col.push(space().height(Length::Fixed(pad_after)));
            }

            scrollable(list_col)
                .id(scrollable_id())
                .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
                .height(Length::Fill)
                .width(Length::Fill)
                .into()
        })
            .into()
    }

    fn view_row<'a>(&'a self, entry: &'a EnemyEntry, is_selected: bool) -> Element<'a, Message> {
        let handle = self.texture_cache.get(&entry.id).cloned().unwrap_or_else(|| self.placeholder.clone());
        let icon = iced_image(handle).height(Length::Fixed(row_window::ROW_HEIGHT));

        let icon_button = button(row![icon].width(Length::Fill))
            .on_press(Message::SelectEnemy(entry.id))
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

        tooltip(icon_button, self.view_tooltip(entry), tooltip::Position::Right).into()
    }

    fn view_tooltip<'a>(&self, entry: &EnemyEntry) -> Element<'a, Message> {
        let content = column![
            row![text("[ID]").size(11), text(entry.id_str())].spacing(4),
            row![text("[Name]").size(11), text(entry.display_name())].spacing(4),
        ].spacing(2);

        container(content).padding(8).style(container::bordered_box).into()
    }
}

fn composite_icon(path: &PathBuf, background: &RgbaImage) -> Option<(u32, u32, Vec<u8>)> {
    if !path.exists() {
        return None;
    }

    let Ok(opened) = image::open(path) else { return None; };
    let mut unit_img = opened.to_rgba8();

    let scaled_w = (unit_img.width() as f32 * ENEMY_ICON_SCALE_FACTOR).round() as u32;
    let scaled_h = (unit_img.height() as f32 * ENEMY_ICON_SCALE_FACTOR).round() as u32;
    unit_img = imageops::resize(&unit_img, scaled_w, scaled_h, imageops::FilterType::Lanczos3);

    let mut final_image = background.clone();
    let bg_w = final_image.width() as i64;
    let bg_h = final_image.height() as i64;
    let h = unit_img.height();

    let shadow_cutoff = h.saturating_sub(ENEMY_SHADOW_MARGIN);
    let mut min_y = h;
    let mut max_y = 0;
    let mut found_solid = false;

    for (_x, y, pixel) in unit_img.enumerate_pixels() {
        if y < shadow_cutoff && pixel[3] > 150 {
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            found_solid = true;
        }
    }

    let center_y = if found_solid { (min_y + max_y) as i64 / 2 } else { h as i64 / 2 };
    let offset_y = (bg_h / 2) - center_y;

    for (x, y, pixel) in unit_img.enumerate_pixels() {
        let dest_x = ENEMY_ICON_OFFSET_X + x as i64;
        let dest_y = offset_y + y as i64;

        if dest_x >= 0 && dest_x < bg_w && dest_y >= 0 && dest_y < bg_h {
            let bg_pixel = final_image.get_pixel_mut(dest_x as u32, dest_y as u32);
            let is_black_border = bg_pixel[0] < 25 && bg_pixel[1] < 25 && bg_pixel[2] < 25 && bg_pixel[3] > 200;
            if !is_black_border {
                bg_pixel.blend(pixel);
            }
        }
    }

    let target_h = 50;
    let ratio = target_h as f32 / final_image.height() as f32;
    let target_w = (final_image.width() as f32 * ratio) as u32;
    let resized = imageops::resize(&final_image, target_w, target_h, imageops::FilterType::Lanczos3);

    Some((resized.width(), resized.height(), resized.into_raw()))
}
