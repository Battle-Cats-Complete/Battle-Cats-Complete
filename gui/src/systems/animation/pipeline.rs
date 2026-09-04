use std::ops::Range;
use std::sync::Arc;

use iced::wgpu;
use iced::widget::shader;
use image::RgbaImage;

use nyanko::graphics::animate::FrameData;

use kore::systems::animation::multiply_mat3;

const VERTEX_STRIDE: u64 = 36;
const VERTS_PER_PART: u32 = 4;
const INDICES_PER_PART: u32 = FrameData::INDICES.len() as u32;
const FLOATS_PER_PART: usize = 36;

const SHADER_SOURCE: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
    @location(3) tint: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
    @location(2) tint: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(input.position, 0.0, 1.0);
    out.uv = input.uv;
    out.opacity = input.opacity;
    out.tint = input.tint;
    return out;
}

@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(atlas, atlas_sampler, input.uv);
    let lit = dot(sampled.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let wash = mix(sampled.rgb, input.tint.rgb * lit, input.tint.a);

    return vec4<f32>(wash, sampled.a) * input.opacity;
}
"#;

pub(super) struct Batch {
    pub variant: usize,
    pub range: Range<u32>,
}

pub(super) struct AtlasBinding {
    pub bind_group: wgpu::BindGroup,
    image: Arc<RgbaImage>,
}

pub struct Pipeline {
    pub pipelines: [wgpu::RenderPipeline; 4],
    pub sampler: wgpu::Sampler,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub atlas: Option<AtlasBinding>,
    pub vertices: Option<wgpu::Buffer>,
    pub vertex_capacity: u64,
    pub indices: Option<wgpu::Buffer>,
    pub index_capacity: u64,
    pub batches: Vec<Batch>,
}

impl shader::Pipeline for Pipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Self::create(device, format)
    }
}

impl Pipeline {
    pub fn create(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
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
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32, 3 => Float32x4],
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
            indices: None,
            index_capacity: 0,
            batches: Vec::new(),
        }
    }

    pub fn upload_atlas(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, image: &Arc<RgbaImage>) {
        if self.atlas.as_ref().is_some_and(|atlas| Arc::ptr_eq(&atlas.image, image)) {
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

        self.atlas = Some(AtlasBinding { bind_group, image: image.clone() });
    }

    pub fn upload_vertices(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertex_data: &[f32]) {
        let bytes: &[u8] = bytemuck::cast_slice(vertex_data);
        let needed = bytes.len() as u64;

        if needed == 0 {
            return;
        }

        if self.vertices.is_none() || self.vertex_capacity < needed {
            self.vertices = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("animation_viewer_vertices"),
                size: needed,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.vertex_capacity = needed;
        }

        if let Some(buffer) = &self.vertices {
            queue.write_buffer(buffer, 0, bytes);
        }
    }

    pub fn upload_indices(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, index_data: &[u32]) {
        let bytes: &[u8] = bytemuck::cast_slice(index_data);
        let needed = bytes.len() as u64;

        if needed == 0 {
            return;
        }

        if self.indices.is_none() || self.index_capacity < needed {
            self.indices = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("animation_viewer_indices"),
                size: needed,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.index_capacity = needed;
        }

        if let Some(buffer) = &self.indices {
            queue.write_buffer(buffer, 0, bytes);
        }
    }
}

pub const NO_TINT: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

#[derive(Clone)]
pub struct Painted {
    pub frame: FrameData,
    pub tint: [f32; 4],
}

impl Painted {
    pub fn plain(frame: FrameData) -> Self {
        Self { frame, tint: NO_TINT }
    }
}

pub fn build_vertices(parts: &[Painted], view_proj: &[f32; 9]) -> (Vec<f32>, Vec<u32>, Vec<Batch>) {
    let mut vertex_data = Vec::with_capacity(parts.len() * FLOATS_PER_PART);
    let mut index_data = Vec::with_capacity(parts.len() * INDICES_PER_PART as usize);
    let mut batches: Vec<Batch> = Vec::new();

    for (part_index, painted) in parts.iter().enumerate() {
        let part = &painted.frame;
        let mvp = multiply_mat3(view_proj, &part.final_matrix);

        for corner in 0..VERTS_PER_PART as usize {
            let x = part.vertices[2 * corner];
            let y = part.vertices[2 * corner + 1];

            vertex_data.push(mvp[0] * x + mvp[3] * y + mvp[6]);
            vertex_data.push(mvp[1] * x + mvp[4] * y + mvp[7]);
            vertex_data.push(part.uvs[2 * corner]);
            vertex_data.push(part.uvs[2 * corner + 1]);
            vertex_data.push(part.opacity);
            vertex_data.extend_from_slice(&painted.tint);
        }

        let base = part_index as u32 * VERTS_PER_PART;
        index_data.extend(FrameData::INDICES.iter().map(|index| base + u32::from(*index)));

        let variant = blend_variant(part.glow);
        let start = part_index as u32 * INDICES_PER_PART;

        match batches.last_mut() {
            Some(batch) if batch.variant == variant => batch.range.end = start + INDICES_PER_PART,
            _ => batches.push(Batch { variant, range: start..start + INDICES_PER_PART }),
        }
    }

    (vertex_data, index_data, batches)
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
    let premultiplied = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    };

    let carried_alpha = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    };

    let color = |src_factor, dst_factor| wgpu::BlendComponent {
        src_factor,
        dst_factor,
        operation: wgpu::BlendOperation::Add,
    };

    [
        wgpu::BlendState {
            color: premultiplied,
            alpha: premultiplied
        },
        wgpu::BlendState {
            color: color(wgpu::BlendFactor::One, wgpu::BlendFactor::One),
            alpha: carried_alpha,
        },
        wgpu::BlendState {
            color: color(wgpu::BlendFactor::Dst, wgpu::BlendFactor::Zero),
            alpha: carried_alpha,
        },
        wgpu::BlendState {
            color: color(wgpu::BlendFactor::One, wgpu::BlendFactor::OneMinusSrc),
            alpha: carried_alpha,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::SHADER_SOURCE;

    #[test]
    fn the_shader_compiles_and_validates() {
        // A bad shader only shows up as a panic the first time the viewer draws, which reads
        // as "opening the page crashes". `filter` being a WGSL reserved word cost a release
        // exactly that way. Catch it here instead.
        let module = naga::front::wgsl::parse_str(SHADER_SOURCE).expect("the shader parses");

        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );

        validator.validate(&module).expect("the shader validates");
    }
}
