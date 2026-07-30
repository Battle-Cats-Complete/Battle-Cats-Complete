use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;

use iced::futures::channel::mpsc::{unbounded, UnboundedReceiver};
use iced::widget::image::Handle;
use iced::widget::{button, column, image as iced_image, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Task, Theme};
use image::imageops;

use core::modules::mods::ModData;

const ICON_UI_HEIGHT: f32 = 46.0;
const ICON_RENDER_SIZE: u32 = 92;

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
}

#[derive(Clone)]
pub struct LoadResult {
    folder_name: String,
    payload: Option<(u32, u32, Vec<u8>)>,
}

pub struct State {
    texture_cache: HashMap<String, Handle>,
    pending_requests: HashSet<String>,
    missing_ids: HashSet<String>,
    last_search_query: String,
    last_mod_count: usize,
    cached_indices: Vec<usize>,
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
                    let _ = tx.unbounded_send(LoadResult { folder_name: request.folder_name, payload });
                });
            }
        });

        Self {
            texture_cache: HashMap::new(),
            pending_requests: HashSet::new(),
            missing_ids: HashSet::new(),
            last_search_query: String::new(),
            last_mod_count: usize::MAX,
            cached_indices: Vec::new(),
            tx_request,
            rx_result: Some(rx_result),
        }
    }
}

impl State {
    pub fn result_stream(&mut self) -> Task<Message> {
        match self.rx_result.take() {
            Some(rx) => Task::stream(rx).map(Message::IconLoaded),
            None => Task::none(),
        }
    }

    pub fn update(&mut self, message: Message) {
        let Message::IconLoaded(result) = message else { return };

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

            if self.texture_cache.contains_key(name) || self.missing_ids.contains(name) || self.pending_requests.contains(name) {
                continue;
            }

            let path = Path::new("mods").join(name);
            self.pending_requests.insert(name.clone());

            let _ = self.tx_request.send(LoadRequest { folder_name: name.clone(), path });
        }
    }

    pub fn view<'a>(&'a self, mods: &'a [ModData], selected: Option<&str>) -> Element<'a, Message> {
        let mut list_col = column![].spacing(4).width(Length::Fill);

        for &index in &self.cached_indices {
            let Some(mod_data) = mods.get(index) else { continue; };
            list_col = list_col.push(self.view_row(mod_data, selected == Some(mod_data.folder_name.as_str())));
        }

        scrollable(list_col).height(Length::Fill).width(Length::Fill).into()
    }

    fn view_row<'a>(&'a self, mod_data: &'a ModData, is_selected: bool) -> Element<'a, Message> {
        let is_enabled = mod_data.enabled;
        let folder_name = mod_data.folder_name.as_str();

        let row_content: Element<'a, Message> = if let Some(handle) = self.texture_cache.get(folder_name) {
            row![
                iced_image(handle.clone()).height(Length::Fixed(ICON_UI_HEIGHT)),
                text(folder_name).size(14)
            ].spacing(12).align_y(Alignment::Center).into()
        } else {
            row![text(folder_name).size(14)].align_y(Alignment::Center).into()
        };

        button(row_content)
            .width(Length::Fill)
            .padding(8)
            .on_press(Message::SelectMod(folder_name.to_string()))
            .style(move |theme: &Theme, status| {
                let palette = theme.palette();
                let bg = if is_selected {
                    palette.primary
                } else if is_enabled {
                    palette.success
                } else if status == button::Status::Hovered {
                    Color { a: 0.1, ..palette.text }
                } else {
                    Color::TRANSPARENT
                };

                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: if is_selected || is_enabled { Color::WHITE } else { palette.text },
                    border: Border::default().rounded(4.0).width(if is_selected { 2.0 } else { 0.0 }).color(palette.primary),
                    ..Default::default()
                }
            })
            .into()
    }
}

fn process_icon(mod_path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let icon_path = mod_path.join("icons").join("icon.png");
    if !icon_path.exists() {
        return None;
    }

    let opened = image::open(&icon_path).ok()?;
    let resized = imageops::resize(&opened.to_rgba8(), ICON_RENDER_SIZE, ICON_RENDER_SIZE, imageops::FilterType::Lanczos3);

    Some((resized.width(), resized.height(), resized.into_raw()))
}
