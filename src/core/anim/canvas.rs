use eframe::egui;
use eframe::glow::{self, HasContext};
use std::sync::{Arc, Mutex};
use crate::data::global::imgcut::SpriteSheet;
use super::transform::WorldTransform;

// --- Shaders ---
const VERTEX_SHADER_SOURCE: &str = r#"
    precision lowp float;
    attribute vec2 a_position;
    attribute vec2 a_texcoord;
    uniform mat3 u_transform;
    varying vec2 v_texcoord;

    void main() {
        vec3 pos = u_transform * vec3(a_position, 1.0);
        gl_Position = vec4(pos.xy, 0.0, 1.0);
        v_texcoord = a_texcoord;
    }
"#;

const FRAGMENT_SHADER_SOURCE: &str = r#"
    precision lowp float;
    uniform sampler2D u_texture;
    uniform float u_opacity;
    varying vec2 v_texcoord;

    void main() {
        gl_FragColor = texture2D(u_texture, v_texcoord) * u_opacity;
    }
"#;

// --- Renderer ---
pub struct GlowRenderer {
    program: glow::Program,
    vertex_array: glow::VertexArray,
    vbo: glow::Buffer, 
    tbo: glow::Buffer,
    texture: Option<glow::Texture>,
    last_sheet_name: String,
}

impl GlowRenderer {
    pub fn new(gl: &glow::Context) -> Self {
        unsafe {
            let program = compile_program(gl, VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE);
            let vertex_array = gl.create_vertex_array().expect("Failed to create VAO");
            let vbo = gl.create_buffer().expect("Failed to create VBO");
            let tbo = gl.create_buffer().expect("Failed to create TBO");

            gl.bind_vertex_array(Some(vertex_array));
            
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let pos_loc = gl.get_attrib_location(program, "a_position").unwrap_or(0);
            gl.enable_vertex_attrib_array(pos_loc);
            gl.vertex_attrib_pointer_f32(pos_loc, 2, glow::FLOAT, false, 0, 0);

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(tbo));
            let tex_loc = gl.get_attrib_location(program, "a_texcoord").unwrap_or(1);
            gl.enable_vertex_attrib_array(tex_loc);
            gl.vertex_attrib_pointer_f32(tex_loc, 2, glow::FLOAT, false, 0, 0);

            gl.bind_vertex_array(None);

            Self {
                program,
                vertex_array,
                vbo,
                tbo,
                texture: None,
                last_sheet_name: String::new(),
            }
        }
    }

    fn upload_texture(&mut self, gl: &glow::Context, sheet: &SpriteSheet, allow_update: bool) {
        unsafe {
            // 1. Cache Hit Check
            if self.last_sheet_name == sheet.sheet_name && self.texture.is_some() {
                return;
            }

            // 2. Safety Lock (Prevent Explosion)
            if !allow_update {
                if self.texture.is_some() { return; } // Keep old texture
            }

            // 3. Data Check
            let img = match &sheet.image_data {
                Some(data) => data,
                None => {
                    if self.texture.is_some() { return; } // Keep old texture if loading
                    return; 
                },
            };

            // 4. Texture Recycling (Prevent Leak/Slowdown)
            let tex = if let Some(existing_tex) = self.texture {
                // REUSE: Bind the existing texture ID. 
                // We do NOT delete it. We just overwrite its data below.
                gl.bind_texture(glow::TEXTURE_2D, Some(existing_tex));
                existing_tex
            } else {
                // CREATE: First time only.
                let new_tex = gl.create_texture().expect("Failed to create texture");
                gl.bind_texture(glow::TEXTURE_2D, Some(new_tex));
                
                // Set parameters only on creation (or if needed)
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
                
                new_tex
            };

            // 5. Convert & Upload Pixels
            let pixels = &img.pixels;
            let mut data: Vec<u8> = Vec::with_capacity(pixels.len() * 4);
            
            let gamma: f32 = 1.883;
            let inv_gamma = 1.0 / gamma;
            let to_linear = |c: u8| -> f32 { (c as f32 / 255.0).powf(gamma) };
            let to_monitor = |f: f32| -> u8 { (f.powf(inv_gamma) * 255.0 + 0.5).clamp(0.0, 255.0) as u8 };

            for p in pixels {
                let a_byte = p.a();
                if a_byte == 0 {
                    data.extend_from_slice(&[0, 0, 0, 0]);
                } else {
                    let r_lin = to_linear(p.r());
                    let g_lin = to_linear(p.g());
                    let b_lin = to_linear(p.b());
                    let a_lin = a_byte as f32 / 255.0; 

                    let r_pre = r_lin * a_lin;
                    let g_pre = g_lin * a_lin;
                    let b_pre = b_lin * a_lin;

                    data.push(to_monitor(r_pre));
                    data.push(to_monitor(g_pre));
                    data.push(to_monitor(b_pre));
                    data.push(a_byte);
                }
            }

            gl.tex_image_2d(
                glow::TEXTURE_2D, 0, glow::RGBA as i32,
                img.width() as i32, img.height() as i32, 0,
                glow::RGBA, glow::UNSIGNED_BYTE, Some(&data),
            );

            self.texture = Some(tex);
            self.last_sheet_name = sheet.sheet_name.clone();
        }
    }

    pub fn paint(
        &mut self, 
        gl: &glow::Context, 
        viewport: egui::Rect, 
        parts: &[WorldTransform], 
        sheet: &SpriteSheet, 
        pan: egui::Vec2, 
        zoom: f32,
        allow_update: bool
    ) {
        unsafe {
            self.upload_texture(gl, sheet, allow_update);
            
            if self.texture.is_none() { return; }

            gl.use_program(Some(self.program));
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, self.texture);

            let w = viewport.width();
            let h = viewport.height();
            
            let projection = [
                2.0 / w, 0.0, 0.0,
                0.0, -2.0 / h, 0.0, 
                -1.0, 1.0, 1.0,
            ];

            let center_x = w / 2.0;
            let center_y = h / 2.0;
            
            let camera = [
                zoom, 0.0, 0.0,
                0.0, zoom, 0.0,
                center_x + pan.x * zoom, center_y + pan.y * zoom, 1.0
            ];

            let view_matrix = multiply_mat3(&projection, &camera);

            let u_transform = gl.get_uniform_location(self.program, "u_transform");
            let u_opacity = gl.get_uniform_location(self.program, "u_opacity");
            let u_texture = gl.get_uniform_location(self.program, "u_texture");
            gl.uniform_1_i32(u_texture.as_ref(), 0);

            gl.enable(glow::BLEND);

            for part in parts {
                if part.hidden || part.opacity < 0.005 { continue; }

                if part.glow > 0 {
                    gl.blend_func(glow::ONE, glow::ONE);
                } else {
                    gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
                }

                if let Some(cut) = sheet.cuts_map.get(&part.sprite_index) {
                    let sw = cut.original_size.x;
                    let sh = cut.original_size.y;
                    let px = part.pivot.x;
                    let py = part.pivot.y;

                    let final_matrix = multiply_mat3(&view_matrix, &part.matrix);
                    
                    gl.uniform_matrix_3_f32_slice(u_transform.as_ref(), false, &final_matrix);
                    gl.uniform_1_f32(u_opacity.as_ref(), part.opacity);

                    let vertices: [f32; 12] = [
                        -px,      -py,          
                        sw - px,  -py,          
                        -px,      sh - py,      
                        
                        -px,      sh - py,      
                        sw - px,  -py,          
                        sw - px,  sh - py,      
                    ];
                    
                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(&vertices), glow::DYNAMIC_DRAW);

                    let uv = cut.uv_coordinates;
                    let tex_coords: [f32; 12] = [
                        uv.min.x, uv.min.y, 
                        uv.max.x, uv.min.y, 
                        uv.min.x, uv.max.y, 
                        
                        uv.min.x, uv.max.y, 
                        uv.max.x, uv.min.y, 
                        uv.max.x, uv.max.y, 
                    ];

                    gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.tbo));
                    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytemuck::cast_slice(&tex_coords), glow::DYNAMIC_DRAW);

                    gl.draw_arrays(glow::TRIANGLES, 0, 6);
                }
            }
            
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
        }
    }
}

