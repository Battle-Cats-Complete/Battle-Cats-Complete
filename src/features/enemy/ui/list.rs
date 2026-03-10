use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use image::imageops;

use crate::features::enemy::logic::scanner::EnemyEntry;

struct LoadedImage {
    id: u32,
    img: Option<egui::ColorImage>,
}

struct LoadRequest {
    id: u32,
    path: PathBuf,
    ctx: egui::Context,
}

pub struct EnemyList {
    texture_cache: HashMap<u32, egui::TextureHandle>,
    tx_request: Sender<LoadRequest>,
    rx_result: Receiver<LoadedImage>,
    pending_requests: HashSet<u32>,
    missing_ids: HashSet<u32>,
    scroll_to_top_needed: bool,
    last_search_query: String,
    last_unit_count: usize,
    cached_indices: Vec<usize>,
}

impl Default for EnemyList {
    fn default() -> Self {
        let (tx_request, rx_request) = mpsc::channel::<LoadRequest>();
        let (tx_result, rx_result) = mpsc::channel::<LoadedImage>();

        thread::spawn(move || {
            while let Ok(req) = rx_request.recv() {
                let tx = tx_result.clone();
                let ctx = req.ctx.clone(); 

                rayon::spawn(move || {
                    let result = process_image(&req.path);
                    let _ = tx.send(LoadedImage { id: req.id, img: result });
                    ctx.request_repaint();
                });
            }
        });

        Self {
            texture_cache: HashMap::new(),
            tx_request,
            rx_result,
            pending_requests: HashSet::new(),
            missing_ids: HashSet::new(),
            scroll_to_top_needed: false,
            last_search_query: String::new(),
            last_unit_count: 0,
            cached_indices: Vec::new(),
        }
    }
}

impl EnemyList {
    pub fn reset_scroll(&mut self) {
        self.scroll_to_top_needed = true;
    }

    pub fn show(
        &mut self, 
        ctx: &egui::Context, 
        ui: &mut egui::Ui, 
        entries: &[EnemyEntry], 
        selected_id: &mut Option<u32>, 
        search_query: &str,
    ) {
        while let Ok(loaded) = self.rx_result.try_recv() {
            if let Some(img) = loaded.img {
                let texture = ctx.load_texture(
                    format!("enemy_{}", loaded.id),
                    img,
                    egui::TextureOptions::LINEAR
                );
                self.texture_cache.insert(loaded.id, texture);
                self.missing_ids.remove(&loaded.id);
            } else {
                self.texture_cache.remove(&loaded.id);
                self.missing_ids.insert(loaded.id);
            }
            self.pending_requests.remove(&loaded.id);
        }

        if search_query != self.last_search_query || entries.len() != self.last_unit_count {
            self.update_search_cache(entries, search_query);
        }

        let row_height = 55.0; 
        let total_rows = self.cached_indices.len(); 

        let mut scroll_area = egui::ScrollArea::vertical().auto_shrink([false, false]);
        if self.scroll_to_top_needed {
            scroll_area = scroll_area.vertical_scroll_offset(0.0);
            self.scroll_to_top_needed = false;
        }

        scroll_area.show_rows(ui, row_height, total_rows, |ui, row_range| {
            for index in row_range {
                if let Some(&real_index) = self.cached_indices.get(index) {
                    self.render_list_row(ui, entries, real_index, selected_id);
                }
            }
        });
    }

    fn render_list_row(&mut self, ui: &mut egui::Ui, entries: &[EnemyEntry], real_index: usize, selected_id: &mut Option<u32>) {
        let entry = &entries[real_index];
        let is_cached = self.texture_cache.contains_key(&entry.id);
        let is_missing = self.missing_ids.contains(&entry.id);

        if !is_cached && !is_missing && !self.pending_requests.contains(&entry.id) {
            if let Some(path) = &entry.icon_path {
                self.pending_requests.insert(entry.id);
                let _ = self.tx_request.send(LoadRequest {
                    id: entry.id,
                    path: path.clone(),
                    ctx: ui.ctx().clone(),
                });
            } else {
                self.missing_ids.insert(entry.id);
            }
        }

        let is_selected = Some(entry.id) == *selected_id;

        let response = if let Some(tex) = self.texture_cache.get(&entry.id) {
            let size = tex.size_vec2();
            let scale = 50.0 / size.y;
            let btn_size = size * scale;
            ui.add(egui::ImageButton::new((tex.id(), btn_size)).selected(is_selected))
        } else {
            let r = ui.allocate_response(egui::vec2(50.0, 50.0), egui::Sense::click());
            ui.painter().rect_filled(r.rect, 4.0, egui::Color32::from_gray(30));
            r
        };

        if response.clicked() { *selected_id = Some(entry.id); }

       response.on_hover_ui(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("[ID]").weak());
                ui.label(entry.id_str());
            });
            
            let name = entry.display_name();
            
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("[Name]").weak());
                ui.label(name);
            });
        });
    }

    fn update_search_cache(&mut self, entries: &[EnemyEntry], query: &str) {
        self.last_search_query = query.to_string();
        self.last_unit_count = entries.len();
        self.cached_indices.clear();

        let query_lower = query.to_lowercase();
        let is_empty = query.is_empty();

        for (i, entry) in entries.iter().enumerate() {
            if is_empty {
                self.cached_indices.push(i);
                continue;
            }

            // CLEAN: Fully reliant on the scanner.rs methods now!
            let base_id = entry.base_id_str(); 
            let full_id = entry.id_str().to_lowercase();

            // Smart search logic: matches '000' or exactly '000-e'
            if base_id.contains(&query_lower) || full_id == query_lower {
                self.cached_indices.push(i);
                continue;
            }

            if entry.name.to_lowercase().contains(&query_lower) {
                self.cached_indices.push(i);
            }
        }
    }
}

fn process_image(path: &PathBuf) -> Option<egui::ColorImage> {
    if !path.exists() { return None; }
    if let Ok(image_buffer) = image::open(path) {
        let final_image = imageops::resize(&image_buffer.to_rgba8(), 50, 50, imageops::FilterType::Lanczos3);
        let size = [final_image.width() as usize, final_image.height() as usize];
        let pixels = final_image.as_flat_samples();
        return Some(egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()));
    }
    None
}