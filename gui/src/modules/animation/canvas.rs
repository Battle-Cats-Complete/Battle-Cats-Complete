use std::cell::RefCell;
use std::collections::HashMap;

use iced::mouse;
use iced::widget::canvas;
use iced::widget::canvas::{Cache, Geometry};
use iced::widget::image::Handle;
use iced::{Element, Length, Point, Radians, Rectangle, Renderer, Theme, Vector};
use image::imageops;

use nyanko::graphics::engine::resolve_frame;
use nyanko::graphics::rig::Unit;

use super::data;

const ZOOM_MIN: f32 = 0.1;
const ZOOM_MAX: f32 = 10.0;
const FRAME_ADVANCE_PER_TICK: f32 = 0.5;

#[derive(Debug, Clone)]
pub enum Message {
    Panned(Vector),
    Zoomed(f32),
    Tick,
}

pub struct State {
    pub pan: Vector,
    pub zoom: f32,
    pub current_frame: f32,
    pub is_playing: bool,
    pub playback_speed: f32,
    pub loop_start: Option<f32>,
    pub loop_end: Option<f32>,
    cache: Cache,
    sprite_cache: RefCell<HashMap<usize, Handle>>,
    cached_unit_id: Option<usize>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            pan: Vector::new(0.0, 0.0),
            zoom: 1.0,
            current_frame: 0.0,
            is_playing: true,
            playback_speed: 1.0,
            loop_start: None,
            loop_end: None,
            cache: Cache::new(),
            sprite_cache: RefCell::new(HashMap::new()),
            cached_unit_id: None,
        }
    }
}

impl State {
    pub fn update(&mut self, message: Message, data: &data::State) {
        match message {
            Message::Panned(delta) => {
                self.pan += Vector::new(delta.x / self.zoom, delta.y / self.zoom);
            }
            Message::Zoomed(factor) => {
                self.zoom = (self.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
            }
            Message::Tick => {
                if self.is_playing && data.held_unit.is_some() {
                    self.current_frame += FRAME_ADVANCE_PER_TICK * self.playback_speed;

                    let range_start = self.loop_start.unwrap_or(0.0);
                    let range_end = self.loop_end.or_else(|| data.current_anim.as_ref().map(|anim| anim.max_frame.max(0) as f32));

                    match range_end {
                        Some(end) if self.current_frame > end => self.current_frame = range_start,
                        None if self.current_frame > 0.0 && data.current_anim.is_none() => self.current_frame = 0.0,
                        _ => {}
                    }
                }
            }
        }

        self.cache.clear();
    }

    fn sync_sprite_cache(&self, unit: &Unit) {
        let unit_id = unit as *const Unit as usize;
        if self.cached_unit_id != Some(unit_id) {
            self.sprite_cache.borrow_mut().clear();
        }
    }

    fn sprite_handle(&self, unit: &Unit, sprite_index: usize) -> Option<Handle> {
        if let Some(cached) = self.sprite_cache.borrow().get(&sprite_index) {
            return Some(cached.clone());
        }

        let cut = unit.sheet.cuts_map.get(&sprite_index)?;
        let image_data = unit.sheet.image_data.as_ref()?;

        let width = image_data.width();
        let height = image_data.height();

        let px = (cut.uv_coordinates.min.x * width as f32).round() as u32;
        let py = (cut.uv_coordinates.min.y * height as f32).round() as u32;
        let pw = cut.original_size.x.round() as u32;
        let ph = cut.original_size.y.round() as u32;

        if pw == 0 || ph == 0 || px + pw > width || py + ph > height {
            return None;
        }

        let cropped = imageops::crop_imm(image_data.as_ref(), px, py, pw, ph).to_image();
        let handle = Handle::from_rgba(pw, ph, cropped.into_raw());
        self.sprite_cache.borrow_mut().insert(sprite_index, handle.clone());
        Some(handle)
    }

    pub fn view<'a>(&'a self, data: &'a data::State) -> Element<'a, Message> {
        if let Some(unit) = &data.held_unit {
            self.sync_sprite_cache(unit);
        }

        canvas(Viewport { state: self, data })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

struct Viewport<'a> {
    state: &'a State,
    data: &'a data::State,
}

#[derive(Default)]
struct Interaction {
    drag_origin: Option<Point>,
}

impl<'a> canvas::Program<Message> for Viewport<'a> {
    type State = Interaction;

    fn update(
        &self,
        interaction: &mut Interaction,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                interaction.drag_origin = Some(position);
                Some(canvas::Action::capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if interaction.drag_origin.take().is_some() {
                    Some(canvas::Action::capture())
                } else {
                    None
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let origin = interaction.drag_origin?;
                interaction.drag_origin = Some(*position);
                let delta = *position - origin;
                Some(canvas::Action::publish(Message::Panned(delta)).and_capture())
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if !cursor.is_over(bounds) {
                    return None;
                }

                let amount = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };

                if amount == 0.0 {
                    return None;
                }

                let factor = 1.0 + (amount * 0.1);
                Some(canvas::Action::publish(Message::Zoomed(factor)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _interaction: &Interaction,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let geometry = self.state.cache.draw(renderer, bounds.size(), |frame| {
            let Some(unit) = &self.data.held_unit else {
                return;
            };

            let center = frame.center();
            let origin = Vector::new(center.x + self.state.pan.x * self.state.zoom, center.y + self.state.pan.y * self.state.zoom);

            let parts = resolve_frame(unit, self.data.current_anim.as_deref(), self.state.current_frame);

            for part in &parts {
                let Some(handle) = self.state.sprite_handle(unit, part.sprite_index) else {
                    continue;
                };

                let (translation, angle, scale) = decompose(&part.final_matrix);

                let top_left_x = part.vertices[0];
                let top_left_y = part.vertices[1];
                let bottom_right_x = part.vertices[10];
                let bottom_right_y = part.vertices[11];
                let local_rect = Rectangle {
                    x: top_left_x,
                    y: top_left_y,
                    width: bottom_right_x - top_left_x,
                    height: bottom_right_y - top_left_y,
                };

                frame.with_save(|frame| {
                    frame.translate(origin);
                    frame.scale(self.state.zoom);
                    frame.translate(translation);
                    frame.rotate(Radians(angle));
                    frame.scale_nonuniform(scale);

                    let image = canvas::Image::new(handle).opacity(part.opacity);
                    frame.draw_image(local_rect, image);
                });
            }
        });

        vec![geometry]
    }

    fn mouse_interaction(&self, interaction: &Interaction, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if interaction.drag_origin.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

/// Decomposes nyanko's fully hierarchy-solved `final_matrix` into translate/rotate/scale.
///
/// The matrix is constructed in `nyanko::graphics::engine::transform::solve_single_part` as an
/// exact `T * R * S` composition (no shear), so this recovers the original components losslessly.
fn decompose(matrix: &[f32; 9]) -> (Vector, f32, Vector) {
    let (m0, m1, m3, m4, m6, m7) = (matrix[0], matrix[1], matrix[3], matrix[4], matrix[6], matrix[7]);

    let scale_x = (m0 * m0 + m1 * m1).sqrt();
    let angle = m1.atan2(m0);
    let (sin_a, cos_a) = angle.sin_cos();

    let scale_y = if cos_a.abs() > sin_a.abs() { m4 / cos_a } else { -m3 / sin_a };

    (Vector::new(m6, m7), angle, Vector::new(scale_x, scale_y))
}
