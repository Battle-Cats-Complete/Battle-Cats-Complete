use iced::advanced::renderer;
use iced::advanced::{layout, mouse, overlay, widget, Clipboard, Layout, Shell, Widget};
use iced::{Element, Event, Length, Point, Rectangle, Size, Theme, Vector};

pub(crate) fn uniform_grid<'a, Message>(
    items: Vec<Element<'a, Message>>,
    spacing: f32,
) -> UniformGrid<'a, Message> {
    UniformGrid { items, spacing, columns: None }
}

pub(crate) struct UniformGrid<'a, Message> {
    items: Vec<Element<'a, Message>>,
    spacing: f32,
    columns: Option<usize>,
}

impl<Message> UniformGrid<'_, Message> {
    pub(crate) fn columns(mut self, columns: usize) -> Self {
        self.columns = Some(columns.max(1));

        self
    }
}

impl<'a, Message: 'a> From<UniformGrid<'a, Message>> for Element<'a, Message> {
    fn from(grid: UniformGrid<'a, Message>) -> Self {
        Element::new(grid)
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for UniformGrid<'_, Message> {
    fn children(&self) -> Vec<widget::Tree> {
        self.items.iter().map(widget::Tree::new).collect()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&self.items);
    }

    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Shrink }
    }

    fn layout(&mut self, tree: &mut widget::Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        if self.items.is_empty() {
            return layout::Node::new(Size::ZERO);
        }

        let available = limits.max().width;
        let columns = self.resolve_columns(tree, renderer, available);
        let column_width = (available - self.spacing * (columns as f32 - 1.0)) / columns as f32;

        let mut nodes: Vec<layout::Node> = Vec::with_capacity(self.items.len());
        let mut offset = 0.0;
        let mut cursor = 0;

        while cursor < self.items.len() {
            let end = (cursor + columns).min(self.items.len());
            let mut tallest: f32 = 0.0;

            for slot in cursor..end {
                let loose = layout::Limits::new(Size::new(column_width, 0.0), Size::new(column_width, f32::INFINITY));
                let node = self.items[slot].as_widget_mut().layout(&mut tree.children[slot], renderer, &loose);

                tallest = tallest.max(node.size().height);
                nodes.push(node);
            }

            let alone = end - cursor == 1;

            for (position, slot) in (cursor..end).enumerate() {
                let placed = Point::new(position as f32 * (column_width + self.spacing), offset);

                if alone {
                    nodes[slot] = std::mem::replace(&mut nodes[slot], layout::Node::new(Size::ZERO)).move_to(placed);
                    continue;
                }

                let fixed = layout::Limits::new(Size::new(column_width, tallest), Size::new(column_width, tallest));

                nodes[slot] = self.items[slot]
                    .as_widget_mut()
                    .layout(&mut tree.children[slot], renderer, &fixed)
                    .move_to(placed);
            }

            offset += tallest + self.spacing;
            cursor = end;
        }

        layout::Node::with_children(Size::new(available, (offset - self.spacing).max(0.0)), nodes)
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
        for ((item, state), child) in self.items.iter_mut().zip(tree.children.iter_mut()).zip(layout.children()) {
            item.as_widget_mut().update(state, event, child, cursor, renderer, clipboard, shell, viewport);
        }
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        for ((item, state), child) in self.items.iter_mut().zip(tree.children.iter_mut()).zip(layout.children()) {
            item.as_widget_mut().operate(state, child, renderer, operation);
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
        self.items
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .map(|((item, state), child)| item.as_widget().mouse_interaction(state, child, cursor, viewport, renderer))
            .max()
            .unwrap_or_default()
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
        for ((item, state), child) in self.items.iter().zip(tree.children.iter()).zip(layout.children()) {
            if child.bounds().intersects(viewport) {
                item.as_widget().draw(state, renderer, theme, style, child, cursor, viewport);
            }
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        overlay::from_children(&mut self.items, tree, layout, renderer, viewport, translation)
    }
}

impl<Message> UniformGrid<'_, Message> {
    fn resolve_columns(&mut self, tree: &mut widget::Tree, renderer: &iced::Renderer, available: f32) -> usize {
        if let Some(columns) = self.columns {
            return columns.min(self.items.len()).max(1);
        }

        let widest = self
            .items
            .iter_mut()
            .zip(tree.children.iter_mut())
            .map(|(item, state)| item.as_widget_mut().layout(state, renderer, &layout::Limits::NONE).size().width)
            .fold(0.0_f32, f32::max);

        if !widest.is_finite() || widest <= 0.0 {
            return 1;
        }

        let fits = ((available + self.spacing) / (widest + self.spacing)).floor().max(1.0) as usize;

        fits.min(self.items.len())
    }
}
