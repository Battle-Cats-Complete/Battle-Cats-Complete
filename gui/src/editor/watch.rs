use iced::advanced::{layout, mouse, overlay, renderer, widget, Clipboard, Layout, Renderer as _, Shell, Widget};
use iced::keyboard::{self, key::Named};
use iced::widget::Space;
use iced::{Element, Event, Length, Point, Rectangle, Size, Theme, Vector};

use super::{menu, target, Item, Message, State};

const CONTENT: usize = 0;
const MENU: usize = 1;

pub(crate) fn watch<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    state: &'a State,
    to_message: fn(Message) -> M,
) -> Element<'a, M> {
    let opened = state.open.as_ref();
    let armed = state.confirm.get().copied();

    let overlay: Element<'a, M> = opened.map_or_else(
        || Space::new().into(),
        |open| menu::view(&open.items, armed, to_message),
    );

    Element::new(Watch {
        layers: vec![content.into(), overlay],
        items: opened.map_or(&[], |open| open.items.as_slice()),
        anchor: opened.map_or(Point::ORIGIN, |open| open.at),
        open: opened.is_some(),
        to_message,
    })
}

struct Watch<'a, M> {
    layers: Vec<Element<'a, M>>,
    items: &'a [Item],
    anchor: Point,
    open: bool,
    to_message: fn(Message) -> M,
}

impl<'a, M: Clone> Widget<M, Theme, iced::Renderer> for Watch<'a, M> {
    fn children(&self) -> Vec<widget::Tree> {
        self.layers.iter().map(widget::Tree::new).collect()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&self.layers);
    }

    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Fill }
    }

    fn layout(&mut self, tree: &mut widget::Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let bounds = limits.max();

        let content = self.layers[CONTENT]
            .as_widget_mut()
            .layout(&mut tree.children[CONTENT], renderer, limits);

        let capped = if self.open {
            let natural = menu::measure(renderer, self.items);

            Size::new(natural.width.min(bounds.width), natural.height.min(bounds.height))
        } else {
            Size::ZERO
        };

        let overlay = self.layers[MENU].as_widget_mut().layout(
            &mut tree.children[MENU],
            renderer,
            &layout::Limits::new(Size::new(capped.width, 0.0), capped),
        );

        let placed = overlay.size();
        let overlay = overlay.move_to(anchored(self.anchor, placed, bounds));

        layout::Node::with_children(bounds, vec![content, overlay])
    }

    fn operate(&mut self, tree: &mut widget::Tree, layout: Layout<'_>, renderer: &iced::Renderer, operation: &mut dyn widget::Operation) {
        for ((layer, state), child) in self.layers.iter_mut().zip(&mut tree.children).zip(layout.children()) {
            layer.as_widget_mut().operate(state, child, renderer, operation);
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
        let mut children = layout.children();
        let (Some(content), Some(overlay)) = (children.next(), children.next()) else {
            return;
        };

        let inside = cursor.is_over(overlay.bounds());
        let right_press = matches!(event, Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)));
        let relocating = self.open && right_press && !inside;

        if self.open {
            self.layers[MENU].as_widget_mut().update(
                &mut tree.children[MENU],
                event,
                overlay,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );

            if !relocating && dismisses(event, inside) {
                shell.publish((self.to_message)(Message::Dismissed));
                shell.capture_event();
                shell.request_redraw();
            }

            if matches!(event, Event::Keyboard(_)) {
                return;
            }
        }

        let blanked = self.open && !relocating;

        self.layers[CONTENT].as_widget_mut().update(
            &mut tree.children[CONTENT],
            event,
            content,
            if blanked { mouse::Cursor::Unavailable } else { cursor },
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if blanked {
            if is_input(event) {
                shell.request_redraw();
            }

            return;
        }

        if !right_press {
            return;
        }

        if target::vetoed() {
            let _ = target::take();

            return;
        }

        let Some(position) = cursor.position() else {
            return;
        };

        shell.publish((self.to_message)(Message::Opened(position, target::take())));
        shell.capture_event();
        shell.request_redraw();
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let slot = if self.open { MENU } else { CONTENT };

        layout.children().nth(slot).map_or(mouse::Interaction::None, |child| {
            self.layers[slot]
                .as_widget()
                .mouse_interaction(&tree.children[slot], child, cursor, viewport, renderer)
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
        let mut children = layout.children();
        let (Some(content), Some(overlay)) = (children.next(), children.next()) else {
            return;
        };

        let base = if self.open { mouse::Cursor::Unavailable } else { cursor };

        self.layers[CONTENT]
            .as_widget()
            .draw(&tree.children[CONTENT], renderer, theme, style, content, base, viewport);

        if !self.open {
            return;
        }

        renderer.with_layer(*viewport, |renderer| {
            self.layers[MENU]
                .as_widget()
                .draw(&tree.children[MENU], renderer, theme, style, overlay, cursor, viewport);
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, iced::Renderer>> {
        overlay::from_children(&mut self.layers, tree, layout, renderer, viewport, translation)
    }
}

fn is_input(event: &Event) -> bool {
    matches!(event, Event::Mouse(_) | Event::Keyboard(_) | Event::Touch(_))
}

fn dismisses(event: &Event, inside: bool) -> bool {
    match event {
        Event::Mouse(mouse::Event::ButtonPressed(_)) => !inside,
        Event::Keyboard(keyboard::Event::KeyPressed { key: keyboard::Key::Named(Named::Escape), .. }) => true,
        _ => false,
    }
}

fn anchored(anchor: Point, size: Size, bounds: Size) -> Point {
    let flip = |start: f32, extent: f32, limit: f32| {
        if start + extent <= limit {
            return start;
        }

        (start - extent).max(0.0)
    };

    Point::new(flip(anchor.x, size.width, bounds.width), flip(anchor.y, size.height, bounds.height))
}
