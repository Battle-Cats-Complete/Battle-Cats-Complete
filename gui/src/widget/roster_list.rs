use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;

use iced::futures::channel::mpsc::UnboundedReceiver;
use iced::widget::image::Handle;
use iced::widget::{responsive, scrollable, space, Column, Id};
use iced::{Element, Length, Size, Task};
use image::RgbaImage;
use tracing::{info, warn};

use core::common::assets;

use crate::common::row_window::{self, RowWindow};
use crate::common::udi_loader::{self, Composite, Dispatcher, LoadRequest, LoadResult};
use crate::widget::{roster_row, smooth_scroll};

const BANNER_ASPECT: f32 = 318.0 / 133.0;
const SCROLLBAR_WIDTH: f32 = 16.0;
pub(crate) const LIST_WIDTH: f32 = row_window::ROW_HEIGHT * BANNER_ASPECT + SCROLLBAR_WIDTH;

pub(crate) trait Roster {
    type Entry;
    type Filter: Clone + Default + PartialEq;

    const SCROLLABLE_ID: &'static str;
    const LABEL: &'static str;
    const COMPOSITE: Composite;

    fn id(entry: &Self::Entry) -> u32;

    fn image_path(entry: &Self::Entry) -> Option<PathBuf>;

    fn passes_filter(entry: &Self::Entry, filter: &Self::Filter) -> bool;

    fn matches_query(entry: &Self::Entry, query: &str) -> bool;

    fn tooltip<'a>(entry: &Self::Entry) -> Element<'a, Message>;
}

#[derive(Clone)]
pub enum Message {
    IconLoaded(LoadResult),
    Select(u32),
    Scrolled(f32),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IconLoaded(result) => write!(f, "IconLoaded({})", result.id),
            Self::Select(id) => write!(f, "Select({})", id),
            Self::Scrolled(offset) => write!(f, "Scrolled({})", offset),
        }
    }
}

pub(crate) struct State<R: Roster> {
    texture_cache: HashMap<u32, Handle>,
    placeholder: Handle,
    background: Option<Arc<RgbaImage>>,
    pending_requests: HashSet<u32>,
    missing_ids: HashSet<u32>,
    last_search_query: String,
    last_unit_count: usize,
    last_filter_state: R::Filter,
    cached_indices: Vec<usize>,
    scroll_offset: f32,
    last_focus_row: usize,
    generation: u64,
    loader: Dispatcher,
    rx_result: Option<UnboundedReceiver<LoadResult>>,
    roster: PhantomData<R>,
}

impl<R: Roster> Default for State<R> {
    fn default() -> Self {
        let background = image::load_from_memory(assets::UDI_F).ok().map(|img| Arc::new(img.to_rgba8()));

        let placeholder = match &background {
            Some(bg) => Handle::from_rgba(bg.width(), bg.height(), bg.as_raw().clone()),
            None => {
                warn!("Failed to decode embedded banner background asset");
                Handle::from_rgba(1, 1, vec![80, 80, 80, 255])
            }
        };

        let (loader, rx_result) = udi_loader::spawn(R::COMPOSITE);

        Self {
            texture_cache: HashMap::new(),
            placeholder,
            background,
            pending_requests: HashSet::new(),
            missing_ids: HashSet::new(),
            last_search_query: String::new(),
            last_unit_count: usize::MAX,
            last_filter_state: R::Filter::default(),
            cached_indices: Vec::new(),
            scroll_offset: 0.0,
            last_focus_row: 0,
            generation: 0,
            loader,
            rx_result: Some(rx_result),
            roster: PhantomData,
        }
    }
}

impl<R: Roster> State<R> {
    pub(crate) fn scrollable_id() -> Id {
        Id::new(R::SCROLLABLE_ID)
    }

    pub(crate) fn scroll_offset(&self) -> f32 {
        self.scroll_offset
    }

    pub(crate) fn set_scroll_offset(&mut self, offset: f32) {
        self.scroll_offset = offset;
    }

    pub(crate) fn result_stream(&mut self) -> Task<Message> {
        self.rx_result.take().map_or_else(Task::none, |rx| Task::stream(rx).map(Message::IconLoaded))
    }

    pub(crate) fn update(&mut self, message: Message) {
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

    pub(crate) fn invalidate(&mut self) {
        self.generation += 1;
        self.texture_cache.clear();
        self.missing_ids.clear();
        self.pending_requests.clear();
        self.last_unit_count = usize::MAX;
    }

    pub(crate) fn refresh(&mut self, entries: &[R::Entry], query: &str, filter_state: &R::Filter) {
        if query == self.last_search_query && entries.len() == self.last_unit_count && filter_state == &self.last_filter_state {
            return;
        }

        self.last_search_query = query.to_string();
        self.last_unit_count = entries.len();
        self.last_filter_state = filter_state.clone();
        self.cached_indices.clear();

        let query_lower = query.to_lowercase();

        for (index, entry) in entries.iter().enumerate() {
            if !R::passes_filter(entry, filter_state) {
                continue;
            }

            if query_lower.is_empty() || R::matches_query(entry, &query_lower) {
                self.cached_indices.push(index);
            }
        }

        info!("Visible {}: {} (of {} total)", R::LABEL, self.cached_indices.len(), entries.len());

        self.dispatch_requests(entries);
    }

    fn dispatch_requests(&mut self, entries: &[R::Entry]) {
        let Some(background) = self.background.clone() else { return; };

        let ranked = self.cached_indices.iter().filter_map(|&index| entries.get(index).map(R::id)).collect();
        self.loader.set_rank(ranked);

        self.last_focus_row = (self.scroll_offset / row_window::ROW_PITCH) as usize;
        self.loader.set_focus(self.last_focus_row);

        for &index in &self.cached_indices {
            let Some(entry) = entries.get(index) else { continue; };
            let id = R::id(entry);

            if self.texture_cache.contains_key(&id) || self.missing_ids.contains(&id) || self.pending_requests.contains(&id) {
                continue;
            }

            let Some(path) = R::image_path(entry) else {
                self.missing_ids.insert(id);
                continue;
            };

            self.pending_requests.insert(id);

            self.loader.request(LoadRequest { id, path, background: background.clone(), generation: self.generation });
        }
    }

    pub(crate) fn view<'a>(&'a self, entries: &'a [R::Entry], selected_id: Option<u32>) -> Element<'a, Message> {
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
                list_col = list_col.push(self.view_row(entry, selected_id == Some(R::id(entry))));
            }

            if pad_after > 0.0 {
                list_col = list_col.push(space().height(Length::Fixed(pad_after)));
            }

            smooth_scroll(
                scrollable(list_col)
                    .id(Self::scrollable_id())
                    .on_scroll(|viewport| Message::Scrolled(viewport.absolute_offset().y))
                    .height(Length::Fill)
                    .width(Length::Fill),
            )
            .into()
        })
            .into()
    }

    fn view_row<'a>(&'a self, entry: &'a R::Entry, is_selected: bool) -> Element<'a, Message> {
        let id = R::id(entry);
        let handle = self.texture_cache.get(&id).cloned().unwrap_or_else(|| self.placeholder.clone());

        roster_row(handle, is_selected, Message::Select(id), R::tooltip(entry))
    }
}
