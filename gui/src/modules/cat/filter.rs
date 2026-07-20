use iced::alignment::Vertical;
use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input, Space};
use iced::{Border, Color, Element, Length, Theme};

use core::modules::cat::filter::{CatFilterState, MatchMode, TalentFilterMode};
use core::modules::cat::game::registry::AbilityIcon;

const STAT_KEYS: [&str; 9] = [
    "Attack", "Dps", "Range", "Atk Cycle (f)", "Hitpoints", "Knockbacks", "Speed", "Cooldown (f)", "Cost",
];

#[derive(Clone)]
pub enum Message {
    Toggle,
    Clear,
    MatchModeChanged(MatchMode),
    TalentModeChanged(TalentFilterMode),
    UltraTalentModeChanged(TalentFilterMode),
    RarityToggled(usize),
    FormToggled(usize),
    IconToggled(AbilityIcon),
    LevelInputChanged(String),
    StatMinChanged(&'static str, String),
    StatMaxChanged(&'static str, String),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Toggle => write!(f, "Toggle"),
            Self::Clear => write!(f, "Clear"),
            Self::MatchModeChanged(_) => write!(f, "MatchModeChanged"),
            Self::TalentModeChanged(_) => write!(f, "TalentModeChanged"),
            Self::UltraTalentModeChanged(_) => write!(f, "UltraTalentModeChanged"),
            Self::RarityToggled(i) => write!(f, "RarityToggled({})", i),
            Self::FormToggled(i) => write!(f, "FormToggled({})", i),
            Self::IconToggled(_) => write!(f, "IconToggled"),
            Self::LevelInputChanged(s) => write!(f, "LevelInputChanged({})", s),
            Self::StatMinChanged(stat, v) => write!(f, "StatMinChanged({}, {})", stat, v),
            Self::StatMaxChanged(stat, v) => write!(f, "StatMaxChanged({}, {})", stat, v),
        }
    }
}

pub struct State {
    pub filter_state: CatFilterState,
}

impl Default for State {
    fn default() -> Self {
        Self { filter_state: CatFilterState::default() }
    }
}

impl State {
    pub fn update(&mut self, message: Message) {
        match message {
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
            Message::LevelInputChanged(input) => self.filter_state.level_input = input,
            Message::StatMinChanged(stat, value) => {
                self.filter_state.stat_ranges.entry(stat).or_default().min = value;
            }
            Message::StatMaxChanged(stat, value) => {
                self.filter_state.stat_ranges.entry(stat).or_default().max = value;
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let title = text("Advanced Cat Filter").size(24);

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
            }),
            text("Talents:").align_y(Vertical::Center),
            pick_list(vec!["Ignore", "Consider", "Only"], Some(self.filter_state.talent_mode.label()), |s| {
                Message::TalentModeChanged(talent_mode_from_label(&s))
            }),
            text("Ultra Talents:").align_y(Vertical::Center),
            pick_list(vec!["Ignore", "Consider", "Only"], Some(self.filter_state.ultra_talent_mode.label()), |s| {
                Message::UltraTalentModeChanged(talent_mode_from_label(&s))
            }),
        ].spacing(8).align_y(Vertical::Center);

        let level_row = row![
            text("Target Level:").align_y(Vertical::Center),
            text_input("Any", &self.filter_state.level_input)
                .on_input(Message::LevelInputChanged)
                .width(Length::Fixed(60.0)),
        ].spacing(8).align_y(Vertical::Center);

        let mut stats_col = column![].spacing(6);
        for pair in STAT_KEYS.chunks(2) {
            let mut stat_row = row![].spacing(16);
            for &stat in pair {
                stat_row = stat_row.push(stat_range_field(stat, &self.filter_state));
            }
            stats_col = stats_col.push(stat_row);
        }

        let icon_placeholder = text("Traits, Attack Type, Abilities & Talents icon filters are pending sprite rendering support.");

        let clear_btn = button(text("Clear Filter").style(text::danger))
            .on_press(Message::Clear)
            .padding([8, 16]);

        let close_btn = button("Close")
            .on_press(Message::Toggle)
            .padding([8, 16]);

        let content = column![
            title,
            Space::new().height(Length::Fixed(12.0)),
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
            text("Target Traits / Attack Type / Abilities").size(18),
            icon_placeholder,
            Space::new().height(Length::Fixed(24.0)),
            row![clear_btn, close_btn].spacing(16),
        ].spacing(8).padding(24);

        container(scrollable(content))
            .width(Length::Fixed(600.0))
            .height(Length::Fixed(500.0))
            .style(container::bordered_box)
            .into()
    }
}

fn toggle_button<'a>(label: &'a str, active: bool, on_press: Message) -> Element<'a, Message> {
    button(text(label))
        .on_press(on_press)
        .style(move |theme: &Theme, status| {
            let palette = theme.palette();
            let background = if active {
                palette.primary
            } else if status == button::Status::Hovered {
                Color { a: 0.15, ..palette.text }
            } else {
                Color::TRANSPARENT
            };

            button::Style {
                background: Some(background.into()),
                text_color: if active { palette.background } else { palette.text },
                border: Border::default().rounded(4.0),
                ..Default::default()
            }
        })
        .into()
}

fn stat_range_field<'a>(stat: &'static str, filter_state: &'a CatFilterState) -> Element<'a, Message> {
    const EMPTY: &str = "";
    let range = filter_state.stat_ranges.get(stat);
    let min_str: &str = range.map(|r| r.min.as_str()).unwrap_or(EMPTY);
    let max_str: &str = range.map(|r| r.max.as_str()).unwrap_or(EMPTY);

    row![
        text(format!("{}:", stat)).width(Length::Fixed(110.0)),
        text_input("Any", min_str).on_input(move |v| Message::StatMinChanged(stat, v)).width(Length::Fixed(55.0)),
        text("~"),
        text_input("Any", max_str).on_input(move |v| Message::StatMaxChanged(stat, v)).width(Length::Fixed(55.0)),
    ].spacing(4).align_y(Vertical::Center).into()
}

fn talent_mode_from_label(label: &str) -> TalentFilterMode {
    match label {
        "Consider" => TalentFilterMode::Consider,
        "Only" => TalentFilterMode::Only,
        _ => TalentFilterMode::Ignore,
    }
}
