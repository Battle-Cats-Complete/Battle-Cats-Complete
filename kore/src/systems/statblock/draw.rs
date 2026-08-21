use std::collections::HashMap;

use ab_glyph::{point, Font, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut, text_size, Canvas};
use imageproc::rect::Rect;
use nyanko::graphics::rig::SpriteCut;

use crate::systems::combat::{AbilityItem, CustomIcon};

const SUPERSCRIPT_SHRINK: f32 = 3.0;
const SUPERSCRIPT_GAP: f32 = 1.25;

pub(super) struct SuperscriptStyle {
    pub(super) base: PxScale,
    small: PxScale,
    gap: i32,
    weak: Rgba<u8>,
}

impl SuperscriptStyle {
    pub(super) fn new(size: f32, scale_f: f32, weak: Rgba<u8>) -> Self {
        Self {
            base: PxScale::from(size * scale_f),
            small: PxScale::from((size - SUPERSCRIPT_SHRINK).max(1.0) * scale_f),
            gap: (SUPERSCRIPT_GAP * scale_f).round() as i32,
            weak,
        }
    }

    pub(super) fn measure(&self, font: &impl Font, text: &str) -> u32 {
        let mut total_width = 0;
        let mut parts = text.split('^');

        if let Some(first) = parts.next()
            && !first.is_empty() { total_width += text_size(self.base, font, first).0; }

        for part in parts {
            let (super_str, normal_str) = split_superscript(part);

            if !super_str.is_empty() {
                total_width += text_size(self.small, font, super_str).0 + self.gap as u32;
            }
            if !normal_str.is_empty() {
                total_width += text_size(self.base, font, normal_str).0;
            }
        }

        total_width
    }

    pub(super) fn draw<C: Canvas<Pixel = Rgba<u8>>>(
        &self, img: &mut C, color: Rgba<u8>, mut x: i32, y: i32, font: &impl Font, text: &str,
    ) {
        let mut parts = text.split('^');

        if let Some(first) = parts.next()
            && !first.is_empty() {
            draw_text_mut(img, color, x, y, self.base, font, first);
            x += text_size(self.base, font, first).0 as i32;
        }

        for part in parts {
            let (super_str, normal_str) = split_superscript(part);

            if !super_str.is_empty() {
                x += self.gap;
                draw_text_mut(img, self.weak, x, y, self.small, font, super_str);
                x += text_size(self.small, font, super_str).0 as i32;
            }
            if !normal_str.is_empty() {
                draw_text_mut(img, color, x, y, self.base, font, normal_str);
                x += text_size(self.base, font, normal_str).0 as i32;
            }
        }
    }
}

fn split_superscript(part: &str) -> (&str, &str) {
    part.find(' ').map_or((part, ""), |break_index| part.split_at(break_index))
}

pub(super) fn line_offset(font: &impl Font, scale: PxScale, line_height: i32) -> i32 {
    let scaled = font.as_scaled(scale);

    ((line_height as f32 - scaled.height()) / 2.0).round() as i32
}

