use std::collections::HashSet;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, image as iced_image, pick_list, row, scrollable, stack, text, text_input, tooltip, Space};
use iced::{Element, Length, Size, Theme};
use nyanko::cat::abilities::{AttrUnit, REGISTRY};

use core::common::game::CustomIcon;
use core::modules::cat::filter::{icons, ATTACK_TYPE_ICONS, CatFilterState, MatchMode, TalentFilterMode};
use core::modules::cat::game::registry::{get_display_def, AbilityIcon, DisplayGroup};

use crate::app::theme;
use crate::common::ability_icon;
use crate::common::{CustomAssets, SpriteSheet};
use crate::widget::popup;
use crate::widget::range_row;
use crate::widget::{fallback_icon, smooth_scroll, ICON_SIZE};

const STAT_KEYS: [&str; 9] = [
    "Attack", "Dps", "Range", "Atk Cycle (f)", "Hitpoints", "Knockbacks", "Speed", "Cooldown (f)", "Cost",
];

const ICONS_PER_ROW: usize = 11;
const CLEAR_BTN_CLEARANCE: f32 = 56.0;

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Toggle,
    Clear,
    MatchModeChanged(MatchMode),
    TalentModeChanged(TalentFilterMode),
    UltraTalentModeChanged(TalentFilterMode),
    RarityToggled(usize),
    FormToggled(usize),
    IconToggled(AbilityIcon),
    AdvMinChanged(AbilityIcon, &'static str, String),
    AdvMaxChanged(AbilityIcon, &'static str, String),
    LevelInputChanged(String),
    StatMinChanged(&'static str, String),
    StatMaxChanged(&'static str, String),
}

#[derive(Default)]
pub struct State {
    pub filter_state: CatFilterState,
    popup: popup::State,
    icons: ability_icon::Cache,
}


impl State {
    pub(super) fn clear_icons(&self) {
        self.icons.clear();
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Popup(msg) => {
                if self.popup.update(msg, popup::Kind::CatFilter) {
                    self.filter_state.is_open = false;
                }
            }
            Message::Toggle => self.filter_state.is_open = !self.filter_state.is_open,
            Message::Clear => {
                self.filter_state = CatFilterState { is_open: self.filter_state.is_open, ..Default::default() };
            }
            Message::MatchModeChanged(mode) => self.filter_state.match_mode = mode,
            Message::TalentModeChanged(mode) => self.filter_state.talent_mode = mode,
            Message::UltraTalentModeChanged(mode) => self.filter_state.ultra_talent_mode = mode,
            Message::RarityToggled(index) => {
                if let Some(active) = self.filter_state.rarities.get_mut(index) {
                    *active = !*active;
                }
            }
            Message::FormToggled(index) => {
                if let Some(active) = self.filter_state.forms.get_mut(index) {
                    *active = !*active;
                }
            }
            Message::IconToggled(icon) => {
                if self.filter_state.active_icons.contains(&icon) {
                    self.filter_state.active_icons.remove(&icon);
                } else {
                    self.filter_state.active_icons.insert(icon);
                }
            }
            Message::AdvMinChanged(icon, attr, value) => {
                self.filter_state.adv_ranges.entry(icon).or_default().entry(attr).or_default().min = value;
            }
            Message::AdvMaxChanged(icon, attr, value) => {
                self.filter_state.adv_ranges.entry(icon).or_default().entry(attr).or_default().max = value;
            }
            Message::LevelInputChanged(input) => self.filter_state.level_input = input,
            Message::StatMinChanged(stat, value) => {
                self.filter_state.stat_ranges.entry(stat).or_default().min = value;
            }
            Message::StatMaxChanged(stat, value) => {
                self.filter_state.stat_ranges.entry(stat).or_default().max = value;
            }
        }
    }

    pub fn view<'a>(&'a self, sheets: &'a [SpriteSheet], assets: &'a CustomAssets, window: Size) -> Element<'a, Message> {
        self.popup.view("Advanced Cat Filter", popup::Kind::CatFilter, window, Message::Popup, move || {
            self.content_view(sheets, assets)
        }, None)
    }

    fn content_view<'a>(&'a self, sheets: &'a [SpriteSheet], assets: &'a CustomAssets) -> Element<'a, Message> {
        let rarity_labels = ["Normal", "Special", "Rare", "Super Rare", "Uber Rare", "Legend Rare"];
        let mut rarity_row = row![].spacing(4);
        for (i, &label) in rarity_labels.iter().enumerate() {
            rarity_row = rarity_row.push(toggle_button(label, self.filter_state.rarities[i], Message::RarityToggled(i)));
        }

        let form_labels = ["Normal Form", "Evolved Form", "True Form", "Ultra Form"];
        let mut forms_row = row![].spacing(4);
        for (i, &label) in form_labels.iter().enumerate() {
            forms_row = forms_row.push(toggle_button(label, self.filter_state.forms[i], Message::FormToggled(i)));
        }

        let match_mode_label = if self.filter_state.match_mode == MatchMode::And { "And" } else { "Or" };

        let mode_row = row![
            text("Mode:").align_y(Vertical::Center),
            pick_list(vec!["And", "Or"], Some(match_mode_label), |s| {
                Message::MatchModeChanged(if s == "And" { MatchMode::And } else { MatchMode::Or })
            }).style(theme::combo_box).menu_style(theme::combo_box_menu),
            text("Talents:").align_y(Vertical::Center),
            pick_list(vec!["Ignore", "Consider", "Only"], Some(self.filter_state.talent_mode.label()), |s| {
                Message::TalentModeChanged(talent_mode_from_label(s))
            }).style(theme::combo_box).menu_style(theme::combo_box_menu),
            text("Ultra Talents:").align_y(Vertical::Center),
            pick_list(vec!["Ignore", "Consider", "Only"], Some(self.filter_state.ultra_talent_mode.label()), |s| {
                Message::UltraTalentModeChanged(talent_mode_from_label(s))
            }).style(theme::combo_box).menu_style(theme::combo_box_menu),
        ].spacing(8).align_y(Vertical::Center);

        let level_row = row![
            text("Target Level:").align_y(Vertical::Center),
            text_input("Any", &self.filter_state.level_input)
                .on_input(Message::LevelInputChanged)
                .width(Length::Fixed(60.0))
                .style(theme::rounded_input),
        ].spacing(8).align_y(Vertical::Center);

        let mut stats_col = column![].spacing(6);
        for pair in STAT_KEYS.chunks(2) {
            let mut stat_row = row![].spacing(16);
            for &stat in pair {
                stat_row = stat_row.push(stat_range_field(stat, &self.filter_state));
            }
            stats_col = stats_col.push(stat_row);
        }

        let trait_icons: Vec<AbilityIcon> = REGISTRY.iter()
            .map(|def| get_display_def(def.identity))
            .filter(|display_def| display_def.group == DisplayGroup::Trait)
            .map(|display_def| display_def.icon)
            .collect();
        let traits_row = self.icon_wrap(trait_icons.into_iter(), sheets, assets);

        let attack_row = self.icon_wrap(ATTACK_TYPE_ICONS.iter().copied(), sheets, assets);

        let mut rendered_icons: HashSet<AbilityIcon> = HashSet::new();
        let mut abilities_col = column![].spacing(0);

        for group in [DisplayGroup::Headline1, DisplayGroup::Headline2] {
            let group_icons = collect_group_icons(group, &mut rendered_icons);
            if !group_icons.is_empty() {
                abilities_col = abilities_col.push(self.icon_wrap(group_icons.into_iter(), sheets, assets));
                abilities_col = abilities_col.push(Space::new().height(Length::Fixed(8.0)));
            }
        }

        for group in [DisplayGroup::Body1, DisplayGroup::Body2] {
            let group_icons = collect_group_icons(group, &mut rendered_icons);
            if !group_icons.is_empty() {
                let mut col = column![].spacing(4);
                for icon in group_icons {
                    col = col.push(self.icon_row_with_label(icon, sheets, assets));
                }
                abilities_col = abilities_col.push(col);
                abilities_col = abilities_col.push(Space::new().height(Length::Fixed(8.0)));
            }
        }

        let footer_icons = collect_group_icons(DisplayGroup::Footer, &mut rendered_icons);
        if !footer_icons.is_empty() {
            abilities_col = abilities_col.push(self.icon_wrap(footer_icons.into_iter(), sheets, assets));
            abilities_col = abilities_col.push(Space::new().height(Length::Fixed(8.0)));
        }

        let check_talents = self.filter_state.talent_mode != TalentFilterMode::Ignore
            || self.filter_state.ultra_talent_mode != TalentFilterMode::Ignore;

        if check_talents {
            let mut talent_icons: Vec<AbilityIcon> = Vec::new();
            for def in REGISTRY.iter() {
                let display_def = get_display_def(def.identity);
                if display_def.group == DisplayGroup::Trait { continue; }
                if rendered_icons.contains(&display_def.icon) { continue; }
                if ATTACK_TYPE_ICONS.contains(&display_def.icon) { continue; }
                if talent_icons.contains(&display_def.icon) { continue; }
                talent_icons.push(display_def.icon);
            }

            if !talent_icons.is_empty() {
                abilities_col = abilities_col.push(text("Talents").size(18));
                abilities_col = abilities_col.push(Space::new().height(Length::Fixed(5.0)));
                abilities_col = abilities_col.push(self.icon_wrap(talent_icons.into_iter(), sheets, assets));
            }
        }

        let content = column![
            text("Attributes").size(18),
            rarity_row,
            forms_row,
            Space::new().height(Length::Fixed(8.0)),
            mode_row,
            Space::new().height(Length::Fixed(16.0)),
            text("Stats").size(18),
            level_row,
            stats_col,
            Space::new().height(Length::Fixed(16.0)),
            text("Target Traits").size(18),
            traits_row,
            Space::new().height(Length::Fixed(16.0)),
            text("Attack Type").size(18),
            attack_row,
            Space::new().height(Length::Fixed(16.0)),
            text("Abilities").size(18),
            abilities_col,
            Space::new().height(Length::Fixed(CLEAR_BTN_CLEARANCE)),
        ].spacing(8).padding(24);

        let scroll_layer = smooth_scroll(scrollable(content).width(Length::Fill).height(Length::Fill));

        let clear_btn = button(text("Clear Filter"))
            .on_press(Message::Clear)
            .padding([8, 16])
            .style(button::danger);

        let clear_btn_layer = container(clear_btn)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Bottom)
            .padding(16);

        stack![scroll_layer, clear_btn_layer]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn icon_wrap<'a>(
        &'a self,
        icons: impl Iterator<Item = AbilityIcon>,
        sheets: &'a [SpriteSheet],
        assets: &'a CustomAssets,
    ) -> Element<'a, Message> {
        let items: Vec<AbilityIcon> = icons.collect();
        let mut col = column![].spacing(4);
        for chunk in items.chunks(ICONS_PER_ROW) {
            let mut wrapped_row = row![].spacing(4).align_y(Vertical::Center);
            for &icon in chunk {
                wrapped_row = wrapped_row.push(self.icon_with_tooltip(icon, sheets, assets));
            }
            col = col.push(wrapped_row);
        }
        col.into()
    }

    fn icon_with_tooltip<'a>(&'a self, icon: AbilityIcon, sheets: &'a [SpriteSheet], assets: &'a CustomAssets) -> Element<'a, Message> {
        let is_active = self.filter_state.active_icons.contains(&icon);
        let name = icons::get_icon_name(&icon);

        tooltip(
            self.icon_button(icon, sheets, assets, is_active),
            container(text(name)).padding(6).style(container::bordered_box),
            tooltip::Position::Top,
        ).into()
    }

    fn icon_row_with_label<'a>(&'a self, icon: AbilityIcon, sheets: &'a [SpriteSheet], assets: &'a CustomAssets) -> Element<'a, Message> {
        let is_active = self.filter_state.active_icons.contains(&icon);
        let name = icons::get_icon_name(&icon);
        let schema = ability_schema(icon);
        let expanded = is_active && !schema.is_empty();

        let label_btn = button(text(name))
            .padding(0)
            .style(button::text)
            .on_press(Message::IconToggled(icon));

        let header = row![
            self.icon_button(icon, sheets, assets, is_active),
            label_btn,
        ].spacing(10).align_y(Vertical::Center);

        if !expanded {
            return header.into();
        }

        let mut grid_col = column![].spacing(6);
        for &(attr, _) in schema {
            grid_col = grid_col.push(self.adv_range_row(icon, attr));
        }

        container(column![header, grid_col].spacing(6))
            .padding(8)
            .style(theme::card_container)
            .into()
    }

    fn adv_range_row<'a>(&'a self, icon: AbilityIcon, attr: &'static str) -> Element<'a, Message> {
        let range = self.filter_state.adv_ranges.get(&icon).and_then(|ranges| ranges.get(attr));
        let min = range.map_or("", |r| r.min.as_str());
        let max = range.map_or("", |r| r.max.as_str());

        range_row(
            attr,
            min,
            max,
            move |v| Message::AdvMinChanged(icon, attr, v),
            move |v| Message::AdvMaxChanged(icon, attr, v),
        )
    }

    fn icon_button<'a>(&'a self, icon: AbilityIcon, sheets: &'a [SpriteSheet], assets: &'a CustomAssets, is_active: bool) -> Element<'a, Message> {
        let icon_el = self.icon_image(icon, sheets, assets, is_active);
        button(icon_el)
            .padding(0)
            .style(button::text)
            .on_press(Message::IconToggled(icon))
            .into()
    }

    fn icon_image<'a>(&'a self, icon: AbilityIcon, sheets: &'a [SpriteSheet], assets: &'a CustomAssets, is_active: bool) -> Element<'a, Message> {
        let opacity: f32 = if is_active { 1.0 } else { 0.4 };

        match icon {
            AbilityIcon::Custom(custom_icon) => {
                if let Some(handle) = assets.get_icon_texture(custom_icon) {
                    return iced_image(handle).width(Length::Fixed(ICON_SIZE)).height(Length::Fixed(ICON_SIZE)).opacity(opacity).into();
                }
            }
            AbilityIcon::Standard(icon_id) => {
                if let Some(handle) = self.icons.handle(icon_id, sheets) {
                    return iced_image(handle).width(Length::Fixed(ICON_SIZE)).height(Length::Fixed(ICON_SIZE)).opacity(opacity).into();
                }
            }
            AbilityIcon::None => {}
        }

        fallback_icon("?")
    }

}

