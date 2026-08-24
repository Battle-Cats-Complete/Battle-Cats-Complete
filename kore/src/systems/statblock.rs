mod draw;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use ab_glyph::{Font, FontRef, PxScale};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut, text_size};
use imageproc::rect::Rect;
use nyanko::combat::Identity;
use nyanko::common::data::img015;

use crate::common::assets;
use crate::common::formats::SpriteSheet;
use crate::systems::combat::{AbilityItem, CustomIcon};
use crate::common::gfx::autocrop;

use draw::*;

#[derive(Clone)]
pub struct SpiritData {
    pub dmg_text: String,
    pub traits: Vec<AbilityItem>,
    pub h1: Vec<AbilityItem>,
    pub h2: Vec<AbilityItem>,
    pub b1: Vec<AbilityItem>,
    pub b2: Vec<AbilityItem>,
    pub footer: Vec<AbilityItem>,
}

#[derive(Clone)]
pub enum StatCell {
    Text(String),
    Frames(i32),
}

#[derive(Clone)]
pub struct StatblockData {
    pub is_cat: bool,
    pub id_str: String,
    pub name: String,
    pub icon_path: Option<PathBuf>,
    pub top_label: String,
    pub top_value: String,
    pub headers_1: Vec<String>,
    pub data_1: Vec<StatCell>,
    pub headers_2: Vec<String>,
    pub data_2: Vec<StatCell>,
    pub traits: Vec<AbilityItem>,
    pub h1: Vec<AbilityItem>,
    pub h2: Vec<AbilityItem>,
    pub b1: Vec<AbilityItem>,
    pub b2: Vec<AbilityItem>,
    pub footer: Vec<AbilityItem>,
    pub spirit_data: Option<SpiritData>,
}

const COLOR_BACKGROUND: Rgba<u8> = Rgba([33, 33, 33, 255]);
const COLOR_TEXT: Rgba<u8> = Rgba([230, 230, 230, 255]);
const COLOR_TEXT_WEAK: Rgba<u8> = Rgba([112, 112, 112, 255]);
const COLOR_SUPERSCRIPT: Rgba<u8> = Rgba([171, 171, 171, 255]);
const COLOR_RULE: Rgba<u8> = Rgba([71, 71, 71, 255]);
const COLOR_CELL_HEADER: Rgba<u8> = Rgba([15, 15, 15, 255]);
const COLOR_CELL_VALUE: Rgba<u8> = Rgba([58, 58, 58, 255]);
const COLOR_CELL_BORDER: Rgba<u8> = Rgba([42, 42, 42, 255]);
const COLOR_INPUT: Rgba<u8> = Rgba([17, 17, 17, 255]);
const COLOR_INPUT_BORDER: Rgba<u8> = Rgba([71, 71, 71, 255]);
const COLOR_CARD: Rgba<u8> = Rgba([7, 7, 7, 255]);
const COLOR_FRAME: Rgba<u8> = Rgba([31, 106, 165, 255]);

const RADIUS_SM: f32 = 4.0;
const RADIUS_MD: f32 = 6.0;
const RADIUS_LG: f32 = 8.0;

const TEXT_SCALE: f32 = 1.5;
const CONTENT_PADDING: f32 = 8.0;
const SECTION_GAP: f32 = 8.0;
const RULE_THICKNESS: f32 = 1.0;
const BORDER_WIDTH: f32 = 1.0;

const ICON_BOX_WIDTH: f32 = 110.0;
const ICON_BOX_HEIGHT: f32 = 83.0;
const HEADER_GAP_X: f32 = 12.0;

const NAME_WRAP_WIDTH: f32 = 145.0;
const NAME_MAX_FONT_SIZE: f32 = 22.0 * TEXT_SCALE;
const NAME_MIN_FONT_SIZE: f32 = 8.0 * TEXT_SCALE;
const NAME_LINE_GAP: f32 = -4.0;
const NAME_TO_ID_GAP: f32 = -1.5;
const NAME_FONT_STEP: f32 = 0.5;
const NAME_MAX_LINES: usize = 2;

