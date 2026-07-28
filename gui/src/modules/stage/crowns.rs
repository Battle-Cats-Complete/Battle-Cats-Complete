use iced::widget::{button, row, space, text};
use iced::{Border, Color, Element, Length, Theme};

use core::modules::stage::Stage;

pub fn view(stage: &Stage, selected_crown: u8) -> Element<'_, super::Message> {
    if stage.max_crowns <= 1 {
        return space().into();
    }

    let mut crown_row = row![].spacing(5);

    for c in 0..stage.max_crowns {
        let is_selected = selected_crown == c;
        let is_enabled = stage.target_crowns == -1 || stage.target_crowns as u8 == c;

        let label = format!("{}♔", c + 1);
        let mut crown_btn = button(text(label).size(14))
            .width(Length::Fixed(50.0))
            .height(Length::Fixed(30.0))
            .style(move |_theme: &Theme, _status| {
                let (background, border_color, text_color) = if is_selected {
                    (Color::from_rgb8(0, 100, 200), Color::WHITE, Color::WHITE)
                } else if is_enabled {
                    (Color::from_rgb8(40, 40, 40), Color::from_rgb8(100, 100, 100), Color::from_rgb8(200, 200, 200))
                } else {
                    (Color::from_rgb8(15, 15, 15), Color::from_rgb8(50, 50, 50), Color::from_rgb8(120, 120, 120))
                };

                button::Style {
                    background: Some(background.into()),
                    text_color,
                    border: Border {
                        color: border_color,
                        width: if is_selected { 2.0 } else { 1.0 },
                        radius: 0.0.into(),
                    },
                    ..button::Style::default()
                }
            });

        if is_enabled {
            crown_btn = crown_btn.on_press(super::Message::SelectCrown(c));
        }

        crown_row = crown_row.push(crown_btn);
    }

    crown_row.into()
}