pub(super) fn draw_rounded_rect_mut(img: &mut RgbaImage, rect: Rect, r: i32, color: Rgba<u8>) {
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

#[derive(Clone, Copy)]
pub(super) struct CellShape {
    pub(super) rect: Rect,
    radius: i32,
    border: i32,
}

impl CellShape {
    pub(super) fn new(rect: Rect, radius: i32, border: i32) -> Self {
        Self { rect, radius, border }
    }

    fn inner(&self) -> (Rect, i32) {
        let width = (self.rect.width() as i32 - self.border * 2).max(1) as u32;
        let height = (self.rect.height() as i32 - self.border * 2).max(1) as u32;
        let rect = Rect::at(self.rect.left() + self.border, self.rect.top() + self.border).of_size(width, height);

        (rect, (self.radius - self.border).max(0))
    }
}

pub(super) fn draw_bordered_rect_mut(img: &mut RgbaImage, cell: CellShape, fill: Rgba<u8>, border_color: Rgba<u8>) {
    draw_rounded_rect_mut(img, cell.rect, cell.radius, border_color);

    if cell.border <= 0 { return; }

    let (inner, inner_radius) = cell.inner();

    draw_rounded_rect_mut(img, inner, inner_radius, fill);
}

pub(super) fn draw_bottom_rounded_rect_mut(img: &mut RgbaImage, rect: Rect, r: i32, color: Rgba<u8>) {
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

pub(super) fn get_icon_image(
    item: &AbilityItem, cuts_map: &HashMap<usize, SpriteCut>,
    img015_base: &RgbaImage, custom_assets: &HashMap<CustomIcon, RgbaImage>, export_size: u32,
) -> RgbaImage {
    let mut icon = if item.custom_icon != CustomIcon::None {
        custom_assets.get(&item.custom_icon).cloned().unwrap_or_else(|| RgbaImage::new(export_size, export_size))
    } else if let Some(icon_id) = item.icon_id {
        cuts_map.get(&icon_id).map_or_else(
            || RgbaImage::new(export_size, export_size),
            |cut| {
                let (px, py) = (cut.x.max(0) as u32, cut.y.max(0) as u32);
                let (pw, ph) = (cut.width.max(0) as u32, cut.height.max(0) as u32);

                if px + pw <= img015_base.width() && py + ph <= img015_base.height() {
                    image::imageops::crop_imm(img015_base, px, py, pw, ph).to_image()
                } else {
                    RgbaImage::new(export_size, export_size)
                }
            },
        )
    } else {
        RgbaImage::new(export_size, export_size)
    };

    if icon.width() != export_size || icon.height() != export_size {
        icon = safe_resize(icon, export_size, export_size);
    }

    if let Some(border_id) = item.border_id
        && let Some(cut) = cuts_map.get(&border_id) {
        let (px, py) = (cut.x.max(0) as u32, cut.y.max(0) as u32);
        let (pw, ph) = (cut.width.max(0) as u32, cut.height.max(0) as u32);

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

pub(super) fn wrap_text(text: &str, font: &impl Font, scale: PxScale, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        process_paragraph(paragraph, font, scale, max_width, &mut lines);
    }
    if lines.is_empty() { lines.push(String::new()); }
    lines
}

fn process_paragraph(paragraph: &str, font: &impl Font, scale: PxScale, max_w: f32, lines: &mut Vec<String>) {
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
                if text_size(scale, font, &t_line).0 as f32 > max_w {
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

fn flush_word(line: String, word: &str, font: &impl Font, scale: PxScale, max_w: f32, lines: &mut Vec<String>) -> String {
    let sep = if line.is_empty() { "" } else { " " };
    let t_line = format!("{}{}{}", line, sep, word);
    if text_size(scale, font, &t_line).0 as f32 > max_w {
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

const BASELINE_REFERENCE: &str = "0";

pub(super) struct RoundedClip<'a> {
    img: &'a mut RgbaImage,
    rect: Rect,
    radius: i32,
}

impl<'a> RoundedClip<'a> {
    fn new(img: &'a mut RgbaImage, rect: Rect, radius: i32) -> Self {
        let radius = radius.min(rect.width() as i32 / 2).min(rect.height() as i32 / 2).max(0);

        Self { img, rect, radius }
    }

    fn contains(&self, x: u32, y: u32) -> bool {
        let (x, y) = (x as i32, y as i32);
        let left = self.rect.left();
        let top = self.rect.top();
        let right = left + self.rect.width() as i32 - 1;
        let bottom = top + self.rect.height() as i32 - 1;

        if x < left || x > right || y < top || y > bottom {
            return false;
        }

        if self.radius == 0 {
            return true;
        }

        let corner_x = if x < left + self.radius {
            left + self.radius
        } else if x > right - self.radius {
            right - self.radius
        } else {
            return true;
        };

        let corner_y = if y < top + self.radius {
            top + self.radius
        } else if y > bottom - self.radius {
            bottom - self.radius
        } else {
            return true;
        };

        let (dx, dy) = (x - corner_x, y - corner_y);

        dx * dx + dy * dy <= self.radius * self.radius
    }
}

impl Canvas for RoundedClip<'_> {
    type Pixel = Rgba<u8>;

    fn dimensions(&self) -> (u32, u32) {
        self.img.dimensions()
    }

    fn get_pixel(&self, x: u32, y: u32) -> Rgba<u8> {
        *self.img.get_pixel(x, y)
    }

    fn draw_pixel(&mut self, x: u32, y: u32, color: Rgba<u8>) {
        if self.contains(x, y) {
            self.img.put_pixel(x, y, color);
        }
    }
}

fn ink_bounds(font: &impl Font, scale: PxScale, text: &str) -> Option<(f32, f32)> {
    let scaled = font.as_scaled(scale);
    let mut caret = 0.0;
    let mut top = f32::MAX;
    let mut bottom = f32::MIN;

    for c in text.chars() {
        let glyph_id = scaled.glyph_id(c);
        let glyph = glyph_id.with_scale_and_position(scale, point(caret, scaled.ascent()));
        caret += scaled.h_advance(glyph_id);

        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            top = top.min(bounds.min.y);
            bottom = bottom.max(bounds.max.y);
        }
    }

    (top <= bottom).then_some((top, bottom))
}

fn centered_ink_y(font: &impl Font, scale: PxScale, rect: Rect) -> i32 {
    let Some((top, bottom)) = ink_bounds(font, scale, BASELINE_REFERENCE) else {
        return rect.top();
    };

    let offset = (rect.height() as f32 - (bottom - top)) / 2.0 - top;

    rect.top() + offset.round() as i32
}

pub(super) fn draw_centered_text(
    img: &mut RgbaImage, color: Rgba<u8>, cell: CellShape, scale: PxScale, font: &impl Font, text: &str,
) {
    let (inner, inner_radius) = cell.inner();
    let (tw, _) = text_size(scale, font, text);
    let tx = inner.left() + (inner.width() as i32 - tw as i32) / 2;
    let ty = centered_ink_y(font, scale, cell.rect);

    draw_text_mut(&mut RoundedClip::new(img, inner, inner_radius), color, tx.max(inner.left()), ty, scale, font, text);
}

pub(super) fn draw_centered_superscript(
    img: &mut RgbaImage, color: Rgba<u8>, cell: CellShape, style: &SuperscriptStyle, font: &impl Font, text: &str,
) {
    let (inner, inner_radius) = cell.inner();
    let width = style.measure(font, text);
    let tx = inner.left() + (inner.width() as i32 - width as i32) / 2;
    let ty = centered_ink_y(font, style.base, cell.rect);

    style.draw(&mut RoundedClip::new(img, inner, inner_radius), color, tx.max(inner.left()), ty, font, text);
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
