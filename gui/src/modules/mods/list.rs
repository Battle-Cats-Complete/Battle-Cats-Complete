use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;

use iced::alignment::Vertical;
use iced::futures::channel::mpsc::{unbounded, UnboundedReceiver};
use iced::widget::image::Handle;
use iced::widget::{container, image as iced_image, row, scrollable, space, text, Column};
use iced::{Element, Length, Task};
use image::imageops;

use core::common::gfx;
use core::modules::mods::{self, ModData};

use crate::app::theme;
use crate::widget::{list_row, smooth_scroll};

use super::SCROLLBAR_GAP;

const ICON_BOX: f32 = 32.0;
const ICON_PADDING: f32 = 4.0;
const ROW_HEIGHT: f32 = ICON_BOX + ICON_PADDING * 2.0;
const ROW_SPACING: f32 = 4.0;
const ROW_TEXT_SIZE: f32 = 12.0;

const ACTIVE_MARKER_WIDTH: f32 = 4.0;
const ICON_RENDER_SIZE: u32 = 64;
const ICON_FILE: &str = "icon.png";

#[derive(Clone)]
pub enum Message {
    IconLoaded(LoadResult),
    SelectMod(String),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IconLoaded(result) => write!(f, "IconLoaded({})", result.folder_name),
            Self::SelectMod(folder) => write!(f, "SelectMod({})", folder),
        }
    }
}

struct LoadRequest {
    folder_name: String,
    path: PathBuf,
    generation: u64,
}

#[derive(Clone)]
pub struct LoadResult {
    folder_name: String,
    generation: u64,
    payload: Option<(u32, u32, Vec<u8>)>,
}

pub struct State {
    texture_cache: HashMap<String, Handle>,
    pending_requests: HashMap<String, u64>,
    missing_ids: HashSet<String>,
    last_search_query: String,
    last_mod_count: usize,
    cached_indices: Vec<usize>,
    generation: u64,
    tx_request: Sender<LoadRequest>,
    rx_result: Option<UnboundedReceiver<LoadResult>>,
}

impl Default for State {
    fn default() -> Self {
        let (tx_request, rx_request) = mpsc::channel::<LoadRequest>();
        let (tx_result, rx_result) = unbounded::<LoadResult>();

        thread::spawn(move || {
            while let Ok(request) = rx_request.recv() {
                let tx = tx_result.clone();
                rayon::spawn(move || {
                    let payload = process_icon(&request.path);
                    let _ = tx.unbounded_send(LoadResult {
                        folder_name: request.folder_name,
                        generation: request.generation,
                        payload,
                    });
                });
            }
        });

        Self {
            texture_cache: HashMap::new(),
            pending_requests: HashMap::new(),
            missing_ids: HashSet::new(),
            last_search_query: String::new(),
            last_mod_count: usize::MAX,
            cached_indices: Vec::new(),
            generation: 0,
            tx_request,
            rx_result: Some(rx_result),
        }
    }
}

impl State {
    pub fn result_stream(&mut self) -> Task<Message> {
        self.rx_result.take().map_or_else(Task::none, |rx| Task::stream(rx).map(Message::IconLoaded))
    }

    pub fn update(&mut self, message: Message) {
        let Message::IconLoaded(result) = message else { return };

        if self.pending_requests.get(&result.folder_name) != Some(&result.generation) {
            return;
        }

        self.pending_requests.remove(&result.folder_name);
        match result.payload {
            Some((width, height, pixels)) => {
                self.texture_cache.insert(result.folder_name.clone(), Handle::from_rgba(width, height, pixels));
                self.missing_ids.remove(&result.folder_name);
            }
            None => {
                self.texture_cache.remove(&result.folder_name);
                self.missing_ids.insert(result.folder_name);
            }
        }
    }

    pub(super) fn invalidate(&mut self, folders: &HashSet<String>, mods: &[ModData]) {
        for folder in folders {
            self.texture_cache.remove(folder);
            self.missing_ids.remove(folder);
            self.pending_requests.remove(folder);
        }

        self.dispatch_requests(mods);
    }

