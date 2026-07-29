use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use iced::widget::image::Handle;
use iced::widget::{button, column, container, image as iced_image, row, scrollable, slider, text, text_input, Space};
use iced::{font, Alignment, Color, Element, Length, Theme};
use nyanko::cat::abilities::get_talent;
use nyanko::cat::unit::{Battle, LevelCurve, Talent, TalentCost, TalentGroup};
use nyanko::common::data::img022;

use core::common::gfx::autocrop;
use core::modules::cat::game::registry::{get_display_def, AbilityIcon};
use core::modules::cat::game::talents as talent_logic;
use core::modules::cat::paths;
use core::modules::settings::Settings;

use crate::common::shared::fallback_icon;
use crate::common::{CustomAssets, SpriteSheet};

const GROUP_ICON_SIZE: f32 = 40.0;
const NP_ICON_SIZE: f32 = 20.0;
const SECTION_SPACING: f32 = 2.0;

fn bold_text<'a>(content: impl ToString, size: f32) -> iced::widget::Text<'a> {
    text(content.to_string())
        .size(size)
        .font(font::Font { weight: font::Weight::Bold, ..Default::default() })
        .color(Color::WHITE)
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleGroup(u32, u8),
    LevelChanged(u8, u8),
    LevelInputChanged(u8, String),
}

pub struct ViewCtx<'a, 'b> {
    pub cat_id: u32,
    pub talent_data: &'a Talent,
    pub talent_levels: &'a HashMap<u8, u8>,
    pub level_inputs: &'a HashMap<u8, String>,
    pub talent_costs: &'a HashMap<u8, TalentCost>,
    pub descriptions: &'a [String],
    pub current_stats: Option<&'b Battle>,
    pub curve: Option<&'a LevelCurve>,
    pub unit_level: i32,
    pub sheets: &'a [SpriteSheet],
    pub img022_sheets: &'a [SpriteSheet],
    pub assets: &'a CustomAssets,
    pub settings: &'a Settings,
}

pub struct State {
    icon_cache: RefCell<HashMap<usize, Handle>>,
    skill_name_cache: RefCell<HashMap<String, Handle>>,
    expanded: HashMap<(u32, u8), bool>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            icon_cache: RefCell::new(HashMap::new()),
            skill_name_cache: RefCell::new(HashMap::new()),
            expanded: HashMap::new(),
        }
    }
}

impl State {
    pub fn update(&mut self, message: Message) {
        if let Message::ToggleGroup(cat_id, index) = message {
            let key = (cat_id, index);
            let current = self.expanded.get(&key).copied().unwrap_or(false);
            self.expanded.insert(key, !current);
        }
    }

