use iced::advanced::{layout, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{self, Interaction};
use iced::widget::{button, column, container, mouse_area, opaque, responsive, row, stack, text, Space};
use iced::{Alignment, Border, Element, Event, Length, Point, Rectangle, Size, Theme, Vector};

const HEADER_HEIGHT: f32 = 28.0;

#[derive(Default)]
pub struct State {
    position: Option<Point>,
    drag: Drag,
}

#[derive(Default, Clone, Copy)]
enum Drag {
    #[default]
    Idle,
    Pressed,
    Moving {
        last: Point,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    HeaderPressed,
    Dragged(Point, Size),
    Released,
    Close,
}

impl State {
    pub fn update(&mut self, message: Message, size: Size) -> bool {
        match message {
            Message::HeaderPressed => self.drag = Drag::Pressed,
            Message::Dragged(cursor, window) => match self.drag {
                Drag::Idle => {}
                Drag::Pressed => self.drag = Drag::Moving { last: cursor },
                Drag::Moving { last } => {
                    let current = self.resolved_position(size, window);
                    let next = Point::new(current.x + cursor.x - last.x, current.y + cursor.y - last.y);
                    self.position = Some(clamp(next, size, window));
                    self.drag = Drag::Moving { last: cursor };
                }
            },
            Message::Released => self.drag = Drag::Idle,
            Message::Close => {
                self.drag = Drag::Idle;
                return true;
            }
        }

        false
    }

    pub fn view<'a, M: Clone + 'a>(
        &'a self,
        title: &'a str,
        size: Size,
        window: Size,
        to_message: fn(Message) -> M,
        content: impl Fn() -> Element<'a, M> + 'a,
    ) -> Element<'a, M> {
        responsive(move |layer| {
            let bounds = if window.width < 1.0 || window.height < 1.0 { layer } else { window };
            let position = self.resolved_position(size, bounds);

            let close_button = button(text("✕").size(14))
                .style(button::text)
                .padding([0.0, 8.0])
                .on_press(to_message(Message::Close));

            let header = mouse_area(
                container(
                    row![
                        text(title).size(14),
                        Space::new().width(Length::Fill),
                        close_button,
                    ]
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .height(Length::Fixed(HEADER_HEIGHT))
                .align_y(iced::alignment::Vertical::Center)
                .padding([0.0, 10.0])
                .style(header_style),
            )
            .interaction(Interaction::Grab)
            .on_press(to_message(Message::HeaderPressed));

            let window = opaque(
                container(column![header, content()])
                    .width(Length::Fixed(size.width))
                    .height(Length::Fixed(size.height))
                    .style(window_style),
            );

            let base = anchored(window, position);

            if matches!(self.drag, Drag::Idle) {
                base
            } else {
                stack![
                    base,
                    mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                        .interaction(Interaction::Grabbing)
                        .on_move(move |cursor| to_message(Message::Dragged(cursor, bounds)))
                        .on_release(to_message(Message::Released))
                        .on_exit(to_message(Message::Released)),
                ]
                .into()
            }
        })
        .into()
    }

    fn resolved_position(&self, size: Size, window: Size) -> Point {
        let centered = Point::new(
            ((window.width - size.width) / 2.0).max(0.0),
            ((window.height - size.height) / 2.0).max(0.0),
        );

        clamp(self.position.unwrap_or(centered), size, window)
    }
}

struct Anchored<'a, M> {
    content: Element<'a, M>,
    position: Point,
}

fn anchored<'a, M: 'a>(content: impl Into<Element<'a, M>>, position: Point) -> Element<'a, M> {
    Element::new(Anchored {
        content: content.into(),
        position,
    })
}

impl<'a, M> Widget<M, Theme, iced::Renderer> for Anchored<'a, M> {
    fn tag(&self) -> widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<widget::Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(&mut self, tree: &mut widget::Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let node = self
            .content
            .as_widget_mut()
            .layout(tree, renderer, &layout::Limits::new(Size::ZERO, Size::INFINITE))
            .move_to(self.position);

        layout::Node::with_children(limits.max(), vec![node])
    }

    fn operate(&mut self, tree: &mut widget::Tree, layout: Layout<'_>, renderer: &iced::Renderer, operation: &mut dyn widget::Operation) {
        if let Some(child) = layout.children().next() {
            self.content.as_widget_mut().operate(tree, child, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        if let Some(child) = layout.children().next() {
            self.content
                .as_widget_mut()
                .update(tree, event, child, cursor, renderer, clipboard, shell, viewport);
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        match layout.children().next() {
            Some(child) => self
                .content
                .as_widget()
                .mouse_interaction(tree, child, cursor, viewport, renderer),
            None => mouse::Interaction::None,
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if let (Some(clipped_viewport), Some(child)) = (bounds.intersection(viewport), layout.children().next()) {
            self.content
                .as_widget()
                .draw(tree, renderer, theme, style, child, cursor, &clipped_viewport);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, iced::Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout.children().next()?, renderer, viewport, translation)
    }
}

fn clamp(position: Point, size: Size, window: Size) -> Point {
    Point::new(
        position.x.clamp(0.0, (window.width - size.width).max(0.0)),
        position.y.clamp(0.0, (window.height - HEADER_HEIGHT).max(0.0)),
    )
}

fn header_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.strong.color.into()),
        border: Border {
            radius: Radius {
                top_left: 6.0,
                top_right: 6.0,
                bottom_right: 0.0,
                bottom_left: 0.0,
            },
            ..Border::default()
        },
        ..container::Style::default()
    }
}

fn window_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weak.color.into()),
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{clamp, HEADER_HEIGHT};
    use iced::{Point, Size};

    #[test]
    fn header_corners_stay_within_window() {
        let size = Size::new(320.0, 500.0);
        let window = Size::new(800.0, 600.0);

        let clamped = clamp(Point::new(900.0, 700.0), size, window);
        assert_eq!(clamped, Point::new(480.0, 600.0 - HEADER_HEIGHT));

        let clamped = clamp(Point::new(-50.0, -20.0), size, window);
        assert_eq!(clamped, Point::ORIGIN);
    }

    #[test]
    fn body_may_overhang_the_bottom_edge() {
        let size = Size::new(320.0, 500.0);
        let window = Size::new(800.0, 600.0);

        let fully_inside_max_y = window.height - size.height;
        let clamped = clamp(Point::new(0.0, 550.0), size, window);
        assert!(clamped.y > fully_inside_max_y);
        assert_eq!(clamped.y, 550.0);
    }

    #[test]
    fn popup_reaches_the_true_window_origin() {
        let size = Size::new(320.0, 500.0);
        let window = Size::new(800.0, 600.0);

        let clamped = clamp(Point::new(-500.0, -500.0), size, window);
        assert_eq!(clamped, Point::ORIGIN);

        let clamped = clamp(Point::new(10.0, 5.0), size, window);
        assert_eq!(clamped, Point::new(10.0, 5.0));
    }

    #[test]
    fn window_smaller_than_popup_pins_to_origin() {
        let size = Size::new(320.0, 500.0);
        let window = Size::new(300.0, 20.0);

        let clamped = clamp(Point::new(100.0, 100.0), size, window);
        assert_eq!(clamped, Point::ORIGIN);
    }
}
