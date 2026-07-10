use std::collections::HashMap;
use std::path::Path;

use eframe::egui;

use core::global::formats::gatyaitembuy::GatyaItemBuy;
use core::global::formats::gatyaitemname::GatyaItemName;
use core::global::utils::autocrop;
use core::stage::logic::materials;
use core::stage::registry::{Map, Stage};

pub const MAT_TABLE_WIDTH: f32 = 345.0;
const COL_SPACING: f32 = 8.0;

fn process_item_icon_texture(p: &Path) -> Option<egui::ColorImage> {
    let Ok(img) = image::open(p) else { return None; };
    let crop = autocrop(img.to_rgba8());
    let (w, h) = crop.dimensions();
    let max_dim = w.max(h) as f32;
    let scale = 32.0 / max_dim;
    let tw = (w as f32 * scale).round() as u32;
    let th = (h as f32 * scale).round() as u32;
    let res = image::imageops::resize(&crop, tw.max(1), th.max(1), image::imageops::FilterType::Triangle);
    let dims = [res.width() as usize, res.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(dims, res.as_flat_samples().as_slice()))
}

pub fn has_drops(_s: &Stage, m: &Map) -> bool {
    m.drop_items.as_ref().is_some_and(|d| d.material_drops.iter().any(|&c| c > 0))
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    s: &Stage,
    m: &Map,
    selected_crown: u8, // <-- ADDED HERE
    buy: &HashMap<u32, GatyaItemBuy>,
    names: &HashMap<usize, GatyaItemName>,
    cache: &mut HashMap<u32, egui::TextureHandle>,
    langs: &[String]
) {
    if !has_drops(s, m) { return; }

    let Some(drops) = &m.drop_items else { return; };

    let s_idx = s.stage_id as usize;
    let b_amt = drops.stage_drops.get(s_idx).copied().unwrap_or(0);
    let mul = drops.crown_multipliers.get(selected_crown as usize).copied().unwrap_or(1.0);
    let f_amt = (b_amt as f32 * mul).round() as u32;

    ui.strong(format!("Material Drops | Amount: {} ({}×{})", f_amt, b_amt, mul));
    ui.separator();
    ui.add_space(4.0);

    let b_mats = &drops.material_drops[0..8];
    let z_mats = &drops.material_drops[8..16];

    let h_base = b_mats.iter().any(|&c| c > 0);
    let h_z = z_mats.iter().any(|&c| c > 0);

    let c_width = (MAT_TABLE_WIDTH - (COL_SPACING * 3.0)) / 4.0;

    egui::Grid::new("mat_grid")
        .striped(true)
        .spacing([COL_SPACING, 6.0])
        .show(ui, |g| {
            if h_base {
                draw_chunks(ctx, g, b_mats, 0, c_width, buy, names, cache, langs);
            }
            if h_z {
                draw_chunks(ctx, g, z_mats, 8, c_width, buy, names, cache, langs);
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn draw_chunks(
    ctx: &egui::Context,
    g: &mut egui::Ui,
    c_arr: &[u32],
    offset: usize,
    c_width: f32,
    buy: &HashMap<u32, GatyaItemBuy>,
    names: &HashMap<usize, GatyaItemName>,
    cache: &mut HashMap<u32, egui::TextureHandle>,
    langs: &[String]
) {
    for (c_idx, chunk) in c_arr.chunks(4).enumerate() {
        let chunk_off = offset + (c_idx * 4);
        let mut res = Vec::new();

        for (i, &c) in chunk.iter().enumerate() {
            res.push(materials::resolve(chunk_off + i, c, buy, names, langs));
        }

        for d in &res {
            g.centered_and_justified(|ui| {
                ui.set_max_width(c_width);
                ui.add(egui::Label::new(egui::RichText::new(&d.name).strong()).wrap_mode(egui::TextWrapMode::Wrap).truncate());
            });
        }
        g.end_row();

        for (i, d) in res.iter().enumerate() {
            g.centered_and_justified(|ui| {
                let mut is_rendered = false;
                let id = 10000 + (chunk_off + i) as u32;

                if let Some(path) = &d.image_path {
                    if !cache.contains_key(&id)
                        && let Some(img) = process_item_icon_texture(path) {
                        let tex = ctx.load_texture(format!("mat_icon_{}", id), img, egui::TextureOptions::LINEAR);
                        cache.insert(id, tex);
                    }

                    if let Some(tex) = cache.get(&id) {
                        let resp = ui.add(egui::Image::new(tex).max_size(egui::vec2(32.0, 32.0)));
                        resp.on_hover_text(d.name.clone());
                        is_rendered = true;
                    }
                }

                if !is_rendered {
                    ui.add_sized([32.0, 32.0], egui::Label::new(&d.name).truncate());
                }
            });
        }
        g.end_row();

        for &c in chunk {
            g.centered_and_justified(|ui| {
                let color = if c > 0 { ui.visuals().text_color() } else { egui::Color32::DARK_GRAY };
                ui.label(egui::RichText::new(format!("{}%", c)).color(color).strong());
            });
        }
        g.end_row();
    }
}