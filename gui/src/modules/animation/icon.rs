use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme};

const REACH: f32 = 7.0;
const ARM: f32 = 4.5;
const STROKE_WIDTH: f32 = 2.0;
const CORNERS: [(f32, f32); 4] = [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)];

pub fn expand<'a, Message: 'a>(is_expanded: bool) -> Element<'a, Message> {
    Canvas::new(Expand { is_expanded })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

struct Expand {
    is_expanded: bool,
}

impl<Message> canvas::Program<Message> for Expand {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let color = if self.is_expanded {
            Color::WHITE
        } else {
            theme.extended_palette().background.strong.text
        };
        let stroke = Stroke::default().with_color(color).with_width(STROKE_WIDTH);
        let center = frame.center();

        for (dx, dy) in CORNERS {
            let corner = Point::new(center.x + dx * REACH, center.y + dy * REACH);
            let bracket = Path::new(|builder| {
                builder.move_to(Point::new(corner.x - dx * ARM, corner.y));
                builder.line_to(corner);
                builder.line_to(Point::new(corner.x, corner.y - dy * ARM));
            });

            frame.stroke(&bracket, stroke);
        }

        vec![frame.into_geometry()]
    }
}
