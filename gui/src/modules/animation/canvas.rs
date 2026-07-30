use std::sync::Arc;

use iced::mouse;
use iced::wgpu;
use iced::widget::shader::{self, Shader};
use iced::{Element, Event, Length, Point, Rectangle, Vector};
use image::RgbaImage;

use nyanko::graphics::engine::{resolve_frame, FrameData};

use core::modules::animation::multiply_mat3;

use super::data;
use super::pipeline::{build_vertices, Pipeline};

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
                    let range_end = self.loop_end.or_else(|| data.loop_bound().map(|v| v.max(0) as f32));

                    if let Some(end) = range_end
                        && self.current_frame > end {
                        self.current_frame = range_start;
                    }
                }
            }
        }
    }

    pub fn view<'a>(&'a self, data: &'a data::State) -> Element<'a, Message> {
        Shader::new(Viewport { state: self, data })
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

impl<'a> shader::Program<Message> for Viewport<'a> {
    type State = Interaction;
    type Primitive = Scene;

    fn update(
        &self,
        interaction: &mut Interaction,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<shader::Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                interaction.drag_origin = Some(position);
                Some(shader::Action::capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if interaction.drag_origin.take().is_some() {
                    Some(shader::Action::capture())
                } else {
                    None
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let origin = interaction.drag_origin?;
                let position = cursor.position_in(bounds)?;
                interaction.drag_origin = Some(position);
                let delta = position - origin;
                Some(shader::Action::publish(Message::Panned(delta)).and_capture())
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
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
                Some(shader::Action::publish(Message::Zoomed(factor)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, _interaction: &Interaction, _cursor: mouse::Cursor, _bounds: Rectangle) -> Scene {
        let (parts, image) = match &self.data.held_unit {
            Some(unit) => (
                resolve_frame(unit, self.data.current_anim.as_deref(), self.state.current_frame),
                unit.sheet.image_data.clone(),
            ),
            None => (Vec::new(), None),
        };

        Scene {
            image,
            parts,
            pan: self.state.pan,
            zoom: self.state.zoom,
        }
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

struct Scene {
    image: Option<Arc<RgbaImage>>,
    parts: Vec<FrameData>,
    pan: Vector,
    zoom: f32,
}

impl std::fmt::Debug for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scene")
            .field("parts", &self.parts.len())
            .field("pan", &self.pan)
            .field("zoom", &self.zoom)
            .finish()
    }
}

impl shader::Primitive for Scene {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipeline: &mut Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &shader::Viewport,
    ) {
        pipeline.batches.clear();

        let Some(image) = &self.image else {
            return;
        };

        if self.parts.is_empty() {
            return;
        }

        let physical = viewport.physical_size();
        let window_width = physical.width as f32;
        let window_height = physical.height as f32;

        if window_width <= 0.0 || window_height <= 0.0 {
            return;
        }

        pipeline.upload_atlas(device, queue, image);

        let scale = viewport.scale_factor();
        let zoom = self.zoom * scale;

        let projection = [
            2.0 / window_width, 0.0, 0.0,
            0.0, -2.0 / window_height, 0.0,
            -1.0, 1.0, 1.0,
        ];

        let camera = [
            zoom, 0.0, 0.0,
            0.0, zoom, 0.0,
            (bounds.x + bounds.width / 2.0) * scale + self.pan.x * zoom,
            (bounds.y + bounds.height / 2.0) * scale + self.pan.y * zoom,
            1.0,
        ];

        let view_proj = multiply_mat3(&projection, &camera);

        let (vertex_data, batches) = build_vertices(&self.parts, &view_proj);
        pipeline.batches = batches;
        pipeline.upload_vertices(device, queue, &vertex_data);
    }

    fn render(
        &self,
        pipeline: &Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        if pipeline.batches.is_empty() || clip_bounds.width == 0 || clip_bounds.height == 0 {
            return;
        }

        let Some(atlas) = &pipeline.atlas else {
            return;
        };

        let Some(vertices) = &pipeline.vertices else {
            return;
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("animation_viewer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_scissor_rect(clip_bounds.x, clip_bounds.y, clip_bounds.width, clip_bounds.height);
        pass.set_bind_group(0, &atlas.bind_group, &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));

        for batch in &pipeline.batches {
            pass.set_pipeline(&pipeline.pipelines[batch.variant]);
            pass.draw(batch.range.clone(), 0..1);
        }
    }
}

