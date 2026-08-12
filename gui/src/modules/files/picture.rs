use iced::advanced::image::{FilterMethod, Handle, Image, Renderer as _};
use iced::advanced::{layout, mouse, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::time::Instant;
use iced::{window, Element, Event, Length, Point, Rectangle, Size, Theme, Vector};
use image::imageops::{self, FilterType};
use image::RgbaImage;

use crate::widget::{BOOTSTRAP_DT, DECAY_RATE, EPSILON, LINE_PIXELS};

const ZOOM_MIN: f32 = 0.1;
const ZOOM_MAX: f32 = 10.0;
const ZOOM_RATE_PER_PIXEL: f32 = 0.004;
const WORLD_PADDING: f32 = 0.75;
const CRISP_MAGNIFICATION: f32 = 1.8;
const MIN_LEVEL: u32 = 128;

fn premultiply(pixels: &mut RgbaImage) {
    for pixel in pixels.pixels_mut() {
        let alpha = u32::from(pixel.0[3]);

        for channel in 0..3 {
            pixel.0[channel] = (u32::from(pixel.0[channel]) * alpha / 255) as u8;
        }
    }
}

fn straighten(pixels: &mut RgbaImage) {
    for pixel in pixels.pixels_mut() {
        let alpha = u32::from(pixel.0[3]);

        if alpha == 0 {
            continue;
        }

        for channel in 0..3 {
            pixel.0[channel] = (u32::from(pixel.0[channel]) * 255 / alpha).min(255) as u8;
        }
    }
}

pub(super) struct Source {
    levels: Vec<(Size, Handle)>,
}

impl Source {
    pub(super) fn new(bytes: Vec<u8>, width: u32, height: u32) -> Self {
        let mut levels = Vec::new();

        if width.max(height) > MIN_LEVEL
            && let Ok(decoded) = image::load_from_memory(&bytes)
        {
            let mut current = decoded.to_rgba8();
            premultiply(&mut current);

            while current.width().max(current.height()) > MIN_LEVEL {
                let half = ((current.width() / 2).max(1), (current.height() / 2).max(1));

                if half == (current.width(), current.height()) {
                    break;
                }

                current = imageops::resize(&current, half.0, half.1, FilterType::Triangle);

                let mut straight = current.clone();
                straighten(&mut straight);

                let size = Size::new(half.0 as f32, half.1 as f32);
                levels.push((size, Handle::from_rgba(half.0, half.1, straight.into_raw())));
            }
        }

        levels.insert(0, (Size::new(width as f32, height as f32), Handle::from_bytes(bytes)));

        Self { levels }
    }

    fn size(&self) -> Size {
        self.levels.first().map_or(Size::ZERO, |(size, _)| *size)
    }

    fn level(&self, target: f32) -> Option<&(Size, Handle)> {
        self.levels.iter().rev().find(|(size, _)| size.width >= target).or_else(|| self.levels.first())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Moved { center: Vector, zoom: f32 },
}

pub(super) struct State {
    center: Vector,
    zoom: f32,
}

impl Default for State {
    fn default() -> Self {
        Self { center: Vector::ZERO, zoom: 1.0 }
    }
}

impl State {
    pub(super) fn reset(&mut self) {
        self.center = Vector::ZERO;
        self.zoom = 1.0;
    }

    pub(super) fn update(&mut self, message: Message) {
        let Message::Moved { center, zoom } = message;

        self.center = center;
        self.zoom = zoom;
    }

    pub(super) fn view<'a>(&self, source: &'a Source) -> Element<'a, Message> {
        Canvas { source, center: self.center, zoom: self.zoom }.into()
    }
}

struct Canvas<'a> {
    source: &'a Source,
    center: Vector,
    zoom: f32,
}

#[derive(Default)]
struct Interaction {
    drag_origin: Option<Point>,
    remaining: f32,
    last_frame: Option<Instant>,
}