    pub fn refresh(&mut self, mods: &[ModData], query: &str) {
        let query_lower = query.to_lowercase();
        if query_lower == self.last_search_query && mods.len() == self.last_mod_count {
            return;
        }

        self.last_search_query = query_lower.clone();
        self.last_mod_count = mods.len();
        self.cached_indices.clear();

        for (index, m) in mods.iter().enumerate() {
            if query_lower.is_empty() || m.folder_name.to_lowercase().contains(&query_lower) {
                self.cached_indices.push(index);
            }
        }

        self.dispatch_requests(mods);
    }

    fn dispatch_requests(&mut self, mods: &[ModData]) {
        for &index in &self.cached_indices {
            let Some(mod_data) = mods.get(index) else { continue; };
            let name = &mod_data.folder_name;

            if self.texture_cache.contains_key(name) || self.missing_ids.contains(name) || self.pending_requests.contains_key(name) {
                continue;
            }

            let path = Path::new("mods").join(name);
            self.generation += 1;
            self.pending_requests.insert(name.clone(), self.generation);

            let _ = self.tx_request.send(LoadRequest {
                folder_name: name.clone(),
                path,
                generation: self.generation,
            });
        }
    }

    pub fn view<'a>(&'a self, mods: &'a [ModData], selected: Option<&str>) -> Element<'a, Message> {
        if self.cached_indices.is_empty() {
            return container(theme::centered_text("No Mods Found").size(13).style(text::danger))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }

        let mut list_col = Column::with_capacity(self.cached_indices.len())
            .spacing(ROW_SPACING)
            .width(Length::Fill);

        for &index in &self.cached_indices {
            let Some(mod_data) = mods.get(index) else { continue; };
            list_col = list_col.push(self.view_row(mod_data, selected == Some(mod_data.folder_name.as_str())));
        }

        smooth_scroll(
            scrollable(list_col)
                .spacing(SCROLLBAR_GAP)
                .height(Length::Fill)
                .width(Length::Fill),
        )
            .into()
    }

    fn view_row<'a>(&'a self, mod_data: &'a ModData, is_selected: bool) -> Element<'a, Message> {
        let folder_name = mod_data.folder_name.as_str();

        let marker: Element<'a, Message> = if mod_data.enabled {
            container(space())
                .width(Length::Fixed(ACTIVE_MARKER_WIDTH))
                .height(Length::Fill)
                .style(theme::accent_marker)
                .into()
        } else {
            space().width(Length::Fixed(ACTIVE_MARKER_WIDTH)).into()
        };

        let mut face = row![marker].align_y(Vertical::Center).height(Length::Fill);

        if let Some(handle) = self.texture_cache.get(folder_name) {
            face = face.push(
                container(iced_image(handle.clone()).width(ICON_BOX).height(ICON_BOX)).padding(ICON_PADDING),
            );
        }

        face = face
            .push(theme::centered_text(folder_name).size(ROW_TEXT_SIZE).width(Length::Fill))
            .push(space().width(Length::Fixed(ACTIVE_MARKER_WIDTH)));

        list_row(
            container(face).width(Length::Fill).height(Length::Fixed(ROW_HEIGHT)),
            is_selected,
            true,
            Length::Fill,
            Message::SelectMod(folder_name.to_string()),
        )
    }
}

fn process_icon(mod_path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let icon_path = mods::locate(mod_path, ICON_FILE)?;
    let opened = image::open(&icon_path).ok()?;

    let cropped = gfx::autocrop(opened.to_rgba8());
    let (width, height) = cropped.dimensions();

    let longest = width.max(height);
    if longest == 0 {
        return None;
    }

    let scale = ICON_RENDER_SIZE as f32 / longest as f32;
    let target_w = ((width as f32 * scale).round() as u32).max(1);
    let target_h = ((height as f32 * scale).round() as u32).max(1);

    let resized = imageops::resize(&cropped, target_w, target_h, imageops::FilterType::Lanczos3);

    Some((resized.width(), resized.height(), resized.into_raw()))
}
