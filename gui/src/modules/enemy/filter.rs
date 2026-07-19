use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length, Theme};
use nyanko::enemy::abilities::Identity;

use core::modules::enemy::filter::{EnemyFilterState, MatchMode};

#[derive(Clone)]
pub enum Message {
    Toggle,
    Clear,
    MatchModeChanged(MatchMode),
    MagChanged(String),
    IdentityToggled(Identity),
    AdvMinChanged(Identity, &'static str, String),
    AdvMaxChanged(Identity, &'static str, String),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Toggle => write!(f, "Toggle"),
            Self::Clear => write!(f, "Clear"),
            Self::MatchModeChanged(_) => write!(f, "MatchModeChanged"),
            Self::MagChanged(s) => write!(f, "MagChanged({})", s),
            Self::IdentityToggled(_) => write!(f, "IdentityToggled"),
            Self::AdvMinChanged(_, _, s) => write!(f, "AdvMinChanged({})", s),
            Self::AdvMaxChanged(_, _, s) => write!(f, "AdvMaxChanged({})", s),
        }
    }
}

pub struct State {
    pub filter_state: EnemyFilterState,
}

impl Default for State {
    fn default() -> Self {
        Self { filter_state: EnemyFilterState::default() }
    }
}

impl State {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => self.filter_state.is_open = !self.filter_state.is_open,
            Message::Clear => {
                self.filter_state = EnemyFilterState { is_open: self.filter_state.is_open, ..Default::default() };
            }
            Message::MatchModeChanged(mode) => self.filter_state.match_mode = mode,
            Message::MagChanged(mag) => self.filter_state.mag_input = mag,
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

    pub fn view(&self) -> Element<Message> {
        let match_mode_row = row![
            text("Mode:"),
            button(if self.filter_state.match_mode == MatchMode::And { "And" } else { "Or" })
                .on_press(Message::MatchModeChanged(
                    if self.filter_state.match_mode == MatchMode::And { MatchMode::Or } else { MatchMode::And }
                ))
        ].spacing(10).align_y(Alignment::Center);

        let mag_row = row![
            text("Target Magnification:"),
            text_input("100", &self.filter_state.mag_input).on_input(Message::MagChanged).width(Length::Fixed(60.0)),
            text("%")
        ].spacing(10).align_y(Alignment::Center);

        let actions = row![
            button("Clear Filter").on_press(Message::Clear).style(button::danger),
            button("Close").on_press(Message::Toggle).style(button::primary)
        ].spacing(15);

        let content = column![
            text("Advanced Enemy Filter").size(24),
            Space::new().height(Length::Fixed(10.0)),
            match_mode_row,
            mag_row,
            Space::new().height(Length::Fixed(20.0)),
            text("Trait & Attack Types... (Filter groups parsed from REGISTRY)"),
            Space::new().height(Length::Fixed(20.0)),
            actions
        ].spacing(10).padding(20);

        container(scrollable(content))
            .width(Length::Fixed(450.0))
            .height(Length::Fixed(500.0))
            .style(|theme: &Theme| {
                container::Style {
                    background: Some(theme.palette().background.into()),
                    border: iced::Border {
                        width: 2.0,
                        color: theme.palette().text,
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }
}
