use std::collections::HashMap;

use ab_glyph::PxScale;
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut, text_size};
use imageproc::rect::Rect;
use nyanko::graphics::rig::SpriteCut;

use core::common::game::{AbilityItem, CustomIcon};

const SUPERSCRIPT_SCALE: f32 = 0.75;
const SUPERSCRIPT_OFFSET_Y: f32 = 0.0;
const SUPERSCRIPT_MARGIN_X: i32 = 2;

pub(crate) fn draw_rounded_rect_mut(img: &mut RgbaImage, rect: Rect, r: i32, color: Rgba<u8>) {
    if r <= 0 { draw_filled_rect_mut(img, rect, color); return; }

    let (w, h) = (rect.width() as i32, rect.height() as i32);
    let (x, y) = (rect.left(), rect.top());
    let r = r.min(w / 2).min(h / 2);

    let w_inner = w - 2 * r;
    let h_inner = h - 2 * r;

    if w_inner > 0 { draw_filled_rect_mut(img, Rect::at(x + r, y).of_size(w_inner as u32, h as u32), color); }
    if h_inner > 0 { draw_filled_rect_mut(img, Rect::at(x, y + r).of_size(w as u32, h_inner as u32), color); }

    imageproc::drawing::draw_filled_circle_mut(img, (x + r, y + r), r, color);
    imageproc::drawing::draw_filled_circle_mut(img, (x + w - 1 - r, y + r), r, color);
    imageproc::drawing::draw_filled_circle_mut(img, (x + r, y + h - 1 - r), r, color);
    imageproc::drawing::draw_filled_circle_mut(img, (x + w - 1 - r, y + h - 1 - r), r, color);
}

pub(crate) fn draw_bottom_rounded_rect_mut(img: &mut RgbaImage, rect: Rect, r: i32, color: Rgba<u8>) {
    if r <= 0 { draw_filled_rect_mut(img, rect, color); return; }

    let (w, h) = (rect.width() as i32, rect.height() as i32);
    let (x, y) = (rect.left(), rect.top());
    let r = r.min(w / 2).min(h);

    let w_inner = w - 2 * r;
    let h_top = h - r;

    if h_top > 0 { draw_filled_rect_mut(img, Rect::at(x, y).of_size(w as u32, h_top as u32), color); }
    if w_inner > 0 && r > 0 { draw_filled_rect_mut(img, Rect::at(x + r, y + h - r).of_size(w_inner as u32, r as u32), color); }

    imageproc::drawing::draw_filled_circle_mut(img, (x + r, y + h - 1 - r), r, color);
    imageproc::drawing::draw_filled_circle_mut(img, (x + w - 1 - r, y + h - 1 - r), r, color);
}

pub(crate) fn get_icon_image(
    item: &AbilityItem, cuts_map: &HashMap<usize, SpriteCut>,
    img015_base: &RgbaImage, custom_assets: &HashMap<CustomIcon, RgbaImage>, export_size: u32,
) -> RgbaImage {
    let mut icon = if item.custom_icon != CustomIcon::None {
        custom_assets.get(&item.custom_icon).cloned().unwrap_or_else(|| RgbaImage::new(export_size, export_size))
    } else if let Some(icon_id) = item.icon_id {
        if let Some(cut) = cuts_map.get(&icon_id) {
            let px = (cut.uv_coordinates.min.x * img015_base.width() as f32).round() as u32;
            let py = (cut.uv_coordinates.min.y * img015_base.height() as f32).round() as u32;
            let pw = cut.original_size.x.round() as u32;
            let ph = cut.original_size.y.round() as u32;

            if px + pw <= img015_base.width() && py + ph <= img015_base.height() {
                image::imageops::crop_imm(img015_base, px, py, pw, ph).to_image()
            } else {
                RgbaImage::new(export_size, export_size)
            }
        } else {
            RgbaImage::new(export_size, export_size)
        }
    } else {
        RgbaImage::new(export_size, export_size)
    };

    if icon.width() != export_size || icon.height() != export_size {
        icon = safe_resize(icon, export_size, export_size);
    }

    if let Some(border_id) = item.border_id
        && let Some(cut) = cuts_map.get(&border_id) {
        let px = (cut.uv_coordinates.min.x * img015_base.width() as f32).round() as u32;
        let py = (cut.uv_coordinates.min.y * img015_base.height() as f32).round() as u32;
        let pw = cut.original_size.x.round() as u32;
        let ph = cut.original_size.y.round() as u32;

        if px + pw <= img015_base.width() && py + ph <= img015_base.height() {
            let mut border = image::imageops::crop_imm(img015_base, px, py, pw, ph).to_image();
            if border.width() != export_size || border.height() != export_size {
                border = safe_resize(border, export_size, export_size);
            }
            image::imageops::overlay(&mut icon, &border, 0, 0);
        }
    }
    icon
}

