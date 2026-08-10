use iced::widget::{button, row, space};
use iced::{Element, Length, Theme};

use core::modules::stage::Stage;

use crate::app::theme;

const CROWN_BTN_WIDTH: f32 = 50.0;
const CROWN_BTN_HEIGHT: f32 = 30.0;
const CROWN_BTN_SPACING: f32 = 5.0;
const CROWN_TEXT_SIZE: f32 = 14.0;

pub fn view(stage: &Stage, selected_crown: u8) -> Element<'_, super::Message> {
    if stage.max_crowns <= 1 {
        return space().into();
    }

    let mut crown_row = row![].spacing(CROWN_BTN_SPACING);

    for crown in 0..stage.max_crowns {
        let is_selected = selected_crown == crown;
        let is_enabled = stage.target_crowns == -1 || stage.target_crowns as u8 == crown;

        let label = theme::centered_text(format!("{}🜲", crown + 1))
            .size(CROWN_TEXT_SIZE)
            .width(Length::Fill)
            .height(Length::Fill);

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