    pub fn view<'a>(&'a self, ctx: ViewCtx<'a, '_>) -> Element<'a, Message> {
        let mut col = column![].spacing(8).width(Length::Fill);

        for (index, group) in ctx.talent_data.groups.iter().enumerate() {
            col = col.push(self.group_view(&ctx, index as u8, group));
        }

        scrollable(col).height(Length::Fill).width(Length::Fill).into()
    }

    fn group_view<'a>(&'a self, ctx: &ViewCtx<'a, '_>, index: u8, group: &'a TalentGroup) -> Element<'a, Message> {
        let expanded = self.expanded.get(&(ctx.cat_id, index)).copied().unwrap_or(false);
        let bg_color = if group.limit == 1 { Color::from_rgb8(120, 20, 20) } else { Color::from_rgb8(180, 140, 20) };

        let mut inner = column![self.group_header(ctx, index, group, expanded)].spacing(0);

        if expanded {
            inner = inner.push(self.group_body(ctx, index, group));
        }

        container(inner)
            .padding(6)
            .width(Length::Fill)
            .style(move |_theme: &Theme| container::Style {
                background: Some(bg_color.into()),
                border: iced::border::rounded(5),
                ..Default::default()
            })
            .into()
    }

    fn group_header<'a>(&'a self, ctx: &ViewCtx<'a, '_>, index: u8, group: &'a TalentGroup, expanded: bool) -> Element<'a, Message> {
        let icon = self.talent_icon(group, ctx.sheets, ctx.assets);

        let name_el: Element<Message> = match self.skill_name_handle(group, ctx.settings) {
            Some(handle) => iced_image(handle).into(),
            None => {
                let fallback_text = match get_talent(group.ability_id) {
                    Some(def) => get_display_def(def.identity).name.to_string(),
                    None => format!("Unknown Skill (ID: {})", group.ability_id),
                };
                bold_text(fallback_text, 18.0).into()
            }
        };

        let arrow = if expanded { "\u{25B2}" } else { "\u{25BC}" };

        let content = row![
            row![icon, name_el].spacing(8).align_y(Alignment::Center).width(Length::Fill),
            text(arrow).size(20)
        ]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        button(content)
            .on_press(Message::ToggleGroup(ctx.cat_id, index))
            .style(button::text)
            .width(Length::Fill)
            .into()
    }

    fn group_body<'a>(&'a self, ctx: &ViewCtx<'a, '_>, index: u8, group: &'a TalentGroup) -> Element<'a, Message> {
        let description_text = ctx.descriptions
            .get(group.text_id as usize)
            .cloned()
            .unwrap_or_else(|| "No skill description found".to_string());

        let description_box = dark_box(text(description_text).size(13).color(Color::WHITE).width(Length::Fill));

        let current_level = *ctx.talent_levels.get(&index).unwrap_or(&0);
        let np_cost = talent_logic::get_talent_np_cost(group.cost_id, current_level, ctx.talent_costs);

        let np_row = row![self.np_icon(ctx.img022_sheets), bold_text(np_cost, 18.0)]
            .spacing(4)
            .align_y(Alignment::Center);
        let np_box = dark_box(np_row);

        let max_level = group.max_level.max(1);
        let input_text = ctx.level_inputs.get(&index).cloned().unwrap_or_else(|| current_level.to_string());

        let level_row = row![
            bold_text("Level:", 14.0),
            slider(0..=max_level, current_level, move |value| Message::LevelChanged(index, value)).width(Length::Fixed(160.0)),
            text_input("0", &input_text)
                .on_input(move |value| Message::LevelInputChanged(index, value))
                .width(Length::Fixed(50.0)),
        ]
            .spacing(8)
            .align_y(Alignment::Center);

        let mut level_col = column![level_row].spacing(4);

        if let Some(stats) = ctx.current_stats
            && let Some(display_text) = talent_logic::calculate_talent_display(group, stats, current_level, ctx.curve, ctx.unit_level) {
            level_col = level_col.push(text(display_text).size(15).color(Color::WHITE).font(font::Font { weight: font::Weight::Bold, ..Default::default() }));
        }

        let level_box = dark_box(level_col);

        column![description_box, Space::new().height(Length::Fixed(SECTION_SPACING)), np_box, Space::new().height(Length::Fixed(SECTION_SPACING)), level_box]
            .padding([6, 0])
            .width(Length::Fill)
            .into()
    }

    fn talent_icon<'a>(&'a self, group: &TalentGroup, sheets: &'a [SpriteSheet], assets: &'a CustomAssets) -> Element<'a, Message> {
        let Some(def) = get_talent(group.ability_id) else {
            return text("?").size(24).into();
        };
        let display_def = get_display_def(def.identity);

        match display_def.icon {
            AbilityIcon::Custom(custom) => {
                if let Some(handle) = assets.get_icon_texture(custom) {
                    return iced_image(handle).width(Length::Fixed(GROUP_ICON_SIZE)).height(Length::Fixed(GROUP_ICON_SIZE)).into();
                }
            }
            AbilityIcon::Standard(icon_id) => {
                if let Some(handle) = self.icon_handle(icon_id, sheets) {
                    return iced_image(handle).width(Length::Fixed(GROUP_ICON_SIZE)).height(Length::Fixed(GROUP_ICON_SIZE)).into();
                }
            }
            AbilityIcon::None => {}
        }

        fallback_icon(display_def.fallback)
    }

    fn np_icon<'a>(&'a self, img022_sheets: &'a [SpriteSheet]) -> Element<'a, Message> {
        match self.icon_handle(img022::ICON_NP_COST, img022_sheets) {
            Some(handle) => iced_image(handle).height(Length::Fixed(NP_ICON_SIZE)).into(),
            None => bold_text("NP Cost", 18.0).into(),
        }
    }

    fn icon_handle(&self, icon_id: usize, sheets: &[SpriteSheet]) -> Option<Handle> {
        if let Some(cached) = self.icon_cache.borrow().get(&icon_id) {
            return Some(cached.clone());
        }

        for sheet in sheets {
            let Some(cut) = sheet.core.cuts_map.get(&icon_id) else { continue; };
            let Some(image_data) = &sheet.core.image_data else { continue; };

            let width = image_data.width();
            let height = image_data.height();

            let px = (cut.uv_coordinates.min.x * width as f32).round() as u32;
            let py = (cut.uv_coordinates.min.y * height as f32).round() as u32;
            let pw = cut.original_size.x.round() as u32;
            let ph = cut.original_size.y.round() as u32;

            if pw == 0 || ph == 0 || px + pw > width || py + ph > height {
                continue;
            }

            let cropped = image::imageops::crop_imm(image_data.as_ref(), px, py, pw, ph).to_image();
            let handle = Handle::from_rgba(pw, ph, cropped.into_raw());
            self.icon_cache.borrow_mut().insert(icon_id, handle.clone());
            return Some(handle);
        }

        None
    }

    fn skill_name_handle(&self, group: &TalentGroup, settings: &Settings) -> Option<Handle> {
        let image_id = if group.name_id > 0 { group.name_id } else { group.ability_id as i16 };
        if image_id <= 0 { return None; }

        let path = find_skill_image_path(image_id, settings)?;
        let file_name = path.file_name()?.to_string_lossy().to_string();

        if let Some(cached) = self.skill_name_cache.borrow().get(&file_name) {
            return Some(cached.clone());
        }

        let img = image::open(&path).ok()?;
        let rgba = autocrop(img.to_rgba8());
        let handle = Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw());
        self.skill_name_cache.borrow_mut().insert(file_name, handle.clone());
        Some(handle)
    }
}

fn dark_box<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding(4)
        .width(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.4).into()),
            border: iced::border::rounded(4),
            ..Default::default()
        })
        .into()
}

fn find_skill_image_path(image_id: i16, settings: &Settings) -> Option<PathBuf> {
    let dir = Path::new(paths::DIR_SKILL_NAME);
    let base_filename = format!("Skill_name_{:03}.png", image_id);
    core::common::get(dir, [base_filename.as_str()], &settings.general.language_priority).into_iter().next()
}