pub(crate) fn measure_text_with_superscript(scale: PxScale, font: &impl ab_glyph::Font, text: &str) -> u32 {
    let mut total_w = 0;
    let mut parts = text.split('^');

    if let Some(first) = parts.next()
        && !first.is_empty() { total_w += text_size(scale, font, first).0; }

    for part in parts {
        if let Some(space_idx) = part.find(' ') {
            let (super_str, normal_str) = part.split_at(space_idx);
            if !super_str.is_empty() {
                let s_scale = PxScale::from(scale.y * SUPERSCRIPT_SCALE);
                total_w += text_size(s_scale, font, super_str).0 + SUPERSCRIPT_MARGIN_X as u32;
            }
            if !normal_str.is_empty() { total_w += text_size(scale, font, normal_str).0; }
        } else if !part.is_empty() {
            let s_scale = PxScale::from(scale.y * SUPERSCRIPT_SCALE);
            total_w += text_size(s_scale, font, part).0 + SUPERSCRIPT_MARGIN_X as u32;
        }
    }
    total_w
}

pub(crate) fn draw_text_with_superscript(
    img: &mut RgbaImage, color: Rgba<u8>, mut x: i32, y: i32, base_scale: PxScale, font: &impl ab_glyph::Font, text: &str,
) {
    let mut parts = text.split('^');
    if let Some(first) = parts.next()
        && !first.is_empty() {
        draw_text_mut(img, color, x, y, base_scale, font, first);
        x += text_size(base_scale, font, first).0 as i32;
    }

    let s_scale = PxScale::from(base_scale.y * SUPERSCRIPT_SCALE);
    let s_y = y - (base_scale.y * SUPERSCRIPT_OFFSET_Y) as i32;

    for part in parts {
        if let Some(space_idx) = part.find(' ') {
            let (super_str, normal_str) = part.split_at(space_idx);
            if !super_str.is_empty() {
                x += SUPERSCRIPT_MARGIN_X;
                draw_text_mut(img, color, x, s_y, s_scale, font, super_str);
                x += text_size(s_scale, font, super_str).0 as i32;
            }
            if !normal_str.is_empty() {
                draw_text_mut(img, color, x, y, base_scale, font, normal_str);
                x += text_size(base_scale, font, normal_str).0 as i32;
            }
        } else if !part.is_empty() {
            x += SUPERSCRIPT_MARGIN_X;
            draw_text_mut(img, color, x, s_y, s_scale, font, part);
            x += text_size(s_scale, font, part).0 as i32;
        }
    }
}

pub(crate) fn wrap_text(text: &str, font: &impl ab_glyph::Font, scale: PxScale, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        process_paragraph(paragraph, font, scale, max_width, &mut lines);
    }
    if lines.is_empty() { lines.push(String::new()); }
    lines
}

fn process_paragraph(paragraph: &str, font: &impl ab_glyph::Font, scale: PxScale, max_w: f32, lines: &mut Vec<String>) {
    let mut cur_line = String::new();
    let mut cur_word = String::new();

    for c in paragraph.chars() {
        let is_cjk = ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{3040}'..='\u{30FF}').contains(&c) || ('\u{AC00}'..='\u{D7AF}').contains(&c);

        if c.is_whitespace() || is_cjk {
            if !cur_word.is_empty() {
                cur_line = flush_word(cur_line, &cur_word, font, scale, max_w, lines);
                cur_word.clear();
            }
            if is_cjk {
                let t_line = if cur_line.is_empty() { c.to_string() } else { format!("{}{}", cur_line, c) };
                if measure_text_with_superscript(scale, font, &t_line) as f32 > max_w {
                    if !cur_line.is_empty() { lines.push(cur_line.clone()); }
                    cur_line = c.to_string();
                } else {
                    cur_line = t_line;
                }
            }
        } else {
            cur_word.push(c);
        }
    }

    if !cur_word.is_empty() { cur_line = flush_word(cur_line, &cur_word, font, scale, max_w, lines); }
    if !cur_line.is_empty() { lines.push(cur_line); }
}

