use iced::mouse;
use iced::widget::canvas;
use iced::widget::canvas::{Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

use super::canvas as viewer;

const MIN_SELECTION_AREA: f32 = 25.0;
const HINT_TEXT: &str = "Right click & drag to set camera";

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
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let (start, _) = drag.anchor?;
                let position = cursor.position_in(bounds)?;
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

            let hint_size = Size::new(240.0, 25.0);
            let hint_top_left = Point::new(frame.center().x - hint_size.width / 2.0, 30.0);
            frame.fill_rectangle(hint_top_left, hint_size, Color::from_rgba8(0, 0, 0, 180.0 / 255.0));
            frame.fill_text(canvas::Text {
                content: HINT_TEXT.to_string(),
                position: Point::new(hint_top_left.x + 10.0, hint_top_left.y + 5.0),
                color: Color::WHITE,
                size: 13.0.into(),
                ..canvas::Text::default()
            });

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

#[cfg(test)]
mod tests {
    use iced::widget::canvas::Program;

    use super::*;

    fn offset_bounds() -> Rectangle {
        Rectangle { x: 400.0, y: 100.0, width: 800.0, height: 600.0 }
    }

    fn selector(selecting: bool) -> Selector {
        Selector {
            selecting,
            pan: Vector::new(-30.0, 10.0),
            zoom: 2.0,
            region: None,
            debug_origin: false,
        }
    }

    fn drive(
        program: &Selector,
        drag: &mut Drag,
        event: canvas::Event,
        point: Point,
    ) -> Option<canvas::Action<Message>> {
        program.update(drag, &event, offset_bounds(), mouse::Cursor::Available(point))
    }

    #[test]
    fn right_drag_publishes_world_space_region() {
        let program = selector(true);
        let mut drag = Drag::default();

        let press = Point::new(500.0, 200.0);
        let action = drive(&program, &mut drag, canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)), press);
        assert!(action.is_some(), "right press should be captured while selecting");

        let release = Point::new(600.0, 260.0);
        drive(&program, &mut drag, canvas::Event::Mouse(mouse::Event::CursorMoved { position: release }), release)
            .expect("cursor move should be captured while dragging");

        let action = drive(&program, &mut drag, canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)), release)
            .expect("release should publish");

        let (message, _, _) = action.into_inner();
        let Some(Message::Selected(region)) = message else {
            panic!("expected a selected region");
        };

        assert_eq!(region, Region { x: -120.0, y: -110.0, w: 50.0, h: 30.0 });
    }

    #[test]
    fn tiny_selection_cancels_instead_of_publishing() {
        let program = selector(true);
        let mut drag = Drag::default();

        let press = Point::new(500.0, 200.0);
        drive(&program, &mut drag, canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)), press);

        let release = Point::new(503.0, 203.0);
        drive(&program, &mut drag, canvas::Event::Mouse(mouse::Event::CursorMoved { position: release }), release);

        let action = drive(&program, &mut drag, canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)), release)
            .expect("release should still be captured");

        let (message, _, _) = action.into_inner();
        assert!(matches!(message, Some(Message::Cancelled)));
    }

    #[test]
    fn events_pass_through_when_not_selecting() {
        let program = selector(false);
        let mut drag = Drag::default();

        let point = Point::new(500.0, 200.0);
        let press = drive(&program, &mut drag, canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)), point);
        assert!(press.is_none());

        let left = drive(&program, &mut drag, canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)), point);
        assert!(left.is_none());
    }
}
