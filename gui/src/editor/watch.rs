use iced::advanced::{layout, mouse, overlay, renderer, widget, Clipboard, Layout, Renderer as _, Shell, Widget};
use iced::keyboard::{self, key::Named};
use iced::widget::Space;
use iced::{Element, Event, Length, Point, Rectangle, Size, Theme, Vector};

use super::{menu, target, Item, Message, State};

const CONTENT: usize = 0;
const MENU: usize = 1;
const SUB: usize = 2;
const SUBMENU_GAP: f32 = 2.0;

pub(crate) fn watch<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    state: &'a State,
    to_message: fn(Message) -> M,
) -> Element<'a, M> {
    let opened = state.open.as_ref();
    let armed = state.confirm.get().copied();
    let hovered = opened.and_then(|open| open.hovered);

    let overlay: Element<'a, M> = opened.map_or_else(
        || Space::new().into(),
        |open| menu::view(&open.items, armed, None, to_message),
    );

    let nested = opened
        .and_then(|open| open.hovered.and_then(|index| open.items.get(index)))
        .map(|item| item.children.as_slice())
        .filter(|kids| !kids.is_empty());

    let sub: Element<'a, M> = nested.map_or_else(
        || Space::new().into(),
        |kids| menu::view(kids, armed, hovered, to_message),
    );

    Element::new(Watch {
        layers: vec![content.into(), overlay, sub],
        items: opened.map_or(&[], |open| open.items.as_slice()),
        children: nested.unwrap_or(&[]),
        hovered,
        anchor: opened.map_or(Point::ORIGIN, |open| open.at),
        open: opened.is_some(),
        to_message,
    })
}

struct Watch<'a, M> {
    layers: Vec<Element<'a, M>>,
    items: &'a [Item],
    children: &'a [Item],
    hovered: Option<usize>,
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

        if !self.open {
            let menu = collapsed(&mut self.layers[MENU], &mut tree.children[MENU], renderer);
            let sub = collapsed(&mut self.layers[SUB], &mut tree.children[SUB], renderer);

            return layout::Node::with_children(bounds, vec![content, menu, sub]);
        }

        let main = clamped(menu::measure(renderer, self.items), bounds);
        let origin = anchored(self.anchor, main, bounds);
        let menu = sized(&mut self.layers[MENU], &mut tree.children[MENU], renderer, main).move_to(origin);

        let sub = if self.children.is_empty() {
            collapsed(&mut self.layers[SUB], &mut tree.children[SUB], renderer)
        } else {
            let size = clamped(menu::measure(renderer, self.children), bounds);
            let top = self.hovered.map_or(0.0, |index| menu::row_offset(renderer, self.items, index));

            sized(&mut self.layers[SUB], &mut tree.children[SUB], renderer, size)
                .move_to(beside(origin, main, size, top, bounds))
        };

        layout::Node::with_children(bounds, vec![content, menu, sub])
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
        let (Some(content), Some(overlay), Some(nested)) = (children.next(), children.next(), children.next())
        else {
            return;
        };

        let panels = if self.children.is_empty() {
            overlay.bounds()
        } else {
            overlay.bounds().union(&nested.bounds())
        };

        let inside = cursor.is_over(panels);
        let right_press = matches!(event, Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)));
        let relocating = self.open && right_press && !inside;

        if self.open {
            self.layers[SUB].as_widget_mut().update(
                &mut tree.children[SUB],
                event,
                nested,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );

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

            if self.hovered.is_some()
                && matches!(event, Event::Mouse(mouse::Event::CursorMoved { .. }))
                && !inside
            {
                shell.publish((self.to_message)(Message::Hovered(None)));
            }

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
        let slots: &[usize] = if self.open { &[SUB, MENU] } else { &[CONTENT] };

        slots
            .iter()
            .filter_map(|slot| layout.children().nth(*slot).map(|child| (*slot, child)))
            .map(|(slot, child)| {
                self.layers[slot]
                    .as_widget()
                    .mouse_interaction(&tree.children[slot], child, cursor, viewport, renderer)
            })
            .find(|interaction| *interaction != mouse::Interaction::None)
            .unwrap_or(mouse::Interaction::None)
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
        let (Some(content), Some(overlay), Some(nested)) = (children.next(), children.next(), children.next())
        else {
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

            self.layers[SUB]
                .as_widget()
                .draw(&tree.children[SUB], renderer, theme, style, nested, cursor, viewport);
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

fn clamped(size: Size, bounds: Size) -> Size {
    Size::new(size.width.min(bounds.width), size.height.min(bounds.height))
}

fn collapsed<'a, M>(element: &mut Element<'a, M>, tree: &mut widget::Tree, renderer: &iced::Renderer) -> layout::Node {
    element
        .as_widget_mut()
        .layout(tree, renderer, &layout::Limits::new(Size::ZERO, Size::ZERO))
}

fn sized<'a, M>(
    element: &mut Element<'a, M>,
    tree: &mut widget::Tree,
    renderer: &iced::Renderer,
    size: Size,
) -> layout::Node {
    element
        .as_widget_mut()
        .layout(tree, renderer, &layout::Limits::new(Size::new(size.width, 0.0), size))
}

fn beside(origin: Point, main: Size, size: Size, top: f32, bounds: Size) -> Point {
    let right = origin.x + main.width + SUBMENU_GAP;
    let left = origin.x - size.width - SUBMENU_GAP;
    let x = if right + size.width <= bounds.width || left < 0.0 { right } else { left };
    let y = (origin.y + top).min((bounds.height - size.height).max(0.0));

    Point::new(x, y)
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
