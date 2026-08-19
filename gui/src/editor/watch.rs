use iced::advanced::{layout, mouse, overlay, renderer, widget, Clipboard, Layout, Renderer as _, Shell, Widget};
use iced::keyboard::{self, key::Named};
use iced::widget::Space;
use iced::{Element, Event, Length, Point, Rectangle, Size, Theme, Vector};

use super::{menu, panels, target, Item, Message, State};

const CONTENT: usize = 0;
const MENU: usize = 1;
const SUBMENU_GAP: f32 = 2.0 * menu::SCALE;

pub(crate) fn watch<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    state: &'a State,
    to_message: fn(Message) -> M,
) -> Element<'a, M> {
    let opened = state.open.as_ref();
    let marks = menu::Marks {
        armed: state.confirm.get().map(Vec::as_slice),
        failed: state.failed.get().map(Vec::as_slice),
    };

    let items = opened.map_or(&[][..], |open| open.items.as_slice());
    let trail = opened.map_or(&[][..], |open| open.trail.as_slice());
    let nested = panels(items, trail);

    let mut layers: Vec<Element<'a, M>> = Vec::with_capacity(nested.len() + 2);

    layers.push(content.into());

    match opened {
        Some(_) => {
            layers.push(menu::view(items, marks, &[], to_message));

            for (level, rows) in nested.iter().enumerate() {
                layers.push(menu::view(rows, marks, &trail[..=level], to_message));
            }
        }
        None => layers.push(Space::new().into()),
    }

    Element::new(Watch {
        layers,
        items,
        panels: nested,
        trail,
        anchor: opened.map_or(Point::ORIGIN, |open| open.at),
        open: opened.is_some(),
        to_message,
    })
}

struct Watch<'a, M> {
    layers: Vec<Element<'a, M>>,
    items: &'a [Item],
    panels: Vec<&'a [Item]>,
    trail: &'a [usize],
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
        let mut nodes = Vec::with_capacity(self.layers.len());

        nodes.push(self.layers[CONTENT].as_widget_mut().layout(&mut tree.children[CONTENT], renderer, limits));

        if !self.open {
            for slot in MENU..self.layers.len() {
                nodes.push(collapsed(&mut self.layers[slot], &mut tree.children[slot], renderer));
            }

            return layout::Node::with_children(bounds, nodes);
        }

        let main = clamped(menu::measure(renderer, self.items), bounds);
        let origin = anchored(self.anchor, main, bounds);

        nodes.push(sized(&mut self.layers[MENU], &mut tree.children[MENU], renderer, main).move_to(origin));

        let mut sizes = Vec::with_capacity(self.panels.len());

        for rows in &self.panels {
            sizes.push(clamped(menu::measure(renderer, rows), bounds));
        }

        let cascade = Cascade::choose(origin, main, &sizes, bounds);
        let mut parent = (self.items, origin, main);

        for level in 0..self.panels.len() {
            let rows = self.panels[level];
            let index = self.trail[level];
            let (above, at, span) = parent;

            let Some(size) = sizes.get(level).copied() else {
                break;
            };

            let placed = beside(cascade, at, span, size, menu::row_offset(renderer, above, index), bounds);
            let slot = MENU + 1 + level;

            nodes.push(sized(&mut self.layers[slot], &mut tree.children[slot], renderer, size).move_to(placed));

            parent = (rows, placed, size);
        }

        layout::Node::with_children(bounds, nodes)
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
        let Some(content) = layout.children().next() else {
            return;
        };

        let inside = self.open && hovering(layout, cursor);
        let right_press = matches!(event, Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)));
        let relocating = self.open && right_press && !inside;

        if self.open {
            for slot in (MENU..self.layers.len()).rev() {
                let (Some(layer), Some(state), Some(region)) =
                    (self.layers.get_mut(slot), tree.children.get_mut(slot), layout.children().nth(slot))
                else {
                    continue;
                };

                layer.as_widget_mut().update(state, event, region, cursor, renderer, clipboard, shell, viewport);
            }

            if !self.trail.is_empty()
                && matches!(event, Event::Mouse(mouse::Event::CursorMoved { .. }))
                && !inside
            {
                shell.publish((self.to_message)(Message::Hovered(Vec::new())));
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
        let reach = if self.open { MENU..self.layers.len() } else { CONTENT..MENU };

        reach
            .rev()
            .filter_map(|slot| {
                let layer = self.layers.get(slot)?;
                let state = tree.children.get(slot)?;
                let child = layout.children().nth(slot)?;

                Some(layer.as_widget().mouse_interaction(state, child, cursor, viewport, renderer))
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
        let Some(content) = layout.children().next() else {
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
            for ((layer, state), child) in
                self.layers.iter().zip(&tree.children).zip(layout.children()).skip(MENU)
            {
                layer.as_widget().draw(state, renderer, theme, style, child, cursor, viewport);
            }
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

fn hovering(layout: Layout<'_>, cursor: mouse::Cursor) -> bool {
    if layout.children().skip(MENU).any(|child| cursor.is_over(child.bounds())) {
        return true;
    }

    let mut previous = None;

    for child in layout.children().skip(MENU) {
        let bounds = child.bounds();

        if let Some(before) = previous.replace(bounds)
            && cursor.is_over(bridge(before, bounds))
        {
            return true;
        }
    }

    false
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

fn bridge(main: Rectangle, sub: Rectangle) -> Rectangle {
    let (x, width) = if sub.x >= main.x + main.width {
        (main.x + main.width, sub.x - main.x - main.width)
    } else {
        (sub.x + sub.width, main.x - sub.x - sub.width)
    };

    Rectangle { x, y: sub.y, width: width.max(0.0), height: sub.height }
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

#[derive(Clone, Copy)]
enum Cascade {
    Right,
    Left,
}

impl Cascade {
    fn choose(origin: Point, main: Size, sizes: &[Size], bounds: Size) -> Self {
        let span: f32 = sizes.iter().map(|size| size.width + SUBMENU_GAP).sum();

        if origin.x + main.width + span <= bounds.width {
            return Cascade::Right;
        }

        if origin.x - span >= 0.0 {
            return Cascade::Left;
        }

        if origin.x + main.width / 2.0 <= bounds.width / 2.0 { Cascade::Right } else { Cascade::Left }
    }
}

fn beside(cascade: Cascade, at: Point, span: Size, size: Size, top: f32, bounds: Size) -> Point {
    let x = match cascade {
        Cascade::Right => at.x + span.width + SUBMENU_GAP,
        Cascade::Left => at.x - size.width - SUBMENU_GAP,
    };

    let y = (at.y + top).min((bounds.height - size.height).max(0.0));

    Point::new(x.clamp(0.0, (bounds.width - size.width).max(0.0)), y)
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
