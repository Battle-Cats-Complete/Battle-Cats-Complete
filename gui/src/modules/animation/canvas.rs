use std::ops::Range;
use std::sync::Arc;

use iced::mouse;
use iced::wgpu;
use iced::widget::shader::{self, Shader};
use iced::{Element, Event, Length, Point, Rectangle, Vector};
use image::RgbaImage;

use nyanko::graphics::engine::{resolve_frame, FrameData};

use core::modules::animation::multiply_mat3;

use super::data;

const ZOOM_MIN: f32 = 0.1;
const ZOOM_MAX: f32 = 10.0;
const FRAME_ADVANCE_PER_TICK: f32 = 0.5;

const VERTEX_STRIDE: u64 = 20;
const VERTS_PER_PART: u32 = 6;
const FLOATS_PER_PART: usize = 30;

const SHADER_SOURCE: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(input.position, 0.0, 1.0);
    out.uv = input.uv;
    out.opacity = input.opacity;
    return out;
}

@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(atlas, atlas_sampler, input.uv) * input.opacity;
}
"#;

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
                    let range_end = self.loop_end.or_else(|| data.current_anim.as_ref().map(|anim| anim.max_frame.max(0) as f32));

                    match range_end {
                        Some(end) if self.current_frame > end => self.current_frame = range_start,
                        None if self.current_frame > 0.0 && data.current_anim.is_none() => self.current_frame = 0.0,
                        _ => {}
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

pub struct Scene {
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

        let mut vertex_data = Vec::with_capacity(self.parts.len() * FLOATS_PER_PART);

        for (part_index, part) in self.parts.iter().enumerate() {
            let mvp = multiply_mat3(&view_proj, &part.final_matrix);

            for i in 0..VERTS_PER_PART as usize {
                let x = part.vertices[2 * i];
                let y = part.vertices[2 * i + 1];

                vertex_data.push(mvp[0] * x + mvp[3] * y + mvp[6]);
                vertex_data.push(mvp[1] * x + mvp[4] * y + mvp[7]);
                vertex_data.push(part.uvs[2 * i]);
                vertex_data.push(part.uvs[2 * i + 1]);
                vertex_data.push(part.opacity);
            }

            let variant = blend_variant(part.glow);
            let start = part_index as u32 * VERTS_PER_PART;

            match pipeline.batches.last_mut() {
                Some(batch) if batch.variant == variant => batch.range.end = start + VERTS_PER_PART,
                _ => pipeline.batches.push(Batch { variant, range: start..start + VERTS_PER_PART }),
            }
        }

        let bytes: &[u8] = bytemuck::cast_slice(&vertex_data);
        let needed = bytes.len() as u64;

        if pipeline.vertices.is_none() || pipeline.vertex_capacity < needed {
            pipeline.vertices = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("animation_viewer_vertices"),
                size: needed,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            pipeline.vertex_capacity = needed;
        }

        if let Some(buffer) = &pipeline.vertices {
            queue.write_buffer(buffer, 0, bytes);
        }
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

struct Batch {
    variant: usize,
    range: Range<u32>,
}

struct AtlasBinding {
    bind_group: wgpu::BindGroup,
    image_id: usize,
}

pub struct Pipeline {
    pipelines: [wgpu::RenderPipeline; 4],
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    atlas: Option<AtlasBinding>,
    vertices: Option<wgpu::Buffer>,
    vertex_capacity: u64,
    batches: Vec<Batch>,
}

impl shader::Pipeline for Pipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("animation_viewer"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("animation_viewer_atlas"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("animation_viewer"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipelines = blend_modes().map(|blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("animation_viewer"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: VERTEX_STRIDE,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32],
                    }],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..wgpu::PrimitiveState::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            })
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("animation_viewer_atlas"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });

        Self {
            pipelines,
            sampler,
            bind_group_layout,
            atlas: None,
            vertices: None,
            vertex_capacity: 0,
            batches: Vec::new(),
        }
    }
}

impl Pipeline {
    fn upload_atlas(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, image: &Arc<RgbaImage>) {
        let image_id = Arc::as_ptr(image) as usize;

        if self.atlas.as_ref().is_some_and(|atlas| atlas.image_id == image_id) {
            return;
        }

        let size = wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("animation_viewer_atlas"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            image.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * image.width()),
                rows_per_image: Some(image.height()),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("animation_viewer_atlas"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.atlas = Some(AtlasBinding { bind_group, image_id });
    }
}

fn blend_variant(glow: u8) -> usize {
    match glow {
        1 => 1,
        2 => 2,
        3 => 3,
        _ => 0,
    }
}

fn blend_modes() -> [wgpu::BlendState; 4] {
    let keep_dst_alpha = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    };

    let premultiplied = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    };

    [
        wgpu::BlendState {
            color: premultiplied,
            alpha: premultiplied,
        },
        wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: keep_dst_alpha,
        },
        wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Dst,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: keep_dst_alpha,
        },
        wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrc,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: keep_dst_alpha,
        },
    ]
}
