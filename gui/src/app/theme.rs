use iced::alignment::Horizontal;
use iced::border::Radius;
use iced::theme::Palette;
use iced::widget::overlay::menu;
use iced::widget::text::Text;
use iced::widget::{button, container, pick_list, text, text_input, toggler};
use iced::{Background, Border, Color, Length, Theme};

pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 8.0;

const DISABLED_BUTTON_SHADE: f32 = 0.6;

fn shade_color(color: Color, factor: f32) -> Color {
    Color { r: color.r * factor, g: color.g * factor, b: color.b * factor, a: color.a }
}

fn lighten_color(color: Color, factor: f32) -> Color {
    let lighten = |c: f32| c + (1.0 - c) * factor;
    Color { r: lighten(color.r), g: lighten(color.g), b: lighten(color.b), a: color.a }
}

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
    let border_color = lighten_color(palette.background, 0.4);

    if !is_available {
        let ext = theme.extended_palette();
        let background = shade_color(ext.background.weak.color, DISABLED_BUTTON_SHADE);

        return button::Style {
            background: Some(Background::Color(background)),
            text_color: Color { a: 0.4, ..ext.background.weak.text },
            border: Border { color: border_color, width: 1.0, radius: Radius::from(RADIUS_SM) },
            ..button::Style::default()
        };
    }

    let base = toggle_button(theme, status, is_selected);

    let (border_color, border_width) = if is_selected { (Color::WHITE, 2.0) } else { (border_color, 1.0) };

    button::Style {
        border: Border { color: border_color, width: border_width, ..base.border },
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
    let background = shade_color(palette.background, 0.35);
    let border_color = shade_color(palette.background, 0.6);

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

const CARD_SHADE: f32 = 0.15;
const CARD_ALPHA: f32 = 0.92;

pub fn card_container(theme: &Theme) -> container::Style {
    let palette = theme.palette();

    container::Style {
        background: Some(Background::Color(Color { a: CARD_ALPHA, ..shade_color(palette.background, CARD_SHADE) })),
        border: Border { radius: Radius::from(RADIUS_MD), ..Border::default() },
        ..container::Style::default()
    }
}

pub fn combo_box(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let base = pick_list::default(theme, status);

    pick_list::Style { border: Border { radius: Radius::from(RADIUS_SM), ..base.border }, ..base }
}

pub fn combo_box_menu(theme: &Theme) -> menu::Style {
    let palette = theme.palette();
    let base = menu::default(theme);

    menu::Style {
        selected_text_color: palette.text,
        selected_background: Background::Color(palette.primary),
        border: Border { radius: Radius::from(RADIUS_SM), ..base.border },
        ..base
    }
}

pub fn ios_toggle(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let base = toggler::default(theme, status);

    toggler::Style { border_radius: Some(Radius::from(RADIUS_SM)), ..base }
}

const TABLE_HEADER_SHADE: f32 = 0.55;
const TABLE_ROW_SHADE: f32 = 0.8;

pub fn zebra_table_header(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    let background = shade_color(palette.background, TABLE_HEADER_SHADE);

    container::Style {
        background: Some(Background::Color(background)),
        text_color: Some(Color::WHITE),
        border: Border {
            radius: Radius { top_left: RADIUS_SM, top_right: RADIUS_SM, bottom_left: 0.0, bottom_right: 0.0 },
            ..Border::default()
        },
        ..container::Style::default()
    }
}

pub fn zebra_table_row(theme: &Theme, index: usize) -> container::Style {
    let palette = theme.palette();
    let background = if index.is_multiple_of(2) { shade_color(palette.background, TABLE_ROW_SHADE) } else { palette.background };

    container::Style { background: Some(Background::Color(background)), ..container::Style::default() }
}

pub fn table_cell_text(content: &str, width: Length) -> Text<'_> {
    text(content).width(width).align_x(Horizontal::Center)
}

pub fn rounded_input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let style = text_input::default(theme, status);
    let background = match style.background {
        Background::Color(color) => Background::Color(shade_color(color, 0.5)),
        other => other,
    };

    text_input::Style {
        background,
        border: Border { radius: Radius::from(RADIUS_SM), ..style.border },
        ..style
    }
}