const INFO_TEXT_SIZE: f32 = 11.0 * TEXT_SCALE;
const LEVEL_ROW_GAP: f32 = 6.0;
const LEVEL_INPUT_MIN_WIDTH: f32 = 28.0;
const LEVEL_INPUT_PADDING_X: f32 = 7.0;
const LEVEL_INPUT_PADDING_Y: f32 = 2.0;

const CELL_WIDTH: f32 = 82.0;
const CELL_HEIGHT: f32 = 28.0;
const CELL_GAP: f32 = 4.0;
const CELL_TEXT_SIZE: f32 = 12.5 * TEXT_SCALE;
const CELL_BORDER_WIDTH: f32 = 2.0;

const ABILITY_ICON_SIZE: f32 = 40.0;
const ABILITY_TEXT_SIZE: f32 = 13.0 * TEXT_SCALE;
const ABILITY_TEXT_GAP: f32 = 8.0;
const ABILITY_LINE_GAP: f32 = -2.0;
const ABILITY_OVERFLOW_PADDING: f32 = 4.5;
const ICON_GAP_X: f32 = 3.2;
const ICON_GAP_Y: f32 = 5.5;
const ICON_TRAIT_GAP_Y: f32 = 7.0;

const CARD_PADDING: f32 = 8.0;
const SPIRIT_PADDING_X: f32 = 8.0;

const FRAME_THICKNESS: f32 = 7.0;
const FRAME_PADDING: f32 = 4.0;
const FRAME_RADIUS: f32 = RADIUS_MD;

const RENDER_SCALE: i32 = 2;