impl Canvas<'_> {
    fn fit(&self, bounds: Rectangle) -> f32 {
        let native = self.source.size();

        if native.width <= 0.0 || native.height <= 0.0 || bounds.width <= 0.0 || bounds.height <= 0.0 {
            return 1.0;
        }

        (bounds.width / native.width).min(bounds.height / native.height).min(1.0)
    }

    fn frame(&self, bounds: Rectangle) -> Rectangle {
        let native = self.source.size();
        let scale = self.fit(bounds) * self.zoom;

        Rectangle {
            x: bounds.x + bounds.width / 2.0 - (native.width / 2.0 + self.center.x) * scale,
            y: bounds.y + bounds.height / 2.0 - (native.height / 2.0 + self.center.y) * scale,
            width: native.width * scale,
            height: native.height * scale,
        }
    }

    fn settle(&self, center: Vector) -> Vector {
        let native = self.source.size();
        let padding = native.width.max(native.height) * WORLD_PADDING;

        let horizontal = native.width / 2.0 + padding;
        let vertical = native.height / 2.0 + padding;

        Vector::new(center.x.clamp(-horizontal, horizontal), center.y.clamp(-vertical, vertical))
    }
}

impl<'a> From<Canvas<'a>> for Element<'a, Message> {
    fn from(canvas: Canvas<'a>) -> Self {
        Element::new(canvas)
    }
}

impl Widget<Message, Theme, iced::Renderer> for Canvas<'_> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<Interaction>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(Interaction::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&mut self, _tree: &mut widget::Tree, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let interaction: &mut Interaction = tree.state.downcast_mut();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_in(bounds) else {
                    return;
                };

                interaction.drag_origin = Some(position);
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if interaction.drag_origin.take().is_some() =>
            {
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let Some(origin) = interaction.drag_origin else {
                    return;
                };

                let local = Point::new(position.x - bounds.x, position.y - bounds.y);
                interaction.drag_origin = Some(local);

                let scale = self.fit(bounds) * self.zoom;

                if scale <= 0.0 {
                    return;
                }

                let travel = Vector::new((local.x - origin.x) / scale, (local.y - origin.y) / scale);
                let center = self.settle(self.center - travel);

                shell.publish(Message::Moved { center, zoom: self.zoom });
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !cursor.is_over(bounds) {
                    return;
                }

                let pixels = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y * LINE_PIXELS,
                    mouse::ScrollDelta::Pixels { y, .. } => *y,
                };

                if pixels == 0.0 {
                    return;
                }

                interaction.remaining += pixels;

                shell.capture_event();
                shell.request_redraw();
            }
            Event::Window(window::Event::RedrawRequested(now)) => {
                if interaction.remaining == 0.0 {
                    return;
                }

                let step = if interaction.remaining.abs() < EPSILON {
                    let step = interaction.remaining;
                    interaction.remaining = 0.0;
                    interaction.last_frame = None;
                    step
                } else {
                    let dt = interaction.last_frame.map_or(BOOTSTRAP_DT, |last| *now - last);
                    let alpha = 1.0 - (-DECAY_RATE * dt.as_secs_f32()).exp();
                    let step = interaction.remaining * alpha;

                    interaction.remaining -= step;
                    interaction.last_frame = Some(*now);
                    step
                };

                let zoom = (self.zoom * (step * ZOOM_RATE_PER_PIXEL).exp()).clamp(ZOOM_MIN, ZOOM_MAX);

                shell.publish(Message::Moved { center: self.center, zoom });

                if interaction.remaining != 0.0 {
                    shell.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };

        let frame = self.frame(bounds);

        if frame.width <= 0.0 || frame.height <= 0.0 {
            return;
        }

        let Some((level, handle)) = self.source.level(frame.width) else {
            return;
        };

        let magnification = if level.width > 0.0 { frame.width / level.width } else { 1.0 };

        let filter = if magnification >= CRISP_MAGNIFICATION {
            FilterMethod::Nearest
        } else {
            FilterMethod::Linear
        };

        renderer.draw_image(Image::new(handle.clone()).filter_method(filter), frame, clip);
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let interaction: &Interaction = tree.state.downcast_ref();

        if interaction.drag_origin.is_some() {
            return mouse::Interaction::Grabbing;
        }

        if cursor.is_over(layout.bounds()) {
            return mouse::Interaction::Grab;
        }

        mouse::Interaction::default()
    }
}
