use std::cell::Cell;
use std::collections::BTreeMap;

use iced::advanced::{layout, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::border::Radius;
use iced::mouse::{self, Interaction};
use iced::widget::{button, column, container, mouse_area, opaque, stack, text, Space};
use iced::{Alignment, Border, Color, Element, Event, Length, Padding, Point, Rectangle, Size, Theme, Vector};
use serde::{Deserialize, Serialize};

use crate::app::theme;

const HEADER_HEIGHT: f32 = 28.0;
const HEADER_MARGIN_X: f32 = 50.0;
const HEADER_MARGIN_Y: f32 = 30.0;
const DEFAULT_BODY_ALPHA: f32 = 1.0;
const FRAME_BORDER_WIDTH: f32 = 3.0;
const MINIMUM_WINDOW: Size = Size::new(800.0, 600.0);
const GRIP_OUTSET: f32 = 5.0;
const GRIP_INSET: f32 = FRAME_BORDER_WIDTH;
const GRIP_BAND: f32 = GRIP_OUTSET + GRIP_INSET;
const GRIP_CORNER: f32 = 14.0;
const HIGHLIGHT_CORNER: f32 = 26.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    CatFilter,
    EnemyFilter,
    StageFilter,
    ModImport,
    ModExport,
    AnimationExport,
    Keys,
    Pem,
    Exceptions,
    Changelog,
    Notice,
    Updater,
    InitErrors,
}

const KIND_COUNT: usize = 13;

const KINDS: [Kind; KIND_COUNT] = [
    Kind::CatFilter,
    Kind::EnemyFilter,
    Kind::StageFilter,
    Kind::ModImport,
    Kind::ModExport,
    Kind::AnimationExport,
    Kind::Keys,
    Kind::Pem,
    Kind::Exceptions,
    Kind::Changelog,
    Kind::Notice,
    Kind::Updater,
    Kind::InitErrors,
];

impl Kind {
    fn key(self) -> &'static str {
        match self {
            Self::CatFilter => "cat_filter",
            Self::EnemyFilter => "enemy_filter",
            Self::StageFilter => "stage_filter",
            Self::ModImport => "mod_import",
            Self::ModExport => "mod_export",
            Self::AnimationExport => "animation_export",
            Self::Keys => "keys",
            Self::Pem => "pem",
            Self::Exceptions => "exceptions",
            Self::Changelog => "changelog",
            Self::Notice => "notice",
            Self::Updater => "updater",
            Self::InitErrors => "init_errors",
        }
    }

    fn slot(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy)]
pub struct Spec {
    kind: Kind,
    minimum: Size,
}

