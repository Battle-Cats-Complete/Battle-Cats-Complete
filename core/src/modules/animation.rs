pub mod export;

use std::sync::Arc;

use glow::HasContext;
use nyanko::graphics::engine::FrameData;
use nyanko::graphics::rig::SpriteSheet;
use tracing::{error, info, trace};

pub const IDX_WALK: usize = 0;
pub const IDX_IDLE: usize = 1;
pub const IDX_ATTACK: usize = 2;
pub const IDX_KB: usize = 3;
pub const IDX_SPIRIT: usize = 4;
pub const IDX_BURROW: usize = 5;
pub const IDX_SURFACE: usize = 6;
pub const IDX_MODEL: usize = 99;
pub const IDX_NONE: usize = 999;

#[derive(Debug)]
pub enum CanvasError {
    ProgramCreation,
    VertexShaderCreation,
    FragmentShaderCreation,
    ShaderCompile(String),
    ProgramLink(String),
    VaoCreation,
    VboCreation,
    TboCreation,
    TextureAllocation,
}

impl std::fmt::Display for CanvasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CanvasError::ProgramCreation => write!(f, "Failed to create OpenGL program"),
            CanvasError::VertexShaderCreation => write!(f, "Failed to create Vertex Shader object"),
            CanvasError::FragmentShaderCreation => write!(f, "Failed to create Fragment Shader object"),
            CanvasError::ShaderCompile(msg) => write!(f, "Shader compilation failed: {}", msg),
            CanvasError::ProgramLink(msg) => write!(f, "Program linking failed: {}", msg),
            CanvasError::VaoCreation => write!(f, "Failed to create Vertex Array Object"),
            CanvasError::VboCreation => write!(f, "Failed to create Vertex Buffer Object"),
            CanvasError::TboCreation => write!(f, "Failed to create Texture Buffer Object"),
            CanvasError::TextureAllocation => write!(f, "Failed to allocate OpenGL texture"),
        }
    }
}

impl std::error::Error for CanvasError {}

const VERTEX_SHADER_SOURCE: &str = r#"
#ifdef GL_ES
precision lowp float;
#endif
in vec2 a_position;
in vec2 a_texcoord;

uniform mat3 u_transform;
uniform mat3 u_view_proj;

out vec2 v_texcoord;

void main() {
vec3 pos = u_view_proj * u_transform * vec3(a_position, 1.0);
gl_Position = vec4(pos.xy, 0.0, 1.0);
v_texcoord = a_texcoord;
}
"#;

const FRAGMENT_SHADER_SOURCE: &str = r#"
#ifdef GL_ES
precision lowp float;
#endif
uniform sampler2D u_texture;
uniform float u_opacity;

in vec2 v_texcoord;
out vec4 f_color;

void main() {
f_color = texture(u_texture, v_texcoord) * u_opacity;
}
"#;

pub struct GlowRenderer {
    program: glow::Program,
    vertex_array: glow::VertexArray,
    vbo: glow::Buffer,
    tbo: glow::Buffer,
    texture: Option<glow::Texture>,
    last_image_id: usize,
}

impl GlowRenderer {
    pub fn new(gl_context: &glow::Context) -> Result<Self, CanvasError> {
        unsafe {
            let program = compile_program(gl_context, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE)?;

            let vertex_array = gl_context.create_vertex_array().map_err(|_| {
                error!("Failed to create OpenGL Vertex Array Object");
                CanvasError::VaoCreation
            })?;

            let vbo = gl_context.create_buffer().map_err(|_| {
                error!("Failed to create OpenGL Vertex Buffer Object");
                CanvasError::VboCreation
            })?;

            let tbo = gl_context.create_buffer().map_err(|_| {
                error!("Failed to create OpenGL Texture Buffer Object");
                CanvasError::TboCreation
            })?;

            gl_context.bind_vertex_array(Some(vertex_array));

            gl_context.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let position_location = gl_context.get_attrib_location(program, "a_position").unwrap_or(0);
            gl_context.enable_vertex_attrib_array(position_location);
            gl_context.vertex_attrib_pointer_f32(position_location, 2, glow::FLOAT, false, 0, 0);

            gl_context.bind_buffer(glow::ARRAY_BUFFER, Some(tbo));
            let texture_location = gl_context.get_attrib_location(program, "a_texcoord").unwrap_or(1);
            gl_context.enable_vertex_attrib_array(texture_location);
            gl_context.vertex_attrib_pointer_f32(texture_location, 2, glow::FLOAT, false, 0, 0);

            gl_context.bind_vertex_array(None);

            info!("GlowRenderer successfully initialized.");

            Ok(Self {
                program,
                vertex_array,
                vbo,
                tbo,
                texture: None,
                last_image_id: 0,
            })
        }
    }