pub fn build_statblock_image(
    priority: &[String],
    layers: &[SpriteSheet],
    data: StatblockData,
) -> Result<RgbaImage, String> {
    let scale = RENDER_SCALE;
    let scale_f = scale as f32;
    let px = |value: f32| (value * scale_f).round() as i32;

    let padding = px(CONTENT_PADDING);
    let section_gap = px(SECTION_GAP);
    let cell_width = px(CELL_WIDTH);
    let cell_height = px(CELL_HEIGHT);
    let cell_gap = px(CELL_GAP);
    let cell_radius = px(RADIUS_SM);
    let cell_border = px(CELL_BORDER_WIDTH);
    let icon_size = px(ABILITY_ICON_SIZE);
    let icon_gap_x = px(ICON_GAP_X);
    let icon_gap_y = px(ICON_GAP_Y);
    let trait_gap_y = px(ICON_TRAIT_GAP_Y);
    let text_gap_x = px(ABILITY_TEXT_GAP);
    let card_padding = px(CARD_PADDING);

    let icon_box_width = px(ICON_BOX_WIDTH);
    let icon_box_height = px(ICON_BOX_HEIGHT);

    let header_icon = data.icon_path.as_ref().and_then(|path| image::open(path).ok()).map(|icon_img| {
        let rgba = autocrop(icon_img.to_rgba8());
        let (source_w, source_h) = (rgba.width() as f32, rgba.height() as f32);

        let scale = (icon_box_width as f32 / source_w).min(icon_box_height as f32 / source_h);
        let target_w = ((source_w * scale).round() as u32).max(1);
        let target_h = ((source_h * scale).round() as u32).max(1);

        if rgba.width() == target_w && rgba.height() == target_h {
            return rgba;
        }

        image::imageops::resize(&rgba, target_w, target_h, image::imageops::FilterType::Lanczos3)
    });

    let icon_column_width = header_icon.as_ref().map_or(icon_box_width, |icon| icon_box_width.max(icon.width() as i32));
    let header_height = icon_box_height;

    let max_cols = data.headers_1.len().max(data.headers_2.len()) as f32;
    let base_grid_width =
        (CONTENT_PADDING * 2.0) + (CELL_WIDTH * max_cols) + (CELL_GAP * (max_cols - 1.0).max(0.0));
    let header_width = (CONTENT_PADDING * 2.0) + HEADER_GAP_X + NAME_WRAP_WIDTH + (icon_column_width as f32 / scale_f);

    let jp_font = FontRef::try_from_slice(assets::FONT_JP)
        .map_err(|err| format!("Failed to load JP font: {err}"))?;
    let kr_font = FontRef::try_from_slice(assets::FONT_KR)
        .map_err(|err| format!("Failed to load KR font: {err}"))?;
    let tc_font = FontRef::try_from_slice(assets::FONT_TC)
        .map_err(|err| format!("Failed to load TC font: {err}"))?;
    let th_font = FontRef::try_from_slice(assets::FONT_TH)
        .map_err(|err| format!("Failed to load TH font: {err}"))?;

    let get_font = |lang: &str| -> &FontRef {
        match lang {
            "ko" | "kr" => &kr_font,
            "tw" | "zh" | "zh-tw" => &tc_font,
            "th" => &th_font,
            _ => &jp_font,
        }
    };

    let font_supports_string = |candidate_font: &FontRef, text: &str| -> bool {
        for character in text.chars() {
            if character.is_ascii() || character.is_whitespace() { continue; }
            if candidate_font.glyph_id(character).0 == 0 {
                return false;
            }
        }
        true
    };

    let mut selected_font = &jp_font;
    let mut found_match = false;

    for lang in priority {
        let candidate_font = get_font(lang);
        if font_supports_string(candidate_font, &data.name) {
            selected_font = candidate_font;
            found_match = true;
            break;
        }
    }

    if !found_match {
        selected_font = get_font(priority.first().map(|s| s.as_str()).unwrap_or("en"));
    }

    let font = selected_font;

    let measure_style = SuperscriptStyle::new(ABILITY_TEXT_SIZE, 1.0, COLOR_SUPERSCRIPT);
    let ability_style = SuperscriptStyle::new(ABILITY_TEXT_SIZE, scale_f, COLOR_SUPERSCRIPT);
    let cell_style = SuperscriptStyle::new(CELL_TEXT_SIZE, scale_f, COLOR_SUPERSCRIPT);

    let check_icon_row_width = |items: &Vec<AbilityItem>| -> f32 {
        if items.is_empty() { return 0.0; }
        (CONTENT_PADDING * 2.0) + (items.len() as f32 * (ABILITY_ICON_SIZE + ICON_GAP_X)) - ICON_GAP_X
    };

    let calc_spirit_width = |spirit: &SpiritData| -> f32 {
        let mut spirit_max = 0.0_f32;
        let start_x = CONTENT_PADDING;
        let text_x = start_x + ABILITY_ICON_SIZE + ABILITY_TEXT_GAP;

        for line in spirit.dmg_text.split('\n') {
            spirit_max = spirit_max.max(text_x + measure_style.measure(font, line) as f32);
        }

        for spirit_item in spirit.b1.iter().chain(spirit.b2.iter()) {
            for line in spirit_item.text.split('\n') {
                spirit_max = spirit_max.max(text_x + measure_style.measure(font, line) as f32);
            }
        }

        for spirit_items in [&spirit.traits, &spirit.h1, &spirit.h2, &spirit.footer] {
            if spirit_items.is_empty() { continue; }
            let icon_width = start_x + (spirit_items.len() as f32 * (ABILITY_ICON_SIZE + ICON_GAP_X)) - ICON_GAP_X;
            spirit_max = spirit_max.max(icon_width);
        }
        spirit_max
    };

    let mut list_max_width = 0.0_f32;
    for item in data.b1.iter().chain(data.b2.iter()) {
        let mut max_line_width = 0.0_f32;
        for line in item.text.split('\n') {
            max_line_width = max_line_width.max(measure_style.measure(font, line) as f32);
        }

        let mut container_width =
            CONTENT_PADDING + ABILITY_ICON_SIZE + ABILITY_TEXT_GAP + max_line_width + CONTENT_PADDING;

        if item.icon_id == Some(img015::ICON_CONJURE)
            && let Some(spirit) = &data.spirit_data {
            container_width = container_width.max(CONTENT_PADDING + calc_spirit_width(spirit) + SPIRIT_PADDING_X);
        }
        list_max_width = list_max_width.max(container_width);
    }

    let mut max_needed_width = base_grid_width;
    max_needed_width = max_needed_width.max(header_width);
    max_needed_width = max_needed_width.max(list_max_width);
    max_needed_width = max_needed_width.max(check_icon_row_width(&data.traits));
    max_needed_width = max_needed_width.max(check_icon_row_width(&data.h1));
    max_needed_width = max_needed_width.max(check_icon_row_width(&data.h2));
    max_needed_width = max_needed_width.max(check_icon_row_width(&data.footer));

    let canvas_width = (max_needed_width.ceil() as i32) * scale;
    let mut target_image = RgbaImage::from_pixel(canvas_width as u32, 4000 * scale as u32, COLOR_BACKGROUND);

    let mut custom_assets = HashMap::new();
    for (variant, bytes) in assets::CUSTOM_ICON_DATA {
        if let Ok(loaded_img) = image::load_from_memory(bytes) {
            custom_assets.insert(*variant, loaded_img.to_rgba8());
        }
    }

    let info_scale = PxScale::from(INFO_TEXT_SIZE * scale_f);
    let info_line_height = px(INFO_TEXT_SIZE);
    let input_padding_x = px(LEVEL_INPUT_PADDING_X);
    let input_height = info_line_height + px(LEVEL_INPUT_PADDING_Y) * 2;
    let level_row_height = input_height.max(info_line_height);

    let name_to_id_gap = px(NAME_TO_ID_GAP);
    let name_box_height = (header_height - name_to_id_gap - info_line_height - level_row_height)
        .max(px(NAME_MIN_FONT_SIZE));

    let mut name_size = NAME_MAX_FONT_SIZE;
    let mut name_lines = wrap_text(&data.name, font, PxScale::from(name_size * scale_f), NAME_WRAP_WIDTH * scale_f);
    let name_overflows = |lines: usize, size: f32| -> bool {
        lines > NAME_MAX_LINES || lines as i32 * px(size + NAME_LINE_GAP) > name_box_height
    };

    while name_overflows(name_lines.len(), name_size) && name_size > NAME_MIN_FONT_SIZE {
        name_size -= NAME_FONT_STEP;
        name_lines = wrap_text(&data.name, font, PxScale::from(name_size * scale_f), NAME_WRAP_WIDTH * scale_f);
    }

    let name_scale = PxScale::from(name_size * scale_f);
    let name_line_height = px(name_size + NAME_LINE_GAP);
    let name_block_height = name_lines.len() as i32 * name_line_height;

    let header_top = padding;

    if let Some(icon) = &header_icon {
        let x_offset = padding as i64 + ((icon_column_width - icon.width() as i32) / 2) as i64;
        let seated = (header_height - icon.height() as i32).max(0);

        image::imageops::overlay(&mut target_image, icon, x_offset, (header_top + seated) as i64);
    }

    let text_start_x = padding + icon_column_width + px(HEADER_GAP_X);
    let info_top = header_top;

    let mut name_y = info_top + (name_box_height - name_block_height).max(0) / 2;
    let name_offset = line_offset(font, name_scale, name_line_height);
    for line in &name_lines {
        draw_text_mut(&mut target_image, COLOR_TEXT, text_start_x, name_y + name_offset, name_scale, font, line);
        name_y += name_line_height;
    }

    let info_offset = line_offset(font, info_scale, info_line_height);
    let id_top = info_top + name_box_height + name_to_id_gap;
    draw_text_mut(
        &mut target_image, COLOR_TEXT_WEAK, text_start_x, id_top + info_offset,
        info_scale, font, &format!("ID: {}", data.id_str),
    );

    let level_top = id_top + info_line_height;
    draw_text_mut(
        &mut target_image, COLOR_TEXT, text_start_x,
        level_top + (level_row_height - info_line_height) / 2 + info_offset,
        info_scale, font, &data.top_label,
    );

    let (prefix_width, _) = text_size(info_scale, font, &data.top_label);
    let (value_width, _) = text_size(info_scale, font, &data.top_value);
    let input_width = (value_width as i32 + input_padding_x * 2).max(px(LEVEL_INPUT_MIN_WIDTH));
    let input_x = text_start_x + prefix_width as i32 + px(LEVEL_ROW_GAP);
    let input_y = level_top + (level_row_height - input_height) / 2;
    let input_rect = Rect::at(input_x, input_y).of_size(input_width as u32, input_height as u32);

    let input_cell = CellShape::new(input_rect, cell_radius, px(BORDER_WIDTH));
    draw_bordered_rect_mut(&mut target_image, input_cell, COLOR_INPUT, COLOR_INPUT_BORDER);
    draw_centered_text(&mut target_image, COLOR_TEXT, input_cell, info_scale, font, &data.top_value);

    let mut current_y_global = header_top + header_height + section_gap;
    draw_filled_rect_mut(
        &mut target_image,
        Rect::at(padding, current_y_global).of_size((canvas_width - padding * 2) as u32, px(RULE_THICKNESS) as u32),
        COLOR_RULE,
    );
    current_y_global += px(RULE_THICKNESS) + section_gap;

    let row_pitch = cell_height + cell_gap;
    let r1_hy = current_y_global;
    let r1_dy = current_y_global + row_pitch;
    let r2_hy = current_y_global + row_pitch * 2;
    let r2_dy = current_y_global + row_pitch * 3;

    let render_row = |ui_img: &mut RgbaImage, headers: &[String], row_data: &[StatCell], h_y: i32, d_y: i32| {
        for col in 0..headers.len() {
            let current_x = padding + ((col as i32) * (cell_width + cell_gap));

            let h_cell = CellShape::new(Rect::at(current_x, h_y).of_size(cell_width as u32, cell_height as u32), cell_radius, cell_border);
            draw_bordered_rect_mut(ui_img, h_cell, COLOR_CELL_HEADER, COLOR_CELL_BORDER);
            draw_centered_text(ui_img, COLOR_TEXT, h_cell, cell_style.base, font, &headers[col]);

            let d_cell = CellShape::new(Rect::at(current_x, d_y).of_size(cell_width as u32, cell_height as u32), cell_radius, cell_border);
            draw_bordered_rect_mut(ui_img, d_cell, COLOR_CELL_VALUE, COLOR_CELL_BORDER);

            match &row_data[col] {
                StatCell::Frames(frames) => draw_centered_superscript(
                    ui_img, COLOR_TEXT, d_cell, &cell_style, font, &frame_text(*frames),
                ),
                StatCell::Text(text) => draw_centered_text(ui_img, COLOR_TEXT, d_cell, cell_style.base, font, text),
            }
        }
    };

    render_row(&mut target_image, &data.headers_1, &data.data_1, r1_hy, r1_dy);
    render_row(&mut target_image, &data.headers_2, &data.data_2, r2_hy, r2_dy);

    current_y_global += row_pitch * 3 + cell_height + section_gap;

    let ability_line_height = px(ABILITY_TEXT_SIZE + ABILITY_LINE_GAP);
    let ability_offset = line_offset(font, ability_style.base, ability_line_height);

    let draw_icon_row = |canvas_image: &mut RgbaImage, items: &Vec<AbilityItem>, start_y: i32, start_x: i32| -> i32 {
        if items.is_empty() { return start_y; }
        let mut current_x = start_x;
        let mut current_y = start_y;
        for ability_item in items {
            if current_x + icon_size > canvas_width - padding {
                current_x = start_x;
                current_y += icon_size + icon_gap_y;
            }
            let icon_surface = get_icon_image(ability_item, layers, &custom_assets, icon_size as u32);
            image::imageops::overlay(canvas_image, &icon_surface, current_x as i64, current_y as i64);
            current_x += icon_size + icon_gap_x;
        }
        current_y + icon_size
    };

    let overflow_padding = |text: &str| -> i32 {
        let block_height = text.split('\n').count() as i32 * ability_line_height;

        if block_height > icon_size { px(ABILITY_OVERFLOW_PADDING) } else { 0 }
    };

    let row_height = |text: &str| -> i32 { icon_size + overflow_padding(text) * 2 };

    let draw_text_block = |canvas_image: &mut RgbaImage, text: &str, icon_x: i32, icon_y: i32| {
        let lines: Vec<&str> = text.split('\n').collect();
        let block_height = lines.len() as i32 * ability_line_height;
        let mut line_y = icon_y + (icon_size - block_height) / 2;

        for line in lines {
            ability_style.draw(canvas_image, COLOR_TEXT, icon_x + icon_size + text_gap_x, line_y + ability_offset, font, line);
            line_y += ability_line_height;
        }
    };

    let draw_spirit_icons = |spirit_image: &mut RgbaImage, spirit_items: &[AbilityItem], start_y: i32, start_x_absolute: i32| -> i32 {
        if spirit_items.is_empty() { return start_y; }
        let mut current_x = start_x_absolute;
        for spirit_item in spirit_items {
            let icon_surface = get_icon_image(spirit_item, layers, &custom_assets, icon_size as u32);
            image::imageops::overlay(spirit_image, &icon_surface, current_x as i64, start_y as i64);
            current_x += icon_size + icon_gap_x;
        }
        start_y + icon_size
    };

    let draw_spirit_list = |spirit_image: &mut RgbaImage, spirit_items: &[AbilityItem], start_y: i32, start_x_absolute: i32| -> i32 {
        if spirit_items.is_empty() { return start_y; }
        let mut current_y = start_y;
        for (index, spirit_item) in spirit_items.iter().enumerate() {
            let icon_surface = get_icon_image(spirit_item, layers, &custom_assets, icon_size as u32);
            let icon_y = current_y + overflow_padding(&spirit_item.text);
            image::imageops::overlay(spirit_image, &icon_surface, start_x_absolute as i64, icon_y as i64);

            draw_text_block(spirit_image, &spirit_item.text, start_x_absolute, icon_y);
            current_y += row_height(&spirit_item.text);
            if index < spirit_items.len() - 1 { current_y += icon_gap_y; }
        }
        current_y
    };

    let draw_spirit_card = |canvas_image: &mut RgbaImage, spirit: &SpiritData, card_start_y: i32| -> i32 {
        let card_inner_y = card_start_y + icon_gap_y;
        let start_x_absolute = padding + card_padding;
        let spirit_panel_width = px(calc_spirit_width(spirit) + SPIRIT_PADDING_X);

        let mut final_panel_height = card_padding;
        final_panel_height += row_height(&spirit.dmg_text) + icon_gap_y;

        let mut has_previous_section = false;
        let mut last_section_was_trait = false;

        let add_gap = |total_height: &mut i32, has_previous_element: &mut bool, current_is_trait: bool, was_last_element_trait: &mut bool| {
            if *has_previous_element { *total_height += if *was_last_element_trait { trait_gap_y } else { icon_gap_y }; }
            *has_previous_element = true;
            *was_last_element_trait = current_is_trait;
        };

        if !spirit.traits.is_empty() { final_panel_height += icon_size; has_previous_section = true; last_section_was_trait = true; }
        if !spirit.h1.is_empty() { add_gap(&mut final_panel_height, &mut has_previous_section, false, &mut last_section_was_trait); final_panel_height += icon_size; }
        if !spirit.h2.is_empty() { add_gap(&mut final_panel_height, &mut has_previous_section, false, &mut last_section_was_trait); final_panel_height += icon_size; }

        if !spirit.b1.is_empty() || !spirit.b2.is_empty() {
            add_gap(&mut final_panel_height, &mut has_previous_section, false, &mut last_section_was_trait);
            let calc_list_height = |items: &[AbilityItem]| -> i32 {
                let gaps = (items.len() as i32 - 1).max(0) * icon_gap_y;

                items.iter().map(|list_item| row_height(&list_item.text)).sum::<i32>() + gaps
            };
            if !spirit.b1.is_empty() { final_panel_height += calc_list_height(&spirit.b1); }
            if !spirit.b1.is_empty() && !spirit.b2.is_empty() { final_panel_height += icon_gap_y; }
            if !spirit.b2.is_empty() { final_panel_height += calc_list_height(&spirit.b2); }
        }

        if !spirit.footer.is_empty() { add_gap(&mut final_panel_height, &mut has_previous_section, false, &mut last_section_was_trait); final_panel_height += icon_size; }
        final_panel_height += card_padding;

        let spirit_rect = Rect::at(padding, card_inner_y).of_size(spirit_panel_width as u32, final_panel_height as u32);
        draw_bottom_rounded_rect_mut(canvas_image, spirit_rect, px(RADIUS_LG), COLOR_CARD);

        let mut current_y_offset = card_inner_y + card_padding;
        let area_item = AbilityItem { identity: Identity::AreaAttack, icon_id: Some(img015::ICON_AREA_ATTACK), border_id: None, custom_icon: CustomIcon::None, text: String::new() };
        let area_icon = get_icon_image(&area_item, layers, &custom_assets, icon_size as u32);
        let area_icon_y = current_y_offset + overflow_padding(&spirit.dmg_text);
        image::imageops::overlay(canvas_image, &area_icon, start_x_absolute as i64, area_icon_y as i64);

        draw_text_block(canvas_image, &spirit.dmg_text, start_x_absolute, area_icon_y);
        current_y_offset += row_height(&spirit.dmg_text) + icon_gap_y;

        has_previous_section = false;
        last_section_was_trait = false;

        if !spirit.traits.is_empty() { current_y_offset = draw_spirit_icons(canvas_image, &spirit.traits, current_y_offset, start_x_absolute); has_previous_section = true; last_section_was_trait = true; }
        if !spirit.h1.is_empty() { add_gap(&mut current_y_offset, &mut has_previous_section, false, &mut last_section_was_trait); current_y_offset = draw_spirit_icons(canvas_image, &spirit.h1, current_y_offset, start_x_absolute); }
        if !spirit.h2.is_empty() { add_gap(&mut current_y_offset, &mut has_previous_section, false, &mut last_section_was_trait); current_y_offset = draw_spirit_icons(canvas_image, &spirit.h2, current_y_offset, start_x_absolute); }

        if !spirit.b1.is_empty() || !spirit.b2.is_empty() {
            add_gap(&mut current_y_offset, &mut has_previous_section, false, &mut last_section_was_trait);
            if !spirit.b1.is_empty() { current_y_offset = draw_spirit_list(canvas_image, &spirit.b1, current_y_offset, start_x_absolute); }
            if !spirit.b1.is_empty() && !spirit.b2.is_empty() { current_y_offset += icon_gap_y; }
            if !spirit.b2.is_empty() { current_y_offset = draw_spirit_list(canvas_image, &spirit.b2, current_y_offset, start_x_absolute); }
        }

        if !spirit.footer.is_empty() { add_gap(&mut current_y_offset, &mut has_previous_section, false, &mut last_section_was_trait); draw_spirit_icons(canvas_image, &spirit.footer, current_y_offset, start_x_absolute); }

        card_inner_y + final_panel_height
    };

    let draw_list = |canvas_image: &mut RgbaImage, items: &Vec<AbilityItem>, start_y: i32| -> i32 {
        if items.is_empty() { return start_y; }
        let mut current_y = start_y;
        for (index, item) in items.iter().enumerate() {
            let icon_surface = get_icon_image(item, layers, &custom_assets, icon_size as u32);
            let icon_y = current_y + overflow_padding(&item.text);
            image::imageops::overlay(canvas_image, &icon_surface, padding as i64, icon_y as i64);

            draw_text_block(canvas_image, &item.text, padding, icon_y);
            current_y += row_height(&item.text);

            if item.icon_id == Some(img015::ICON_CONJURE)
                && let Some(spirit) = &data.spirit_data {
                current_y = draw_spirit_card(canvas_image, spirit, current_y);
            }

            if index < items.len() - 1 { current_y += icon_gap_y; }
        }
        current_y
    };

    let mut previously_drew_section = false;
    let mut last_main_section_was_trait = false;

    let draw_section_gap = |current_y: &mut i32, has_previous_element: &mut bool, current_is_trait: bool, was_last_element_trait: &mut bool| {
        if *has_previous_element { *current_y += if *was_last_element_trait { trait_gap_y } else { icon_gap_y }; }
        *has_previous_element = true;
        *was_last_element_trait = current_is_trait;
    };

    if !data.traits.is_empty() {
        current_y_global = draw_icon_row(&mut target_image, &data.traits, current_y_global, padding);
        previously_drew_section = true;
        last_main_section_was_trait = true;
    }
    if !data.h1.is_empty() {
        draw_section_gap(&mut current_y_global, &mut previously_drew_section, false, &mut last_main_section_was_trait);
        current_y_global = draw_icon_row(&mut target_image, &data.h1, current_y_global, padding);
    }
    if !data.h2.is_empty() {
        draw_section_gap(&mut current_y_global, &mut previously_drew_section, false, &mut last_main_section_was_trait);
        current_y_global = draw_icon_row(&mut target_image, &data.h2, current_y_global, padding);
    }
    if !data.b1.is_empty() || !data.b2.is_empty() {
        draw_section_gap(&mut current_y_global, &mut previously_drew_section, false, &mut last_main_section_was_trait);
        if !data.b1.is_empty() { current_y_global = draw_list(&mut target_image, &data.b1, current_y_global); }
        if !data.b1.is_empty() && !data.b2.is_empty() { current_y_global += icon_gap_y; }
        if !data.b2.is_empty() { current_y_global = draw_list(&mut target_image, &data.b2, current_y_global); }
    }
    if !data.footer.is_empty() {
        draw_section_gap(&mut current_y_global, &mut previously_drew_section, false, &mut last_main_section_was_trait);
        current_y_global = draw_icon_row(&mut target_image, &data.footer, current_y_global, padding);
    }

    let final_height = current_y_global + padding;
    let final_cropped = image::imageops::crop_imm(&target_image, 0, 0, canvas_width as u32, final_height as u32).to_image();

    let border_thick = px(FRAME_THICKNESS);
    let margin = border_thick + px(FRAME_PADDING);

    let final_width_with_pad = canvas_width as u32 + (margin * 2) as u32;
    let final_height_with_pad = final_height as u32 + (margin * 2) as u32;
    let mut final_background_layer = RgbaImage::new(final_width_with_pad, final_height_with_pad);

    let border_radius = px(FRAME_RADIUS);
    let outer_rect = Rect::at(0, 0).of_size(final_width_with_pad, final_height_with_pad);

    if border_thick > 0 {
        draw_bordered_rect_mut(
            &mut final_background_layer,
            CellShape::new(outer_rect, border_radius + border_thick, border_thick),
            COLOR_BACKGROUND, COLOR_FRAME,
        );
    } else {
        draw_rounded_rect_mut(&mut final_background_layer, outer_rect, border_radius, COLOR_BACKGROUND);
    }

    image::imageops::overlay(&mut final_background_layer, &final_cropped, margin as i64, margin as i64);

    Ok(final_background_layer)
}

fn frame_text(frames: i32) -> String {
    format!("{:.2}s^{}f", frames as f32 / 30.0, frames)
}

pub fn save_to_disk(image: &RgbaImage, is_cat: bool, id_str: &str, top_value: &str) -> Result<PathBuf, String> {
    let export_dir = Path::new("exports");
    fs::create_dir_all(export_dir).map_err(|err| err.to_string())?;

    let safe_val_str = top_value.replace(|c: char| !c.is_alphanumeric() && c != '+', "");
    let prefix = if is_cat { "Lv" } else { "Mag" };
    let filename = export_dir.join(format!("{}.{}{}.statblock.png", id_str, prefix, safe_val_str));

    image.save(&filename).map_err(|err| err.to_string())?;
    Ok(filename)
}
