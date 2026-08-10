use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

const SPAN: f32 = 14.0;
const ARM: f32 = 5.0;
const THICKNESS: f32 = 2.0;
const CORNERS: [(f32, f32); 4] = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)];

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
        frame.translate(Vector::new(bounds.x.round() - bounds.x, bounds.y.round() - bounds.y));

        let color = if self.is_expanded {
            Color::WHITE
        } else {
            theme.extended_palette().background.strong.text
        };

        let center = frame.center();
        let left = (center.x - SPAN / 2.0).round();
        let top = (center.y - SPAN / 2.0).round();

        for (fx, fy) in CORNERS {
            let arm_x = left + fx * (SPAN - ARM);
            let arm_y = top + fy * (SPAN - ARM);
            let edge_x = left + fx * (SPAN - THICKNESS);
            let edge_y = top + fy * (SPAN - THICKNESS);

            frame.fill_rectangle(Point::new(arm_x, edge_y), Size::new(ARM, THICKNESS), color);
            frame.fill_rectangle(Point::new(edge_x, arm_y), Size::new(THICKNESS, ARM), color);
        }

        vec![frame.into_geometry()]
    }
}