    fn upload_texture(&mut self, gl_context: &glow::Context, sheet: &SpriteSheet) -> Result<(), CanvasError> {
        unsafe {
            let Some(image) = &sheet.image_data else {
                trace!("No image data found in SpriteSheet; skipping texture upload.");
                return Ok(());
            };

            let current_image_id = Arc::as_ptr(image) as usize;

            if self.last_image_id == current_image_id && self.texture.is_some() {
                return Ok(());
            }

            let texture_id = if let Some(existing_texture) = self.texture {
                gl_context.bind_texture(glow::TEXTURE_2D, Some(existing_texture));
                existing_texture
            } else {
                let new_texture = gl_context.create_texture().map_err(|_| {
                    error!("Failed to allocate new OpenGL texture");
                    CanvasError::TextureAllocation
                })?;

                gl_context.bind_texture(glow::TEXTURE_2D, Some(new_texture));
                gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                gl_context.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);

                new_texture
            };

            let raw_pixels = image.as_raw();

            gl_context.tex_image_2d(
                glow::TEXTURE_2D, 0, glow::RGBA8 as i32,
                image.width() as i32, image.height() as i32, 0,
                glow::RGBA, glow::UNSIGNED_BYTE, Some(raw_pixels),
            );

            self.texture = Some(texture_id);
            self.last_image_id = current_image_id;

            trace!("Uploaded new texture with dimensions {}x{}", image.width(), image.height());

            Ok(())
        }
    }

    pub fn draw_frame(
        &mut self,
        gl_context: &glow::Context,
        frame_data: &[FrameData],
        sheet: &SpriteSheet,
        viewport_width: f32,
        viewport_height: f32,
        pan_x: f32,
        pan_y: f32,
        zoom: f32,
    ) -> Result<(), CanvasError> {
        unsafe {
            self.upload_texture(gl_context, sheet)?;

            if self.texture.is_none() {
                trace!("No texture bound; skipping draw_frame call.");
                return Ok(());
            }

            let projection = [
                2.0 / viewport_width, 0.0, 0.0,
                0.0, -2.0 / viewport_height, 0.0,
                -1.0, 1.0, 1.0,
            ];

            let camera = [
                zoom, 0.0, 0.0,
                0.0, zoom, 0.0,
                (viewport_width / 2.0) + pan_x * zoom, (viewport_height / 2.0) + pan_y * zoom, 1.0
            ];

            let view_proj_matrix = multiply_mat3(&projection, &camera);

            gl_context.disable(glow::DEPTH_TEST);
            gl_context.depth_mask(false);
            gl_context.disable(glow::CULL_FACE);

            gl_context.use_program(Some(self.program));
            gl_context.bind_vertex_array(Some(self.vertex_array));
            gl_context.active_texture(glow::TEXTURE0);
            gl_context.bind_texture(glow::TEXTURE_2D, self.texture);

            let u_transform = gl_context.get_uniform_location(self.program, "u_transform");
            let u_view_proj = gl_context.get_uniform_location(self.program, "u_view_proj");
            let u_opacity = gl_context.get_uniform_location(self.program, "u_opacity");
            let u_texture = gl_context.get_uniform_location(self.program, "u_texture");

            gl_context.uniform_1_i32(u_texture.as_ref(), 0);
            gl_context.uniform_matrix_3_f32_slice(u_view_proj.as_ref(), false, &view_proj_matrix);
            gl_context.enable(glow::BLEND);

            for data in frame_data {
                match data.glow {
                    1 => gl_context.blend_func_separate(glow::ONE, glow::ONE, glow::ZERO, glow::ONE),
                    2 => gl_context.blend_func_separate(glow::DST_COLOR, glow::ZERO, glow::ZERO, glow::ONE),
                    3 => gl_context.blend_func_separate(glow::ONE, glow::ONE_MINUS_SRC_COLOR, glow::ZERO, glow::ONE),
                    _ => gl_context.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA),
                }

                gl_context.uniform_matrix_3_f32_slice(u_transform.as_ref(), false, &data.final_matrix);
                gl_context.uniform_1_f32(u_opacity.as_ref(), data.opacity);

                gl_context.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
                gl_context.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(&data.vertices), glow::DYNAMIC_DRAW);

                gl_context.bind_buffer(glow::ARRAY_BUFFER, Some(self.tbo));
                gl_context.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(&data.uvs), glow::DYNAMIC_DRAW);

                gl_context.draw_arrays(glow::TRIANGLES, 0, 6);
            }

            gl_context.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);

            Ok(())
        }
    }
}

