use iced::border::Radius;
use iced::theme::Palette;
use iced::widget::{button, container, text_input};
use iced::{Background, Border, Color, Theme};

pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 8.0;

#[derive(PartialEq, Clone, Copy, Debug, Default, serde::Deserialize, serde::Serialize)]
pub enum AppTheme {
    Light,
    #[default]
    Dark,
}

impl AppTheme {
    pub fn palette(self) -> Palette {
        match self {
            Self::Light => Palette {
                background: Color::from_rgb8(245, 245, 245),
                text: Color::from_rgb8(25, 25, 25),
                primary: Color::from_rgb8(31, 106, 165),
                success: Color::from_rgb8(30, 130, 80),
                warning: Color::from_rgb8(180, 130, 20),
                danger: Color::from_rgb8(190, 40, 40),
            },
            Self::Dark => Palette {
                background: Color::from_rgb8(33, 33, 33),
                text: Color::from_rgb8(230, 230, 230),
                primary: Color::from_rgb8(31, 106, 165),
                success: Color::from_rgb8(46, 160, 90),
                warning: Color::from_rgb8(210, 180, 60),
                danger: Color::from_rgb8(210, 60, 60),
            },
        }
    }

    pub fn to_iced_theme(self) -> Theme {
        let name = match self {
            Self::Light => "BCC Light",
            Self::Dark => "BCC Dark",
        };

        Theme::custom(name, self.palette())
    }
}

pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let background = if status == button::Status::Hovered { Color { a: 0.8, ..palette.primary } } else { palette.primary };

    solid_button(background)
}

pub fn danger_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let background = if status == button::Status::Hovered { Color { a: 0.8, ..palette.danger } } else { palette.danger };

    solid_button(background)
}

pub fn toggle_button(theme: &Theme, status: button::Status, is_active: bool) -> button::Style {
    let palette = theme.extended_palette();

    let pair = if is_active {
        if status == button::Status::Hovered { palette.primary.strong } else { palette.primary.base }
    } else if status == button::Status::Hovered {
        palette.background.strongest
    } else {
        palette.background.strong
    };

    button::Style {
        background: Some(Background::Color(pair.color)),
        text_color: if is_active { Color::WHITE } else { pair.text },
        border: Border { radius: Radius::from(RADIUS_SM), ..Border::default() },
        ..button::Style::default()
    }
}

pub fn header_toggle_button(theme: &Theme, status: button::Status, is_selected: bool, is_available: bool) -> button::Style {
    let palette = theme.palette();
    let lighten = |c: f32, factor: f32| c + (1.0 - c) * factor;
    let border_color = Color {
        r: lighten(palette.background.r, 0.4),
        g: lighten(palette.background.g, 0.4),
        b: lighten(palette.background.b, 0.4),
        a: palette.background.a,
    };

    if !is_available {
        let ext = theme.extended_palette();

        return button::Style {
            background: Some(Background::Color(ext.background.weak.color)),
            text_color: Color { a: 0.4, ..ext.background.weak.text },
            border: Border { color: border_color, width: 1.0, radius: Radius::from(RADIUS_SM) },
            ..button::Style::default()
        };
    }

    let base = toggle_button(theme, status, is_selected);

    button::Style {
        border: Border { color: border_color, width: 1.0, ..base.border },
        ..base
    }
}

pub fn solid_button(background: Color) -> button::Style {
    button::Style {
        background: Some(Background::Color(background)),
        text_color: Color::WHITE,
        border: Border { radius: Radius::from(RADIUS_SM), ..Border::default() },
        ..button::Style::default()
    }
}

pub fn sidebar_container(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    let shade = |c: f32, factor: f32| c * factor;
    let background = Color {
        r: shade(palette.background.r, 0.35),
        g: shade(palette.background.g, 0.35),
        b: shade(palette.background.b, 0.35),
        a: palette.background.a,
    };
    let border_color = Color {
        r: shade(palette.background.r, 0.6),
        g: shade(palette.background.g, 0.6),
        b: shade(palette.background.b, 0.6),
        a: palette.background.a,
    };

    container::Style {
        background: Some(Background::Color(background)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: Radius { top_left: 10.0, bottom_left: 10.0, top_right: 0.0, bottom_right: 0.0 },
        },
        ..container::Style::default()
    }
}

pub fn list_panel_container(theme: &Theme) -> container::Style {
    let style = container::bordered_box(theme);

    container::Style {
        border: Border {
            radius: Radius { top_left: 0.0, bottom_left: 0.0, top_right: RADIUS_MD, bottom_right: RADIUS_MD },
            ..style.border
        },
        ..style
    }
}

pub fn confirm_modal_container(theme: &Theme) -> container::Style {
    let palette = theme.palette();

    container::Style {
        background: Some(Background::Color(palette.background)),
        border: Border { color: palette.text, width: 1.0, radius: Radius::from(RADIUS_LG) },
        ..container::Style::default()
    }
}

#[allow(dead_code)]
pub fn rounded_input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let style = text_input::default(theme, status);

    text_input::Style {
        border: Border { radius: Radius::from(RADIUS_MD), ..style.border },
        ..style
    }
}

#[allow(dead_code)]
pub fn card_container(theme: &Theme) -> container::Style {
    let style = container::rounded_box(theme);

    container::Style {
        border: Border { radius: Radius::from(RADIUS_MD), ..style.border },
        ..style
    }
}
