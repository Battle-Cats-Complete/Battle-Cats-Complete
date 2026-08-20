use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, container, row, space, text};
use iced::{Element, Length, Theme};

use kore::domains::stage::Stage;

use crate::app::theme;
use crate::common::fonts;

const CROWN_BTN_WIDTH: f32 = 50.0;
const CROWN_BTN_HEIGHT: f32 = 30.0;
const CROWN_BTN_SPACING: f32 = 5.0;
const CROWN_TEXT_SIZE: f32 = 14.0;

const CROWN_GLYPH: &str = "\u{1F732}";

pub fn view(stage: &Stage, selected_crown: u8) -> Element<'_, super::Message> {
    if stage.max_crowns <= 1 {
        return space().into();
    }

    let mut crown_row = row![].spacing(CROWN_BTN_SPACING);

    for crown in 0..stage.max_crowns {
        let is_selected = selected_crown == crown;
        let is_enabled = stage.target_crowns == -1 || stage.target_crowns as u8 == crown;

        let label = container(row![
            text(crown + 1).size(CROWN_TEXT_SIZE),
            text(CROWN_GLYPH).font(fonts::MISC_SYMBOLS).size(CROWN_TEXT_SIZE),
        ]
        .align_y(Vertical::Center))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);

        crown_row = crown_row.push(
            button(label)
                .width(Length::Fixed(CROWN_BTN_WIDTH))
                .height(Length::Fixed(CROWN_BTN_HEIGHT))
                .padding(0)
                .on_press_maybe(is_enabled.then_some(super::Message::SelectCrown(crown)))
                .style(move |theme: &Theme, status| theme::header_toggle_button(theme, status, is_selected, is_enabled)),
        );
    }

    crown_row.into()
}
