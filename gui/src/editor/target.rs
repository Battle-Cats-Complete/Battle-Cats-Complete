use std::cell::Cell;

use iced::advanced::{layout, mouse, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::{Element, Event, Length, Point, Rectangle, Size, Theme, Vector};

use super::Target;

thread_local! {
    static HIT: Cell<Option<Target>> = const { Cell::new(None) };
    static VETO: Cell<Veto> = const { Cell::new(Veto::None) };
}

#[derive(Clone, Copy, Default)]
enum Veto {
    #[default]
    None,
    Everywhere,
    Over(Rectangle),
}

pub(super) fn take() -> Option<Target> {
    HIT.with(Cell::take)
}

pub(super) fn vetoed(at: Option<Point>) -> bool {
    match VETO.with(Cell::take) {
        Veto::None => false,
        Veto::Everywhere => true,
        Veto::Over(bounds) => at.is_some_and(|at| bounds.contains(at)),
    }
}

fn claim(target: Target) {
    HIT.with(|hit| {
        if hit.get().is_none() {
            hit.set(Some(target));
        }
    });
}

pub(crate) fn target<'a, M: 'a>(content: impl Into<Element<'a, M>>, target: Target) -> Element<'a, M> {
    Element::new(Region { content: content.into(), target: Role::Claim(target) })
}

pub(crate) fn suppress<'a, M: 'a>(content: impl Into<Element<'a, M>>, active: bool) -> Element<'a, M> {
    Element::new(Region { content: content.into(), target: Role::Veto(active) })
}

pub(crate) fn deflect<'a, M: 'a>(content: impl Into<Element<'a, M>>, active: bool) -> Element<'a, M> {
    Element::new(Region { content: content.into(), target: Role::Deflect(active) })
}

#[derive(Clone, Copy)]
enum Role {
    Claim(Target),
    Veto(bool),
    Deflect(bool),
}

impl Role {
    fn apply(self, over: bool, bounds: Rectangle) {
        match self {
            Self::Claim(target) => {
                if over {
                    claim(target);
                }
            }
            Self::Veto(true) => VETO.with(|veto| veto.set(Veto::Everywhere)),
            Self::Deflect(true) => {
                VETO.with(|veto| {
                    if matches!(veto.get(), Veto::None) {
                        veto.set(Veto::Over(bounds));
                    }
                });
            }
            Self::Veto(false) | Self::Deflect(false) => {}
        }
    }
}

struct Region<'a, M> {
    content: Element<'a, M>,
    target: Role,
}

impl<'a, M> Widget<M, Theme, iced::Renderer> for Region<'a, M> {
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
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut widget::Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(&mut self, tree: &mut widget::Tree, layout: Layout<'_>, renderer: &iced::Renderer, operation: &mut dyn widget::Operation) {
        self.content.as_widget_mut().operate(tree, layout, renderer, operation);
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
        self.content
            .as_widget_mut()
            .update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);

        if !matches!(event, Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))) {
            return;
        }

        let bounds = layout.bounds();
        let over = bounds.intersection(viewport).is_some_and(|visible| cursor.is_over(visible));

        self.target.apply(over, bounds);
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer)
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
        self.content.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(tree, layout, renderer, viewport, translation)
    }
}
