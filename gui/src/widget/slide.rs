use std::slice;

use iced::advanced::renderer::{self, Renderer as _};
use iced::advanced::{layout, mouse, overlay, widget, Clipboard, Layout, Shell, Widget};
use iced::animation::Easing;
use iced::time::{Duration, Instant};
use iced::{window, Animation, Element, Event, Length, Point, Rectangle, Size, Theme, Vector};

const SLIDE_DURATION: Duration = Duration::from_millis(260);
const SLIDE_EASING: Easing = Easing::EaseOut;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slide {
    Left,
    Right,
    Up,
    Down,
}

impl Slide {
    fn is_horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

pub(crate) fn slide<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    open: bool,
    direction: Slide,
) -> Sliding<'a, Message> {
    Sliding { content: content.into(), open, direction }
}

pub(crate) struct Sliding<'a, Message> {
    content: Element<'a, Message>,
    open: bool,
    direction: Slide,
}

impl<'a, Message: 'a> From<Sliding<'a, Message>> for Element<'a, Message> {
    fn from(widget: Sliding<'a, Message>) -> Self {
        Element::new(widget)
    }
}

struct SlideState {
    animation: Animation<bool>,
    factor: f32,
    was_animating: bool,
}

impl<'a, Message> Widget<Message, Theme, iced::Renderer> for Sliding<'a, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<SlideState>()
    }

    fn state(&self) -> widget::tree::State {
        let factor = if self.open { 1.0 } else { 0.0 };

        widget::tree::State::new(SlideState {
            animation: Animation::new(self.open).easing(SLIDE_EASING).duration(SLIDE_DURATION),
            factor,
            was_animating: false,
        })
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        let inner = self.content.as_widget().size();

        if self.direction.is_horizontal() {
            Size { width: Length::Shrink, height: inner.height }
        } else {
            Size { width: inner.width, height: Length::Shrink }
        }
    }

    fn layout(&mut self, tree: &mut widget::Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let state: &mut SlideState = tree.state.downcast_mut();
        let now = Instant::now();

        if state.animation.value() != self.open {
            state.animation.go_mut(self.open, now);
        }

        let factor = state.animation.interpolate(0.0, 1.0, now).clamp(0.0, 1.0);
        state.factor = factor;

        let node = self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits);
        let natural = node.size();

        let (size, offset) = if self.direction.is_horizontal() {
            let visible = natural.width * factor;
            let x = if self.direction == Slide::Left { visible - natural.width } else { 0.0 };

            (Size::new(visible, natural.height), Point::new(x, 0.0))
        } else {
            let visible = natural.height * factor;
            let y = if self.direction == Slide::Up { visible - natural.height } else { 0.0 };

            (Size::new(natural.width, visible), Point::new(0.0, y))
        };

        layout::Node::with_children(size, vec![node.move_to(offset)])
    }

    fn operate(&mut self, tree: &mut widget::Tree, layout: Layout<'_>, renderer: &iced::Renderer, operation: &mut dyn widget::Operation) {
        let state: &SlideState = tree.state.downcast_ref();

        if state.factor <= 0.0 {
            return;
        }

        let Some(child) = layout.children().next() else {
            return;
        };

        self.content.as_widget_mut().operate(&mut tree.children[0], child, renderer, operation);
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
        let Some(child) = layout.children().next() else {
            return;
        };

        let Event::Window(window::Event::RedrawRequested(now)) = event else {
            let state: &SlideState = tree.state.downcast_ref();

            if state.factor < 1.0 {
                return;
            }

            self.content
                .as_widget_mut()
                .update(&mut tree.children[0], event, child, cursor, renderer, clipboard, shell, viewport);
            return;
        };

        self.content
            .as_widget_mut()
            .update(&mut tree.children[0], event, child, cursor, renderer, clipboard, shell, viewport);

        let state: &mut SlideState = tree.state.downcast_mut();
        let animating = state.animation.is_animating(*now);

        if animating || state.was_animating {
            shell.invalidate_layout();
        }

        if animating {
            shell.request_redraw();
        }

        state.was_animating = animating;
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state: &SlideState = tree.state.downcast_ref();

        if state.factor < 1.0 {
            return mouse::Interaction::None;
        }

        layout.children().next().map_or(mouse::Interaction::None, |child| {
            self.content.as_widget().mouse_interaction(&tree.children[0], child, cursor, viewport, renderer)
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
        let state: &SlideState = tree.state.downcast_ref();

        if state.factor <= 0.0 {
            return;
        }

        let Some(child) = layout.children().next() else {
            return;
        };

        if state.factor >= 1.0 {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, child, cursor, viewport);
            return;
        }

        let Some(clipped) = layout.bounds().intersection(viewport) else {
            return;
        };

        renderer.with_layer(clipped, |renderer| {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, child, cursor, &clipped);
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        let state: &SlideState = tree.state.downcast_ref();

        if state.factor < 1.0 {
            return None;
        }

        self.content
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout.children().next()?, renderer, viewport, translation)
    }
}
