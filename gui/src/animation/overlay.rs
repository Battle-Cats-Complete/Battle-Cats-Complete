use iced::alignment::{Horizontal, Vertical};
use iced::border::Radius;
use iced::mouse;
use iced::widget::canvas;
use iced::widget::canvas::{Geometry, Path, Stroke};
use iced::widget::{container, text};
use iced::{Background, Border, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

use crate::app::theme;
use crate::widget::{slide, Slide};

use super::canvas as viewer;

const MIN_SELECTION_AREA: f32 = 25.0;
const HINT_TEXT: &str = "Right click & drag to set camera";
const HINT_TEXT_SIZE: f32 = 13.0;
const HINT_WIDTH: f32 = 260.0;
const HINT_HEIGHT: f32 = 28.0;
const HINT_ALPHA: f32 = 160.0 / 255.0;
const HINT_SHADE: f32 = 0.15;

#[derive(Default)]
pub struct State {
    pub selecting: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Selected(Region),
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl State {
    pub fn view(&self, viewer_state: &viewer::State, region: Option<Region>, debug_origin: bool) -> Element<'_, Message> {
        canvas(Selector {
            selecting: self.selecting,
            pan: viewer_state.pan,
            zoom: viewer_state.zoom,
            region,
            debug_origin,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub fn hint_view<'a, M: 'a>(&self) -> Element<'a, M> {
        let label = container(text(HINT_TEXT).size(HINT_TEXT_SIZE).color(Color::WHITE))
            .width(Length::Fixed(HINT_WIDTH))
            .height(Length::Fixed(HINT_HEIGHT))
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .style(hint_style);

        container(slide(label, self.selecting, Slide::Up))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Top)
            .into()
    }
}

fn hint_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    let shade = |c: f32| c * HINT_SHADE;

    container::Style {
        background: Some(Background::Color(Color {
            r: shade(palette.background.r),
            g: shade(palette.background.g),
            b: shade(palette.background.b),
            a: HINT_ALPHA,
        })),
        border: Border {
            color: theme.extended_palette().background.strong.color,
            width: 1.0,
            radius: Radius { top_left: 0.0, top_right: 0.0, bottom_left: theme::RADIUS_LG, bottom_right: theme::RADIUS_LG },
        },
        ..container::Style::default()
    }
}

struct Selector {
    selecting: bool,
    pan: Vector,
    zoom: f32,
    region: Option<Region>,
    debug_origin: bool,
}

#[derive(Default)]
struct Drag {
    anchor: Option<(Point, Point)>,
}

impl canvas::Program<Message> for Selector {
    type State = Drag;

    fn update(
        &self,
        drag: &mut Drag,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if !self.selecting {
            return None;
        }

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let position = cursor.position_in(bounds)?;
                drag.anchor = Some((position, position));
                Some(canvas::Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let (start, _) = drag.anchor?;
                let position = Point::new(position.x - bounds.x, position.y - bounds.y);
                drag.anchor = Some((start, position));
                Some(canvas::Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)) => {
                let (start, end) = drag.anchor.take()?;

                let width = (end.x - start.x).abs();
                let height = (end.y - start.y).abs();

                let message = if width * height > MIN_SELECTION_AREA {
                    let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
                    let to_world = |point: Point| Vector::new(
                        (point.x - center.x) / self.zoom - self.pan.x,
                        (point.y - center.y) / self.zoom - self.pan.y,
                    );

                    let a = to_world(start);
                    let b = to_world(end);

                    Message::Selected(Region {
                        x: a.x.min(b.x),
                        y: a.y.min(b.y),
                        w: (b.x - a.x).abs(),
                        h: (b.y - a.y).abs(),
                    })
                } else {
                    Message::Cancelled
                };

                Some(canvas::Action::publish(message).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        drag: &Drag,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        if !self.selecting && self.region.is_none() && !self.debug_origin {
            return Vec::new();
        }

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let yellow = Color::from_rgb8(255, 255, 0);

        if self.selecting {
            frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgba8(0, 0, 0, 50.0 / 255.0));

            if let Some((start, end)) = drag.anchor {
                let top_left = Point::new(start.x.min(end.x), start.y.min(end.y));
                let size = Size::new((end.x - start.x).abs(), (end.y - start.y).abs());

                frame.fill_rectangle(top_left, size, Color::from_rgba8(255, 255, 0, 30.0 / 255.0));
                frame.stroke(
                    &Path::rectangle(top_left, size),
                    Stroke::default().with_color(yellow).with_width(2.0),
                );
            }

        } else if let Some(region) = self.region {
            let center = frame.center();
            let to_screen = |x: f32, y: f32| Point::new(
                center.x + (x + self.pan.x) * self.zoom,
                center.y + (y + self.pan.y) * self.zoom,
            );

            let min = to_screen(region.x, region.y);
            let max = to_screen(region.x + region.w, region.y + region.h);

            frame.stroke(
                &Path::rectangle(min, Size::new(max.x - min.x, max.y - min.y)),
                Stroke::default().with_color(yellow).with_width(1.0),
            );
        }

        if self.debug_origin {
            let center = frame.center();
            let origin = Point::new(center.x + self.pan.x * self.zoom, center.y + self.pan.y * self.zoom);
            let cross_size = 15.0;
            let stroke = Stroke::default().with_color(Color::from_rgb8(0, 255, 0)).with_width(2.0);
            frame.stroke(
                &Path::line(Point::new(origin.x - cross_size, origin.y), Point::new(origin.x + cross_size, origin.y)),
                stroke,
            );
            frame.stroke(
                &Path::line(Point::new(origin.x, origin.y - cross_size), Point::new(origin.x, origin.y + cross_size)),
                stroke,
            );
        }

        vec![frame.into_geometry()]
    }
}

