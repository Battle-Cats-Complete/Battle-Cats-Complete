use std::cell::Cell;
use std::rc::Rc;
use std::slice;

use iced::advanced::{layout, mouse, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::animation::Easing;
use iced::time::{Duration, Instant};
use iced::{window, Animation, Element, Event, Length, Rectangle, Size, Theme, Vector};

const FADE_IN: Duration = Duration::from_millis(90);
const FADE_OUT: Duration = Duration::from_millis(140);
const FADE_EASING: Easing = Easing::EaseOut;

#[derive(Clone, Default)]
pub(crate) struct Reveal(Rc<Cell<f32>>);

impl Reveal {
    pub(crate) fn get(&self) -> f32 {
        self.0.get()
    }
}

pub(crate) fn fade<'a, Message>(
    build: impl FnOnce(Reveal) -> Element<'a, Message>,
) -> Fading<'a, Message> {
    let reveal = Reveal::default();

    Fading { content: build(reveal.clone()), reveal }
}

pub(crate) struct Fading<'a, Message> {
    content: Element<'a, Message>,
    reveal: Reveal,
}

struct FadeState {
    animation: Animation<bool>,
    was_animating: bool,
}

impl<'a, Message: 'a> From<Fading<'a, Message>> for Element<'a, Message> {
    fn from(widget: Fading<'a, Message>) -> Self {
        Element::new(widget)
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Fading<'_, Message> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<FadeState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(FadeState {
            animation: Animation::new(false).easing(FADE_EASING).duration(FADE_OUT),
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
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_ref::<FadeState>();

        self.reveal.0.set(state.animation.interpolate(0.0, 1.0, Instant::now()).clamp(0.0, 1.0));

        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
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
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let Event::Window(window::Event::RedrawRequested(now)) = event else {
            return;
        };

        let state = tree.state.downcast_mut::<FadeState>();
        let over = cursor.is_over(layout.bounds());

        if state.animation.value() != over {
            let span = if over { FADE_IN } else { FADE_OUT };

            state.animation = state.animation.clone().duration(span);
            state.animation.go_mut(over, *now);
        }

        self.reveal.0.set(state.animation.interpolate(0.0, 1.0, *now).clamp(0.0, 1.0));

        let animating = state.animation.is_animating(*now);

        if animating || state.was_animating {
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
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}