impl Spec {
    pub const fn new(kind: Kind, minimum: Size) -> Self {
        Self { kind, minimum }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

const EDGES: [Edge; 8] = [
    Edge::Top,
    Edge::Bottom,
    Edge::Left,
    Edge::Right,
    Edge::TopLeft,
    Edge::TopRight,
    Edge::BottomLeft,
    Edge::BottomRight,
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Start,
    End,
}

impl Edge {
    fn horizontal(self) -> Option<Side> {
        match self {
            Self::Left | Self::TopLeft | Self::BottomLeft => Some(Side::Start),
            Self::Right | Self::TopRight | Self::BottomRight => Some(Side::End),
            Self::Top | Self::Bottom => None,
        }
    }

    fn vertical(self) -> Option<Side> {
        match self {
            Self::Top | Self::TopLeft | Self::TopRight => Some(Side::Start),
            Self::Bottom | Self::BottomLeft | Self::BottomRight => Some(Side::End),
            Self::Left | Self::Right => None,
        }
    }

    fn interaction(self) -> Interaction {
        match self {
            Self::Left | Self::Right => Interaction::ResizingHorizontally,
            Self::Top | Self::Bottom => Interaction::ResizingVertically,
            Self::TopLeft | Self::BottomRight => Interaction::ResizingDiagonallyDown,
            Self::TopRight | Self::BottomLeft => Interaction::ResizingDiagonallyUp,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub(crate) struct StoredSize {
    pub width: f32,
    pub height: f32,
}

thread_local! {
    static SIZES: Cell<[Option<Size>; KIND_COUNT]> = const { Cell::new([None; KIND_COUNT]) };
}

fn stored_size(kind: Kind) -> Option<Size> {
    SIZES.with(|sizes| sizes.get()[kind.slot()])
}

fn store_size(kind: Kind, size: Option<Size>) {
    SIZES.with(|sizes| {
        let mut slots = sizes.get();
        slots[kind.slot()] = size;
        sizes.set(slots);
    });
}

pub(crate) fn snapshot() -> BTreeMap<String, StoredSize> {
    KINDS
        .into_iter()
        .filter_map(|kind| {
            let size = stored_size(kind)?;

            Some((kind.key().to_owned(), StoredSize { width: size.width, height: size.height }))
        })
        .collect()
}

pub(crate) fn restore(stored: &BTreeMap<String, StoredSize>) {
    for kind in KINDS {
        let Some(entry) = stored.get(kind.key()).filter(|entry| entry.width.is_finite() && entry.height.is_finite()) else {
            continue;
        };

        store_size(kind, Some(Size::new(entry.width, entry.height)));
    }
}

#[derive(Default)]
pub struct State {
    position: Option<Point>,
    drag: Drag,
    hovered: Option<Edge>,
}

#[derive(Default, Clone, Copy)]
enum Drag {
    #[default]
    Idle,
    Pressed,
    Moving {
        last: Point,
    },
    Grabbed {
        edge: Edge,
    },
    Resizing {
        edge: Edge,
        origin: Point,
        position: Point,
        size: Size,
    },
}

#[derive(Debug, Clone)]
pub enum Message {
    HeaderPressed,
    EdgePressed(Edge),
    EdgeEntered(Edge),
    EdgeExited(Edge),
    Dragged(Point, Size),
    Released,
    Close,
}

impl State {
    pub fn update(&mut self, message: Message, spec: Spec) -> bool {
        match message {
            Message::HeaderPressed => self.drag = Drag::Pressed,
            Message::EdgePressed(edge) => self.drag = Drag::Grabbed { edge },
            Message::EdgeEntered(edge) => self.hovered = Some(edge),
            Message::EdgeExited(edge) => {
                if self.hovered == Some(edge) {
                    self.hovered = None;
                }
            }
            Message::Dragged(cursor, window) => match self.drag {
                Drag::Idle => {}
                Drag::Pressed => self.drag = Drag::Moving { last: cursor },
                Drag::Moving { last } => {
                    let (position, size) = self.resolved(spec, window);
                    let next = Point::new(position.x + cursor.x - last.x, position.y + cursor.y - last.y);
                    self.position = Some(clamp(next, size, window));
                    self.drag = Drag::Moving { last: cursor };
                }
                Drag::Grabbed { edge } => {
                    let (position, size) = self.resolved(spec, window);
                    self.drag = Drag::Resizing { edge, origin: cursor, position, size };
                }
                Drag::Resizing { edge, origin, position, size } => {
                    self.position = Some(resize(spec, edge, cursor - origin, position, size, window));
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
        spec: Spec,
        window: Size,
        to_message: fn(Message) -> M,
        content: impl Fn() -> Element<'a, M> + 'a,
        body_alpha: Option<f32>,
    ) -> Element<'a, M> {
        let body_alpha = body_alpha.unwrap_or(DEFAULT_BODY_ALPHA);

        let bounds = if window.width < 1.0 || window.height < 1.0 { MINIMUM_WINDOW } else { window };
        let (position, size) = self.resolved(spec, bounds);

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

        let frame = opaque(
            container(column![header, body])
                .width(Length::Fixed(size.width))
                .height(Length::Fixed(size.height))
                .padding(FRAME_BORDER_WIDTH)
                .style(frame_style),
        );

        let mut layers = vec![anchored(frame, position)];

        let lit = match self.drag {
            Drag::Grabbed { edge } | Drag::Resizing { edge, .. } => Some(edge),
            Drag::Idle => self.hovered,
            Drag::Pressed | Drag::Moving { .. } => None,
        };

        if let Some(edge) = lit {
            layers.extend(highlight(edge, position, size));
        }

        layers.extend(EDGES.map(|edge| grip(edge, position, size, to_message)));

        let mut drag_layer = mouse_area(Space::new().width(Length::Fill).height(Length::Fill));

        if !matches!(self.drag, Drag::Idle) {
            let interaction = match self.drag {
                Drag::Grabbed { edge } | Drag::Resizing { edge, .. } => edge.interaction(),
                _ => Interaction::Grabbing,
            };

            drag_layer = drag_layer
                .interaction(interaction)
                .on_move(move |cursor| to_message(Message::Dragged(cursor, bounds)))
                .on_release(to_message(Message::Released))
                .on_exit(to_message(Message::Released));
        }

        layers.push(drag_layer.into());

        stack(layers).into()
    }

    fn resolved(&self, spec: Spec, window: Size) -> (Point, Size) {
        let stored = stored_size(spec.kind).unwrap_or(spec.minimum);
        let size = Size::new(stored.width.max(spec.minimum.width), stored.height.max(spec.minimum.height));

        let centered = Point::new(
            ((window.width - size.width) / 2.0).max(0.0),
            ((window.height - size.height) / 2.0).max(0.0),
        );

        (clamp(self.position.unwrap_or(centered), size, window), size)
    }
}

fn resize(spec: Spec, edge: Edge, delta: Vector, position: Point, size: Size, window: Size) -> Point {
    let minimum = spec.minimum;
    let right = position.x + size.width;
    let bottom = position.y + size.height;

    let x = match edge.horizontal() {
        Some(Side::Start) => (position.x + delta.x).min(right - minimum.width),
        Some(Side::End) | None => position.x,
    };

    let y = match edge.vertical() {
        Some(Side::Start) => (position.y + delta.y).min(bottom - minimum.height).max(0.0),
        Some(Side::End) | None => position.y,
    };

    let width = match edge.horizontal() {
        Some(Side::Start) => right - x,
        Some(Side::End) => (size.width + delta.x).max(minimum.width),
        None => size.width,
    };

    let height = match edge.vertical() {
        Some(Side::Start) => bottom - y,
        Some(Side::End) => (size.height + delta.y).max(minimum.height),
        None => size.height,
    };

    let resized = Size::new(width, height);
    store_size(spec.kind, (resized != minimum).then_some(resized));

    clamp(Point::new(x, y), resized, window)
}

fn grip_bounds(edge: Edge, position: Point, size: Size) -> Rectangle {
    let (x, y, width, height) = (position.x, position.y, size.width, size.height);
    let far_x = x + width + GRIP_OUTSET - GRIP_CORNER;
    let far_y = y + height + GRIP_OUTSET - GRIP_CORNER;
    let corner = Size::new(GRIP_CORNER, GRIP_CORNER);

    match edge {
        Edge::Top => Rectangle::new(Point::new(x, y - GRIP_OUTSET), Size::new(width, GRIP_BAND)),
        Edge::Bottom => Rectangle::new(Point::new(x, y + height - GRIP_INSET), Size::new(width, GRIP_BAND)),
        Edge::Left => Rectangle::new(Point::new(x - GRIP_OUTSET, y), Size::new(GRIP_BAND, height)),
        Edge::Right => Rectangle::new(Point::new(x + width - GRIP_INSET, y), Size::new(GRIP_BAND, height)),
        Edge::TopLeft => Rectangle::new(Point::new(x - GRIP_OUTSET, y - GRIP_OUTSET), corner),
        Edge::TopRight => Rectangle::new(Point::new(far_x, y - GRIP_OUTSET), corner),
        Edge::BottomLeft => Rectangle::new(Point::new(x - GRIP_OUTSET, far_y), corner),
        Edge::BottomRight => Rectangle::new(Point::new(far_x, far_y), corner),
    }
}

fn grip<'a, M: Clone + 'a>(edge: Edge, position: Point, size: Size, to_message: fn(Message) -> M) -> Element<'a, M> {
    let bounds = grip_bounds(edge, position, size);

    let area = mouse_area(Space::new().width(bounds.width).height(bounds.height))
        .interaction(edge.interaction())
        .on_press(to_message(Message::EdgePressed(edge)))
        .on_enter(to_message(Message::EdgeEntered(edge)))
        .on_exit(to_message(Message::EdgeExited(edge)));

    anchored(area, bounds.position())
}

fn highlight<'a, M: 'a>(edge: Edge, position: Point, size: Size) -> Vec<Element<'a, M>> {
    let (x, y, width, height) = (position.x, position.y, size.width, size.height);
    let along_x = HIGHLIGHT_CORNER.min(width);
    let along_y = HIGHLIGHT_CORNER.min(height);

    let top = |length: f32| Rectangle::new(Point::new(x, y), Size::new(length, FRAME_BORDER_WIDTH));
    let bottom = |length: f32| Rectangle::new(Point::new(x, y + height - FRAME_BORDER_WIDTH), Size::new(length, FRAME_BORDER_WIDTH));
    let left = |length: f32| Rectangle::new(Point::new(x, y), Size::new(FRAME_BORDER_WIDTH, length));
    let right = |length: f32| Rectangle::new(Point::new(x + width - FRAME_BORDER_WIDTH, y), Size::new(FRAME_BORDER_WIDTH, length));
    let shift_x = |bounds: Rectangle| Rectangle { x: bounds.x + width - bounds.width, ..bounds };
    let shift_y = |bounds: Rectangle| Rectangle { y: bounds.y + height - bounds.height, ..bounds };

    let bars = match edge {
        Edge::Top => vec![top(width)],
        Edge::Bottom => vec![bottom(width)],
        Edge::Left => vec![left(height)],
        Edge::Right => vec![right(height)],
        Edge::TopLeft => vec![top(along_x), left(along_y)],
        Edge::TopRight => vec![shift_x(top(along_x)), right(along_y)],
        Edge::BottomLeft => vec![bottom(along_x), shift_y(left(along_y))],
        Edge::BottomRight => vec![shift_x(bottom(along_x)), shift_y(right(along_y))],
    };

    bars.into_iter().map(|bounds| anchored(bar(bounds.size()), bounds.position())).collect()
}

fn bar<'a, M: 'a>(size: Size) -> Element<'a, M> {
    container(Space::new())
        .width(Length::Fixed(size.width))
        .height(Length::Fixed(size.height))
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().primary.into()),
            ..container::Style::default()
        })
        .into()
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

fn header_color(theme: &Theme) -> Color {
    let palette = theme.palette();
    let shade = |c: f32| c * 0.7;

    Color {
        r: shade(palette.background.r),
        g: shade(palette.background.g),
        b: shade(palette.background.b),
        a: palette.background.a,
    }
}

fn inner_radius() -> f32 {
    (theme::RADIUS_MD - FRAME_BORDER_WIDTH).max(0.0)
}

fn header_style(theme: &Theme) -> container::Style {
    let radius = inner_radius();

    container::Style {
        background: Some(header_color(theme).into()),
        border: Border {
            radius: Radius { top_left: radius, top_right: radius, bottom_right: 0.0, bottom_left: 0.0 },
            ..Border::default()
        },
        ..container::Style::default()
    }
}

fn frame_style(theme: &Theme) -> container::Style {
    container::Style {
        border: Border {
            color: header_color(theme),
            width: FRAME_BORDER_WIDTH,
            radius: theme::RADIUS_MD.into(),
        },
        ..container::Style::default()
    }
}

fn body_style(theme: &Theme, alpha: f32) -> container::Style {
    let palette = theme.extended_palette();
    let background = Color { a: alpha, ..palette.background.base.color };
    let radius = inner_radius();

    container::Style {
        background: Some(background.into()),
        border: Border {
            radius: Radius { top_left: 0.0, top_right: 0.0, bottom_left: radius, bottom_right: radius },
            ..Border::default()
        },
        ..container::Style::default()
    }
}
