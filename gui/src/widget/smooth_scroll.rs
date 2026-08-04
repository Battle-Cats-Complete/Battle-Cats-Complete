use iced::advanced::{layout, mouse, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::time::{Duration, Instant};
use iced::{window, Element, Event, Length, Point, Rectangle, Size, Theme, Vector};

pub(crate) const LINE_PIXELS: f32 = 60.0;
const DECAY_RATE: f32 = 16.0;
const EPSILON: f32 = 0.05;
const DEFAULT_STRENGTH: f32 = 1.0;
const BOOTSTRAP_DT: Duration = Duration::from_millis(16);

pub(crate) fn smooth_scroll<'a, Message>(content: impl Into<Element<'a, Message>>) -> SmoothScroll<'a, Message> {
    SmoothScroll { content: content.into(), strength: DEFAULT_STRENGTH }
}

pub(crate) struct SmoothScroll<'a, Message> {
    content: Element<'a, Message>,
    strength: f32,
}

impl<'a, Message> SmoothScroll<'a, Message> {
    pub(crate) fn strength(mut self, strength: f32) -> Self {
        self.strength = strength;
        self
    }
}

impl<'a, Message: 'a> From<SmoothScroll<'a, Message>> for Element<'a, Message> {
    fn from(widget: SmoothScroll<'a, Message>) -> Self {
        Element::new(widget)
    }
}

struct ScrollState {
    remaining: Vector,
    last_frame: Option<Instant>,
    anchor: Point,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self { remaining: Vector::ZERO, last_frame: None, anchor: Point::ORIGIN }
    }
}

impl<'a, Message> Widget<Message, Theme, iced::Renderer> for SmoothScroll<'a, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<ScrollState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(ScrollState::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut widget::Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut widget::Tree, layout: Layout<'_>, renderer: &iced::Renderer, operation: &mut dyn widget::Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state: &mut ScrollState = tree.state.downcast_mut();

        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !cursor.is_over(layout.bounds()) {
                    return;
                }

                let raw = match delta {
                    mouse::ScrollDelta::Lines { x, y } => Vector::new(*x, *y) * LINE_PIXELS,
                    mouse::ScrollDelta::Pixels { x, y } => Vector::new(*x, *y),
                };

                state.anchor = cursor.position().unwrap_or(state.anchor);
                state.remaining += raw * self.strength;

                shell.capture_event();
                shell.request_redraw();
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                self.content
                    .as_widget_mut()
                    .update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);

                if state.remaining == Vector::ZERO {
                    return;
                }

                if state.last_frame == Some(*now) {
                    shell.request_redraw();
                    return;
                }

                let dt = state.last_frame.map(|last| *now - last).unwrap_or(BOOTSTRAP_DT);

                let step = if state.remaining.x.abs() < EPSILON && state.remaining.y.abs() < EPSILON {
                    let step = state.remaining;
                    state.remaining = Vector::ZERO;
                    state.last_frame = None;
                    step
                } else {
                    let alpha = 1.0 - (-DECAY_RATE * dt.as_secs_f32()).exp();
                    let step = state.remaining * alpha;
                    state.remaining -= step;
                    state.last_frame = Some(*now);
                    step
                };

                let synthetic = Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels { x: step.x, y: step.y },
                });
                let synthetic_cursor = mouse::Cursor::Available(state.anchor);

                let mut synthetic_messages = Vec::new();
                let mut synthetic_shell = Shell::new(&mut synthetic_messages);

                self.content.as_widget_mut().update(
                    &mut tree.children[0],
                    &synthetic,
                    layout,
                    synthetic_cursor,
                    renderer,
                    clipboard,
                    &mut synthetic_shell,
                    viewport,
                );

                shell.merge(synthetic_shell, std::convert::identity);

                if state.remaining != Vector::ZERO {
                    shell.request_redraw();
                }
            }
            _ => {
                let was_captured = shell.is_event_captured();

                self.content
                    .as_widget_mut()
                    .update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);

                if !was_captured && shell.is_event_captured() && state.remaining != Vector::ZERO {
                    state.remaining = Vector::ZERO;
                    state.last_frame = None;
                }
            }
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
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
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
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}
