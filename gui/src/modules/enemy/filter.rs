use std::collections::HashSet;

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, image as iced_image, pick_list, row, scrollable, stack, text, text_input, tooltip, Space};
use iced::{Element, Length, Size};
use nyanko::enemy::abilities::{AttrUnit, Identity, REGISTRY};

use core::modules::enemy::filter::evaluation::get_identity_name;
use core::modules::enemy::filter::{EnemyFilterState, MatchMode, ATTACK_TYPE_IDENTITIES};
use core::modules::enemy::game::registry::{get_display_def, AbilityIcon, DisplayGroup};

use crate::app::theme;
use crate::common::ability_icon;
use crate::common::{CustomAssets, SpriteSheet};
use crate::widget::popup;
use crate::widget::range_row;
use crate::widget::{fallback_icon, smooth_scroll, ICON_SIZE};

const STAT_KEYS: [&str; 8] = [
    "Attack", "Dps", "Range", "Atk Cycle (f)", "Hitpoints", "Knockbacks", "Speed", "Cash Drop",
];

const ICONS_PER_ROW: usize = 11;
const CLEAR_BTN_CLEARANCE: f32 = 56.0;

#[derive(Debug, Clone)]
pub enum Message {
    Popup(popup::Message),
    Toggle,
    Clear,
    MatchModeChanged(MatchMode),
    MagChanged(String),
    StatMinChanged(&'static str, String),
    StatMaxChanged(&'static str, String),
    IdentityToggled(Identity),
    AdvMinChanged(Identity, &'static str, String),
    AdvMaxChanged(Identity, &'static str, String),
}

#[derive(Default)]
pub struct State {
    pub filter_state: EnemyFilterState,
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
                if self.popup.update(msg, popup::Kind::EnemyFilter) {
                    self.filter_state.is_open = false;
                }
            }
            Message::Toggle => self.filter_state.is_open = !self.filter_state.is_open,
            Message::Clear => {
                self.filter_state = EnemyFilterState { is_open: self.filter_state.is_open, ..Default::default() };
            }
            Message::MatchModeChanged(mode) => self.filter_state.match_mode = mode,
            Message::MagChanged(mag) => self.filter_state.mag_input = mag,
            Message::StatMinChanged(stat, value) => {
                self.filter_state.stat_ranges.entry(stat).or_default().min = value;
            }
            Message::StatMaxChanged(stat, value) => {
                self.filter_state.stat_ranges.entry(stat).or_default().max = value;
            }
            Message::IdentityToggled(identity) => {
                if self.filter_state.active_identities.contains(&identity) {
                    self.filter_state.active_identities.remove(&identity);
                } else {
                    self.filter_state.active_identities.insert(identity);
                }
            }
            Message::AdvMinChanged(identity, attr, val) => {
                self.filter_state.adv_ranges.entry(identity).or_default().entry(attr).or_default().min = val;
            }
            Message::AdvMaxChanged(identity, attr, val) => {
                self.filter_state.adv_ranges.entry(identity).or_default().entry(attr).or_default().max = val;
            }
        }
    }

    pub fn view<'a>(&'a self, sheets: &'a [SpriteSheet], assets: &'a CustomAssets, window: Size) -> Element<'a, Message> {
        self.popup.view("Advanced Enemy Filter", popup::Kind::EnemyFilter, window, Message::Popup, move || {
            self.content_view(sheets, assets)
        }, None)
    }

    fn content_view<'a>(&'a self, sheets: &'a [SpriteSheet], assets: &'a CustomAssets) -> Element<'a, Message> {
        let match_mode_label = if self.filter_state.match_mode == MatchMode::And { "And" } else { "Or" };

        let mode_row = row![
            text("Mode:").align_y(Vertical::Center),
            pick_list(vec!["And", "Or"], Some(match_mode_label), |s| {
                Message::MatchModeChanged(if s == "And" { MatchMode::And } else { MatchMode::Or })
            }).style(theme::combo_box).menu_style(theme::combo_box_menu),
        ].spacing(8).align_y(Vertical::Center);

        let mag_row = row![
            text("Target Magnification:").align_y(Vertical::Center),
            text_input("100", &self.filter_state.mag_input)
                .on_input(Message::MagChanged)
                .width(Length::Fixed(60.0))
                .style(theme::rounded_input),
            text("%"),
        ].spacing(8).align_y(Vertical::Center);

        let mut stats_col = column![].spacing(6);
        for pair in STAT_KEYS.chunks(2) {
            let mut stat_row = row![].spacing(16);
            for &stat in pair {
                stat_row = stat_row.push(stat_range_field(stat, &self.filter_state));
            }
            stats_col = stats_col.push(stat_row);
        }

        let type_identities: Vec<Identity> = REGISTRY.iter()
            .map(|def| def.identity)
            .filter(|&identity| get_display_def(identity).group == DisplayGroup::Type)
            .collect();
        let type_row = self.icon_wrap(type_identities.into_iter(), sheets, assets);

        let attack_row = self.icon_wrap(ATTACK_TYPE_IDENTITIES.iter().copied(), sheets, assets);

        let mut rendered_identities: HashSet<Identity> = HashSet::new();
        let mut abilities_col = column![].spacing(0);

        for group in [DisplayGroup::Headline1, DisplayGroup::Headline2] {
            let group_identities = collect_group_identities(group, &mut rendered_identities);
            if !group_identities.is_empty() {
                abilities_col = abilities_col.push(self.icon_wrap(group_identities.into_iter(), sheets, assets));
                abilities_col = abilities_col.push(Space::new().height(Length::Fixed(8.0)));
            }
        }

        for group in [DisplayGroup::Body1, DisplayGroup::Body2] {
            let group_identities = collect_group_identities(group, &mut rendered_identities);
            if !group_identities.is_empty() {
                let mut col = column![].spacing(4);
                for identity in group_identities {
                    col = col.push(self.icon_row_with_label(identity, sheets, assets));
                }
                abilities_col = abilities_col.push(col);
                abilities_col = abilities_col.push(Space::new().height(Length::Fixed(8.0)));
            }
        }

        let footer_identities = collect_group_identities(DisplayGroup::Footer, &mut rendered_identities);
        if !footer_identities.is_empty() {
            abilities_col = abilities_col.push(self.icon_wrap(footer_identities.into_iter(), sheets, assets));
        }

        let content = column![
            text("Attributes").size(18),
            mode_row,
            Space::new().height(Length::Fixed(16.0)),
            text("Stats").size(18),
            mag_row,
            stats_col,
            Space::new().height(Length::Fixed(16.0)),
            text("Trait Type").size(18),
            type_row,
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
        identities: impl Iterator<Item = Identity>,
        sheets: &'a [SpriteSheet],
        assets: &'a CustomAssets,
    ) -> Element<'a, Message> {
        let items: Vec<Identity> = identities.collect();
        let mut col = column![].spacing(4);
        for chunk in items.chunks(ICONS_PER_ROW) {
            let mut wrapped_row = row![].spacing(4).align_y(Vertical::Center);
            for &identity in chunk {
                wrapped_row = wrapped_row.push(self.icon_with_tooltip(identity, sheets, assets));
            }
            col = col.push(wrapped_row);
        }
        col.into()
    }

    fn icon_with_tooltip<'a>(&'a self, identity: Identity, sheets: &'a [SpriteSheet], assets: &'a CustomAssets) -> Element<'a, Message> {
        let is_active = self.filter_state.active_identities.contains(&identity);
        let name = get_identity_name(identity);

        tooltip(
            self.icon_button(identity, sheets, assets, is_active),
            container(text(name)).padding(6).style(container::bordered_box),
            tooltip::Position::Top,
        ).into()
    }

    fn icon_row_with_label<'a>(&'a self, identity: Identity, sheets: &'a [SpriteSheet], assets: &'a CustomAssets) -> Element<'a, Message> {
        let is_active = self.filter_state.active_identities.contains(&identity);
        let name = get_identity_name(identity);
        let schema = ability_schema(identity);
        let expanded = is_active && !schema.is_empty();

        let label_btn = button(text(name))
            .padding(0)
            .style(button::text)
            .on_press(Message::IdentityToggled(identity));

        let header = row![
            self.icon_button(identity, sheets, assets, is_active),
            label_btn,
        ].spacing(10).align_y(Vertical::Center);

        if !expanded {
            return header.into();
        }

        let mut grid_col = column![].spacing(6);
        for &(attr, _) in schema {
            grid_col = grid_col.push(self.adv_range_row(identity, attr));
        }

        container(column![header, grid_col].spacing(6))
            .padding(8)
            .style(theme::card_container)
            .into()
    }

    fn adv_range_row<'a>(&'a self, identity: Identity, attr: &'static str) -> Element<'a, Message> {
        let range = self.filter_state.adv_ranges.get(&identity).and_then(|ranges| ranges.get(attr));
        let min = range.map_or("", |r| r.min.as_str());
        let max = range.map_or("", |r| r.max.as_str());

        range_row(
            attr,
            min,
            max,
            move |v| Message::AdvMinChanged(identity, attr, v),
            move |v| Message::AdvMaxChanged(identity, attr, v),
        )
    }

    fn icon_button<'a>(&'a self, identity: Identity, sheets: &'a [SpriteSheet], assets: &'a CustomAssets, is_active: bool) -> Element<'a, Message> {
        let icon_el = self.icon_image(identity, sheets, assets, is_active);
        button(icon_el)
            .padding(0)
            .style(button::text)
            .on_press(Message::IdentityToggled(identity))
            .into()
    }

    fn icon_image<'a>(&'a self, identity: Identity, sheets: &'a [SpriteSheet], assets: &'a CustomAssets, is_active: bool) -> Element<'a, Message> {
        let opacity: f32 = if is_active { 1.0 } else { 0.4 };
        let display_def = get_display_def(identity);

        match display_def.icon {
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

fn collect_group_identities(target_group: DisplayGroup, rendered_identities: &mut HashSet<Identity>) -> Vec<Identity> {
    let mut identities_in_group = Vec::new();

    for def in REGISTRY.iter() {
        let display_def = get_display_def(def.identity);
        if display_def.group != target_group { continue; }
        if display_def.group == DisplayGroup::Type { continue; }
        if ATTACK_TYPE_IDENTITIES.contains(&def.identity) { continue; }
        if identities_in_group.contains(&def.identity) { continue; }

        identities_in_group.push(def.identity);
        rendered_identities.insert(def.identity);
    }

    identities_in_group
}

fn ability_schema(identity: Identity) -> &'static [(&'static str, AttrUnit)] {
    REGISTRY.iter()
        .find(|def| def.identity == identity)
        .map(|def| def.schema)
        .unwrap_or(&[])
}

fn stat_range_field<'a>(stat: &'static str, filter_state: &'a EnemyFilterState) -> Element<'a, Message> {
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