fn collect_group_icons(target_group: DisplayGroup, rendered_icons: &mut HashSet<AbilityIcon>) -> Vec<AbilityIcon> {
    let mut icons_in_group = Vec::new();

    for def in REGISTRY.iter() {
        let display_def = get_display_def(def.identity);
        if display_def.group != target_group { continue; }
        if display_def.group == DisplayGroup::Trait { continue; }
        if ATTACK_TYPE_ICONS.contains(&display_def.icon) { continue; }
        if icons_in_group.contains(&display_def.icon) { continue; }

        icons_in_group.push(display_def.icon);
        rendered_icons.insert(display_def.icon);
    }

    if target_group == DisplayGroup::Headline2 {
        let kamikaze = AbilityIcon::Custom(CustomIcon::Kamikaze);
        if !icons_in_group.contains(&kamikaze) {
            icons_in_group.push(kamikaze);
            rendered_icons.insert(kamikaze);
        }
    }

    icons_in_group
}

fn ability_schema(icon: AbilityIcon) -> &'static [(&'static str, AttrUnit)] {
    REGISTRY.iter()
        .find(|def| get_display_def(def.identity).icon == icon)
        .map(|def| def.schema)
        .unwrap_or(&[])
}

fn toggle_button<'a>(label: &'a str, active: bool, on_press: Message) -> Element<'a, Message> {
    button(text(label))
        .on_press(on_press)
        .style(move |t: &Theme, status| theme::toggle_button(t, status, active))
        .into()
}

fn stat_range_field<'a>(stat: &'static str, filter_state: &'a CatFilterState) -> Element<'a, Message> {
    let range = filter_state.stat_ranges.get(stat);
    let min = range.map_or("", |r| r.min.as_str());
    let max = range.map_or("", |r| r.max.as_str());

    range_row(
        stat,
        min,
        max,
        move |v| Message::StatMinChanged(stat, v),
        move |v| Message::StatMaxChanged(stat, v),
    )
}

fn talent_mode_from_label(label: &str) -> TalentFilterMode {
    match label {
        "Consider" => TalentFilterMode::Consider,
        "Only" => TalentFilterMode::Only,
        _ => TalentFilterMode::Ignore,
    }
}