fn flush_word(line: String, word: &str, font: &impl ab_glyph::Font, scale: PxScale, max_w: f32, lines: &mut Vec<String>) -> String {
    let sep = if line.is_empty() { "" } else { " " };
    let t_line = format!("{}{}{}", line, sep, word);
    if measure_text_with_superscript(scale, font, &t_line) as f32 > max_w {
        if !line.is_empty() {
            lines.push(line);
            word.to_string()
        } else {
            lines.push(word.to_string());
            String::new()
        }
    } else {
        t_line
    }
}

pub(crate) fn draw_centered_text(img: &mut RgbaImage, color: Rgba<u8>, rect: Rect, scale: PxScale, font: &impl ab_glyph::Font, text: &str) {
    let (tw, _) = text_size(scale, font, text);
    let tx = rect.left() + (rect.width() as i32 - tw as i32) / 2;
    let ty = rect.top() + (rect.height() as i32 - scale.y as i32) / 2;
    draw_text_mut(img, color, tx.max(rect.left()), ty.max(rect.top()), scale, font, text);
}

pub(crate) struct TimeCellStyle {
    pub(crate) scale_f: f32,
    pub(crate) scale_i: i32,
    pub(crate) radius: i32,
    pub(crate) text_scale: f32,
}

pub(crate) fn draw_time_cell(
    img: &mut RgbaImage, bg: Rgba<u8>, rect: Rect, frames: i32, font: &impl ab_glyph::Font,
    style: &TimeCellStyle
) {
    draw_rounded_rect_mut(img, rect, style.radius, bg);

    let sec_str = format!("{:.2}s", frames as f32 / 30.0);
    let f_str = format!(" {}f", frames);

    let scale_sec = PxScale::from(15.0 * style.text_scale * style.scale_f);
    let scale_f_text = PxScale::from(15.0 * 0.65 * style.text_scale * style.scale_f);

    let sec_w = text_size(scale_sec, font, &sec_str).0;
    let gap = style.scale_i as u32;
    let total_w = sec_w + text_size(scale_f_text, font, &f_str).0 + gap;

    let start_x = rect.left() + (rect.width() as i32 - total_w as i32) / 2;
    let start_y = rect.top() + (rect.height() as i32 - scale_sec.y as i32) / 2;

    draw_text_mut(img, Rgba([255, 255, 255, 255]), start_x, start_y, scale_sec, font, &sec_str);

    let f_y_offset = (scale_sec.y - scale_f_text.y) * 0.75;
    draw_text_mut(img, Rgba([200, 200, 200, 255]), start_x + sec_w as i32 + gap as i32, start_y + f_y_offset as i32, scale_f_text, font, &f_str);
}

fn safe_resize(mut img: RgbaImage, width: u32, height: u32) -> RgbaImage {
    for p in img.pixels_mut() {
        let a = p[3] as u32;
        if a > 0 && a < 255 {
            p[0] = ((p[0] as u32 * a) / 255) as u8;
            p[1] = ((p[1] as u32 * a) / 255) as u8;
            p[2] = ((p[2] as u32 * a) / 255) as u8;
        } else if a == 0 {
            p[0] = 0; p[1] = 0; p[2] = 0;
        }
    }

    let mut resized = image::imageops::resize(&img, width, height, image::imageops::FilterType::Lanczos3);
    for p in resized.pixels_mut() {
        let a = p[3] as u32;
        if a > 0 && a < 255 {
            p[0] = ((p[0] as u32 * 255) / a).min(255) as u8;
            p[1] = ((p[1] as u32 * 255) / a).min(255) as u8;
            p[2] = ((p[2] as u32 * 255) / a).min(255) as u8;
        }
    }
    resized
}