#[inline(always)]
pub fn multiply_mat3(matrix_a: &[f32; 9], matrix_b: &[f32; 9]) -> [f32; 9] {
    [
        matrix_a[0]*matrix_b[0] + matrix_a[3]*matrix_b[1] + matrix_a[6]*matrix_b[2],
        matrix_a[1]*matrix_b[0] + matrix_a[4]*matrix_b[1] + matrix_a[7]*matrix_b[2],
        matrix_a[2]*matrix_b[0] + matrix_a[5]*matrix_b[1] + matrix_a[8]*matrix_b[2],

        matrix_a[0]*matrix_b[3] + matrix_a[3]*matrix_b[4] + matrix_a[6]*matrix_b[5],
        matrix_a[1]*matrix_b[3] + matrix_a[4]*matrix_b[4] + matrix_a[7]*matrix_b[5],
        matrix_a[2]*matrix_b[3] + matrix_a[5]*matrix_b[4] + matrix_a[8]*matrix_b[5],

        matrix_a[0]*matrix_b[6] + matrix_a[3]*matrix_b[7] + matrix_a[6]*matrix_b[8],
        matrix_a[1]*matrix_b[6] + matrix_a[4]*matrix_b[7] + matrix_a[7]*matrix_b[8],
        matrix_a[2]*matrix_b[6] + matrix_a[5]*matrix_b[7] + matrix_a[8]*matrix_b[8],
    ]
}

unsafe fn compile_program(gl_context: &glow::Context, vertex_shader_source: &str, fragment_shader_source: &str) -> Result<glow::Program, CanvasError> {
    unsafe {
        let program = gl_context.create_program().map_err(|_| {
            error!("Failed to create OpenGL program");
            CanvasError::ProgramCreation
        })?;

        let shader_version = if cfg!(target_arch = "wasm32") {
            "#version 300 es\n"
        } else {
            "#version 330\n"
        };

        let compiled_vertex_source = format!("{}{}", shader_version, vertex_shader_source);
        let compiled_fragment_source = format!("{}{}", shader_version, fragment_shader_source);

        let vertex_shader = gl_context.create_shader(glow::VERTEX_SHADER).map_err(|_| {
            error!("Failed to create Vertex Shader object");
            CanvasError::VertexShaderCreation
        })?;

        gl_context.shader_source(vertex_shader, &compiled_vertex_source);
        gl_context.compile_shader(vertex_shader);

        if !gl_context.get_shader_compile_status(vertex_shader) {
            let log = gl_context.get_shader_info_log(vertex_shader);
            error!("Vertex shader failed to compile: {}", log);
            return Err(CanvasError::ShaderCompile(log));
        }
        gl_context.attach_shader(program, vertex_shader);

        let fragment_shader = gl_context.create_shader(glow::FRAGMENT_SHADER).map_err(|_| {
            error!("Failed to create Fragment Shader object");
            CanvasError::FragmentShaderCreation
        })?;

        gl_context.shader_source(fragment_shader, &compiled_fragment_source);
        gl_context.compile_shader(fragment_shader);

        if !gl_context.get_shader_compile_status(fragment_shader) {
            let log = gl_context.get_shader_info_log(fragment_shader);
            error!("Fragment shader failed to compile: {}", log);
            return Err(CanvasError::ShaderCompile(log));
        }
        gl_context.attach_shader(program, fragment_shader);

        gl_context.link_program(program);
        if !gl_context.get_program_link_status(program) {
            let log = gl_context.get_program_info_log(program);
            error!("Shader program failed to link: {}", log);
            return Err(CanvasError::ProgramLink(log));
        }

        gl_context.delete_shader(vertex_shader);
        gl_context.delete_shader(fragment_shader);

        Ok(program)
    }
}