fn multiply_mat3(a: &[f32; 9], b: &[f32; 9]) -> [f32; 9] {
    [
        a[0]*b[0] + a[3]*b[1] + a[6]*b[2],
        a[1]*b[0] + a[4]*b[1] + a[7]*b[2],
        a[2]*b[0] + a[5]*b[1] + a[8]*b[2],

        a[0]*b[3] + a[3]*b[4] + a[6]*b[5],
        a[1]*b[3] + a[4]*b[4] + a[7]*b[5],
        a[2]*b[3] + a[5]*b[4] + a[8]*b[5],

        a[0]*b[6] + a[3]*b[7] + a[6]*b[8],
        a[1]*b[6] + a[4]*b[7] + a[7]*b[8],
        a[2]*b[6] + a[5]*b[7] + a[8]*b[8],
    ]
}

unsafe fn compile_program(gl: &glow::Context, vs_source: &str, fs_source: &str) -> glow::Program {
    unsafe {
        let program = gl.create_program().expect("Cannot create program");
        
        let vs = gl.create_shader(glow::VERTEX_SHADER).expect("cannot create vertex shader");
        gl.shader_source(vs, vs_source);
        gl.compile_shader(vs);
        if !gl.get_shader_compile_status(vs) {
            panic!("{}", gl.get_shader_info_log(vs));
        }
        gl.attach_shader(program, vs);

        let fs = gl.create_shader(glow::FRAGMENT_SHADER).expect("cannot create fragment shader");
        gl.shader_source(fs, fs_source);
        gl.compile_shader(fs);
        if !gl.get_shader_compile_status(fs) {
            panic!("{}", gl.get_shader_info_log(fs));
        }
        gl.attach_shader(program, fs);

        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("{}", gl.get_program_info_log(program));
        }
        
        gl.delete_shader(vs);
        gl.delete_shader(fs);

        program
    }
}

pub fn paint(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    renderer_ref: Arc<Mutex<Option<GlowRenderer>>>,
    sheet: Arc<SpriteSheet>,
    parts: Vec<WorldTransform>,
    pan: egui::Vec2,
    zoom: f32,
    allow_update: bool
) {
    let callback = egui::PaintCallback {
        rect,
        callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |info, painter| {
            let mut renderer_lock = renderer_ref.lock().unwrap();
            
            if renderer_lock.is_none() {
                *renderer_lock = Some(GlowRenderer::new(painter.gl()));
            }

            if let Some(renderer) = renderer_lock.as_mut() {
                renderer.paint(painter.gl(), info.viewport, &parts, &sheet, pan, zoom, allow_update);
            }
        })),
    };

    ui.painter().add(callback);
}