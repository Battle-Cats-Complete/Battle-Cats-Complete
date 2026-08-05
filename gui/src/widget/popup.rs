use iced::advanced::{layout, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{self, Interaction};
use iced::widget::{button, column, container, mouse_area, opaque, responsive, stack, text, Space};
use iced::{Alignment, Border, Color, Element, Event, Length, Padding, Point, Rectangle, Size, Theme, Vector};

use crate::app::theme;

const HEADER_HEIGHT: f32 = 28.0;
const HEADER_MARGIN_X: f32 = 50.0;
const HEADER_MARGIN_Y: f32 = 30.0;
const DEFAULT_BODY_ALPHA: f32 = 1.0;

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
        body_alpha: Option<f32>,
    ) -> Element<'a, M> {
        let body_alpha = body_alpha.unwrap_or(DEFAULT_BODY_ALPHA);

        responsive(move |layer| {
            let bounds = if window.width < 1.0 || window.height < 1.0 { layer } else { window };
            let position = self.resolved_position(size, bounds);

            let close_button = button(text("✕").size(18))
                .style(button::text)
                .padding(2.0)
                .on_press(to_message(Message::Close));

            let title_layer = container(text(title).size(14))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center);

            let button_layer = container(close_button)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::Center);

            let header = mouse_area(
                container(stack![title_layer, button_layer])
                    .width(Length::Fill)
                    .height(Length::Fixed(HEADER_HEIGHT))
                    .padding(Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 10.0 })
                    .style(header_style),
            )
            .interaction(Interaction::Grab)
            .on_press(to_message(Message::HeaderPressed));

            let body = container(content())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |theme: &Theme| body_style(theme, body_alpha));

            let window = opaque(
                container(column![header, body])
                    .width(Length::Fixed(size.width))
                    .height(Length::Fixed(size.height))
                    .style(frame_style),
            );

            let base = anchored(window, position);

            let mut drag_layer = mouse_area(Space::new().width(Length::Fill).height(Length::Fill));

            if !matches!(self.drag, Drag::Idle) {
                drag_layer = drag_layer
                    .interaction(Interaction::Grabbing)
                    .on_move(move |cursor| to_message(Message::Dragged(cursor, bounds)))
                    .on_release(to_message(Message::Released))
                    .on_exit(to_message(Message::Released));
            }

            stack![base, drag_layer].into()
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
        layout.children().next().map_or(mouse::Interaction::None, |child| {
            self.content.as_widget().mouse_interaction(tree, child, cursor, viewport, renderer)
        })
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
    let min_x = HEADER_MARGIN_X - size.width;
    let max_x = (window.width - HEADER_MARGIN_X).max(min_x);
    let max_y = (window.height - HEADER_MARGIN_Y).max(0.0);

    Point::new(position.x.clamp(min_x, max_x), position.y.clamp(0.0, max_y))
}

fn header_style(theme: &Theme) -> container::Style {
    let palette = theme.palette();
    let shade = |c: f32| c * 0.7;
    let background = Color {
        r: shade(palette.background.r),
        g: shade(palette.background.g),
        b: shade(palette.background.b),
        a: palette.background.a,
    };

    container::Style {
        background: Some(background.into()),
        border: Border {
            radius: Radius {
                top_left: theme::RADIUS_MD,
                top_right: theme::RADIUS_MD,
                bottom_right: 0.0,
                bottom_left: 0.0,
            },
            ..Border::default()
        },
        ..container::Style::default()
    }
}

fn frame_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        border: Border {
            color: palette.background.strong.color,
            width: 1.0,
            radius: theme::RADIUS_MD.into(),
        },
        ..container::Style::default()
    }
}

fn body_style(theme: &Theme, alpha: f32) -> container::Style {
    let palette = theme.extended_palette();
    let background = Color { a: alpha, ..palette.background.base.color };

    container::Style {
        background: Some(background.into()),
        border: Border {
            radius: Radius {
                top_left: 0.0,
                top_right: 0.0,
                bottom_left: theme::RADIUS_MD,
                bottom_right: theme::RADIUS_MD,
            },
            ..Border::default()
        },
        ..container::Style::default()
    }
}

