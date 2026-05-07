// ABOUTME: Phase 2.0a GL-backed renderer for the 3D view. Replaces the painter's-algorithm
// ABOUTME: software pass with glow + a real depth buffer. Flat-shaded for now; textures next.

use std::collections::HashMap;

use eframe::glow::{self, HasContext};

/// Color-shaded vertex: position (3 floats) + RGBA color (4 floats) = 7 f32.
const FLOATS_PER_COLOR_VERTEX: usize = 7;
/// Textured wall vertex: position (3) + UV (2) + brightness (1) = 6 f32.
const FLOATS_PER_WALL_VERTEX: usize = 6;

const COLOR_VS_BODY: &str = r#"
layout(location = 0) in vec3 in_pos;
layout(location = 1) in vec4 in_color;
uniform mat4 u_view_proj;
out vec4 v_color;
void main() {
    v_color = in_color;
    gl_Position = u_view_proj * vec4(in_pos, 1.0);
}
"#;
const COLOR_FS_BODY: &str = r#"
in vec4 v_color;
out vec4 frag;
void main() {
    frag = v_color;
}
"#;

const WALL_VS_BODY: &str = r#"
layout(location = 0) in vec3 in_pos;
layout(location = 1) in vec2 in_uv_px;
layout(location = 2) in float in_brightness;
uniform mat4 u_view_proj;
out vec2 v_uv_px;
out float v_brightness;
void main() {
    v_uv_px = in_uv_px;
    v_brightness = in_brightness;
    gl_Position = u_view_proj * vec4(in_pos, 1.0);
}
"#;
const WALL_FS_BODY: &str = r#"
in vec2 v_uv_px;
in float v_brightness;
uniform sampler2D u_tex;
uniform vec2 u_tex_size;
out vec4 frag;
void main() {
    // UVs come in as pixel offsets, so normalise per-batch with the bound
    // texture's true size before sampling. GL_REPEAT then tiles correctly.
    vec2 uv = v_uv_px / u_tex_size;
    vec4 tx = texture(u_tex, uv);
    if (tx.a < 0.1) discard;
    frag = vec4(tx.rgb * v_brightness, 1.0);
}
"#;

#[cfg(target_arch = "wasm32")]
fn get_vs_header() -> &'static str { "#version 300 es\n" }
#[cfg(not(target_arch = "wasm32"))]
fn get_vs_header() -> &'static str { "#version 330 core\n" }

#[cfg(target_arch = "wasm32")]
fn get_fs_header() -> &'static str { "#version 300 es\nprecision mediump float;\n" }
#[cfg(not(target_arch = "wasm32"))]
fn get_fs_header() -> &'static str { "#version 330 core\n" }

/// One wall draw call within the wall vertex buffer, keyed by texture id.
#[derive(Clone)]
pub struct WallBatch {
    pub texture_id: u64,
    pub vertex_offset: i32,
    pub vertex_count: i32,
}

/// Texture wrap mode requested at upload time. Walls and flats tile via
/// `Repeat`; sprites clamp so the alpha border doesn't bleed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TexWrap {
    Repeat,
    ClampToEdge,
}

/// CPU-decoded RGBA8 texture awaiting GL upload. The renderer takes ownership
/// on the next `render` call and stores the resulting GL texture in its cache,
/// keyed by `id`. Re-uploads with the same id are no-ops.
pub struct WallUpload {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub wrap: TexWrap,
}

/// GL resources for the 3D view. Lazily initialized on the first paint
/// callback (which is the only place a `&glow::Context` is available).
pub struct Renderer3D {
    inner: Option<Inner>,
    /// Fingerprint of the flat-shaded static geometry currently uploaded.
    static_fingerprint: u64,
    /// Number of vertices in the flat-shaded static buffer (3 per triangle).
    static_vertex_count: i32,
    /// Fingerprint of the wall geometry + batches currently uploaded.
    wall_fingerprint: u64,
    /// Per-batch metadata for the wall vertex buffer.
    wall_batches: Vec<WallBatch>,
    /// GL texture cache keyed by `WallUpload.id`. Persists across map loads;
    /// stale entries just sit unused (cheap). Stores the GL handle plus the
    /// (width, height) needed for the wall shader's UV normalization uniform.
    wall_textures: HashMap<u64, (glow::Texture, u32, u32)>,
}

struct Inner {
    color_program: glow::Program,
    u_color_view_proj: glow::UniformLocation,
    wall_program: glow::Program,
    u_wall_view_proj: glow::UniformLocation,
    u_wall_tex: glow::UniformLocation,
    u_wall_tex_size: glow::UniformLocation,
    vao_static: glow::VertexArray,
    vbo_static: glow::Buffer,
    vao_dynamic: glow::VertexArray,
    vbo_dynamic: glow::Buffer,
    vao_walls: glow::VertexArray,
    vbo_walls: glow::Buffer,
    /// Per-frame textured sprite stream — same vertex layout as walls but lives
    /// in its own VBO so wall geometry caching isn't disturbed by sprite churn.
    vao_sprites: glow::VertexArray,
    vbo_sprites: glow::Buffer,
    sprite_capacity: usize,
    /// Capacity of the (currently unused) dynamic color VBO in floats.
    dynamic_capacity: usize,
}

/// Aggregate per-frame inputs to keep the `render` signature manageable.
pub struct RenderInput<'a> {
    pub viewport: (i32, i32, i32, i32),
    pub view_proj: [[f32; 4]; 4],
    /// Flat-shaded floors + ceilings, packed (x,y,z,r,g,b,a) per vertex.
    /// Currently unused (everything textured) but kept for future debug overlays.
    pub static_verts: &'a [f32],
    pub static_fp: u64,
    /// Dynamic flat-shaded geometry. Currently unused.
    pub dynamic_verts: &'a [f32],
    /// Wall + flat textured vertex stream, packed (x,y,z,u,v,brightness).
    pub wall_verts: &'a [f32],
    /// Per-texture wall/flat draw groups within `wall_verts`.
    pub wall_batches: &'a [WallBatch],
    /// New textures to upload (caller provides only on cache invalidation).
    pub wall_uploads: &'a [WallUpload],
    pub wall_fp: u64,
    /// Per-frame textured sprite vertex stream, same layout as `wall_verts`.
    pub sprite_verts: &'a [f32],
    /// Per-texture sprite draw groups within `sprite_verts`.
    pub sprite_batches: &'a [WallBatch],
}

impl Renderer3D {
    pub fn new() -> Self {
        Self {
            inner: None,
            static_fingerprint: 0,
            static_vertex_count: 0,
            wall_fingerprint: 0,
            wall_batches: Vec::new(),
            wall_textures: HashMap::new(),
        }
    }

    /// Render one frame. Lazily initializes GL resources on first call.
    pub fn render(&mut self, gl: &glow::Context, input: RenderInput<'_>) {
        unsafe {
            if self.inner.is_none() {
                self.inner = Some(Inner::new(gl));
                // Force re-uploads on the first frame after init.
                self.static_fingerprint = input.static_fp.wrapping_add(1);
                self.wall_fingerprint = input.wall_fp.wrapping_add(1);
            }
            let inner = self.inner.as_mut().unwrap();

            // ------------------------------------------------------------------
            // Texture uploads (cheap to skip on duplicate ids).
            // ------------------------------------------------------------------
            for up in input.wall_uploads {
                if self.wall_textures.contains_key(&up.id) {
                    continue;
                }
                let tex = upload_rgba_texture(gl, up.width, up.height, &up.pixels, up.wrap);
                self.wall_textures.insert(up.id, (tex, up.width, up.height));
            }

            // ------------------------------------------------------------------
            // Static (flat-shaded) geometry: floors + ceilings.
            // ------------------------------------------------------------------
            if self.static_fingerprint != input.static_fp {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(inner.vbo_static));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck_cast(input.static_verts),
                    glow::STATIC_DRAW,
                );
                self.static_vertex_count =
                    (input.static_verts.len() / FLOATS_PER_COLOR_VERTEX) as i32;
                self.static_fingerprint = input.static_fp;
            }

            // ------------------------------------------------------------------
            // Wall geometry: re-uploaded only when the wall fingerprint changes.
            // ------------------------------------------------------------------
            if self.wall_fingerprint != input.wall_fp {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(inner.vbo_walls));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck_cast(input.wall_verts),
                    glow::STATIC_DRAW,
                );
                self.wall_batches = input.wall_batches.to_vec();
                self.wall_fingerprint = input.wall_fp;
            }

            // ------------------------------------------------------------------
            // Per-frame textured sprite stream (billboards).
            // ------------------------------------------------------------------
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(inner.vbo_sprites));
            if input.sprite_verts.len() > inner.sprite_capacity {
                let new_cap = input.sprite_verts.len().next_power_of_two().max(1024);
                gl.buffer_data_size(
                    glow::ARRAY_BUFFER,
                    (new_cap * std::mem::size_of::<f32>()) as i32,
                    glow::DYNAMIC_DRAW,
                );
                inner.sprite_capacity = new_cap;
            }
            if !input.sprite_verts.is_empty() {
                gl.buffer_sub_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    0,
                    bytemuck_cast(input.sprite_verts),
                );
            }

            // ------------------------------------------------------------------
            // Dynamic flat-shaded color stream (currently unused — kept for
            // future debug overlays / reticles).
            // ------------------------------------------------------------------
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(inner.vbo_dynamic));
            if input.dynamic_verts.len() > inner.dynamic_capacity {
                let new_cap = input.dynamic_verts.len().next_power_of_two().max(1024);
                gl.buffer_data_size(
                    glow::ARRAY_BUFFER,
                    (new_cap * std::mem::size_of::<f32>()) as i32,
                    glow::DYNAMIC_DRAW,
                );
                inner.dynamic_capacity = new_cap;
            }
            if !input.dynamic_verts.is_empty() {
                gl.buffer_sub_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    0,
                    bytemuck_cast(input.dynamic_verts),
                );
            }
            let dyn_vertex_count = (input.dynamic_verts.len() / FLOATS_PER_COLOR_VERTEX) as i32;

            // ------------------------------------------------------------------
            // GL state setup (scissor mirrors viewport so glClear stays local).
            // ------------------------------------------------------------------
            let (vx, vy, vw, vh) = input.viewport;
            gl.viewport(vx, vy, vw, vh);
            gl.scissor(vx, vy, vw, vh);
            gl.enable(glow::SCISSOR_TEST);
            gl.enable(glow::DEPTH_TEST);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
            gl.front_face(glow::CCW);
            gl.depth_func(glow::LESS);
            gl.depth_mask(true);
            gl.clear_color(0.07, 0.085, 0.13, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            let mat: [f32; 16] = flatten_mat4(input.view_proj);

            // ------------------------------------------------------------------
            // Pass 1: textured walls.
            // ------------------------------------------------------------------
            if !self.wall_batches.is_empty() {
                gl.use_program(Some(inner.wall_program));
                gl.uniform_matrix_4_f32_slice(Some(&inner.u_wall_view_proj), false, &mat);
                gl.uniform_1_i32(Some(&inner.u_wall_tex), 0);
                gl.active_texture(glow::TEXTURE0);
                gl.bind_vertex_array(Some(inner.vao_walls));
                
                let mut drawn_batches = 0;
                let mut missing_tex_batches = 0;

                for batch in &self.wall_batches {
                    let Some(&(tex, w, h)) = self.wall_textures.get(&batch.texture_id) else {
                        missing_tex_batches += 1;
                        continue; // texture not yet uploaded — skip this batch silently
                    };
                    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                    gl.uniform_2_f32(Some(&inner.u_wall_tex_size), w as f32, h as f32);
                    gl.draw_arrays(glow::TRIANGLES, batch.vertex_offset, batch.vertex_count);
                    drawn_batches += 1;
                }

                if drawn_batches == 0 && !self.wall_batches.is_empty() {
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::log_1(&format!("3D Error: {} wall batches missing textures!", missing_tex_batches).into());
                }
            }

            // ------------------------------------------------------------------
            // Pass 2: textured sprite billboards. Cull disabled because
            // sprites should be visible from either side of the quad.
            // ------------------------------------------------------------------
            if !input.sprite_batches.is_empty() {
                gl.disable(glow::CULL_FACE);
                gl.bind_vertex_array(Some(inner.vao_sprites));
                // Sprites emit UVs already in [0, 1] (one full sprite per quad);
                // overriding u_tex_size = 1 makes the shader's `uv / u_tex_size`
                // a no-op so the sprite spans the billboard exactly once.
                gl.uniform_2_f32(Some(&inner.u_wall_tex_size), 1.0, 1.0);
                for batch in input.sprite_batches {
                    let Some(&(tex, _w, _h)) = self.wall_textures.get(&batch.texture_id) else {
                        continue;
                    };
                    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
                    gl.draw_arrays(glow::TRIANGLES, batch.vertex_offset, batch.vertex_count);
                }
                gl.enable(glow::CULL_FACE);
            }

            // ------------------------------------------------------------------
            // Pass 3: flat-shaded color geometry (kept for debug overlays).
            // ------------------------------------------------------------------
            if self.static_vertex_count > 0 || dyn_vertex_count > 0 {
                gl.use_program(Some(inner.color_program));
                gl.uniform_matrix_4_f32_slice(Some(&inner.u_color_view_proj), false, &mat);
                if self.static_vertex_count > 0 {
                    gl.bind_vertex_array(Some(inner.vao_static));
                    gl.draw_arrays(glow::TRIANGLES, 0, self.static_vertex_count);
                }
                if dyn_vertex_count > 0 {
                    gl.bind_vertex_array(Some(inner.vao_dynamic));
                    gl.draw_arrays(glow::TRIANGLES, 0, dyn_vertex_count);
                }
            }

            gl.bind_vertex_array(None);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.use_program(None);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::SCISSOR_TEST);
        }
    }
}

impl Inner {
    unsafe fn new(gl: &glow::Context) -> Self {
        let vs_h = get_vs_header();
        let fs_h = get_fs_header();

        let color_program = link_program(
            gl,
            &format!("{}{}", vs_h, COLOR_VS_BODY),
            &format!("{}{}", fs_h, COLOR_FS_BODY),
        );
        let u_color_view_proj = gl
            .get_uniform_location(color_program, "u_view_proj")
            .expect("u_view_proj uniform missing on color shader");
        let wall_program = link_program(
            gl,
            &format!("{}{}", vs_h, WALL_VS_BODY),
            &format!("{}{}", fs_h, WALL_FS_BODY),
        );
        let u_wall_view_proj = gl
            .get_uniform_location(wall_program, "u_view_proj")
            .expect("u_view_proj uniform missing on wall shader");
        let u_wall_tex = gl
            .get_uniform_location(wall_program, "u_tex")
            .expect("u_tex uniform missing on wall shader");
        let u_wall_tex_size = gl
            .get_uniform_location(wall_program, "u_tex_size")
            .expect("u_tex_size uniform missing on wall shader");

        let (vao_static, vbo_static) = make_color_vao(gl);
        let (vao_dynamic, vbo_dynamic) = make_color_vao(gl);
        let (vao_walls, vbo_walls) = make_wall_vao(gl);
        let (vao_sprites, vbo_sprites) = make_wall_vao(gl);

        let initial_cap: usize = 4096;
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo_dynamic));
        gl.buffer_data_size(
            glow::ARRAY_BUFFER,
            (initial_cap * std::mem::size_of::<f32>()) as i32,
            glow::DYNAMIC_DRAW,
        );
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo_sprites));
        gl.buffer_data_size(
            glow::ARRAY_BUFFER,
            (initial_cap * std::mem::size_of::<f32>()) as i32,
            glow::DYNAMIC_DRAW,
        );

        Self {
            color_program,
            u_color_view_proj,
            wall_program,
            u_wall_view_proj,
            u_wall_tex,
            u_wall_tex_size,
            vao_static,
            vbo_static,
            vao_dynamic,
            vbo_dynamic,
            vao_walls,
            vbo_walls,
            vao_sprites,
            vbo_sprites,
            sprite_capacity: initial_cap,
            dynamic_capacity: initial_cap,
        }
    }
}

/// VAO + VBO for the color shader: position (3 floats) + RGBA color (4 floats).
unsafe fn make_color_vao(gl: &glow::Context) -> (glow::VertexArray, glow::Buffer) {
    let vao = gl.create_vertex_array().expect("create_vertex_array");
    let vbo = gl.create_buffer().expect("create_buffer");
    gl.bind_vertex_array(Some(vao));
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
    let stride = (FLOATS_PER_COLOR_VERTEX * std::mem::size_of::<f32>()) as i32;
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
    gl.enable_vertex_attrib_array(1);
    gl.vertex_attrib_pointer_f32(
        1,
        4,
        glow::FLOAT,
        false,
        stride,
        (3 * std::mem::size_of::<f32>()) as i32,
    );
    gl.bind_vertex_array(None);
    (vao, vbo)
}

/// VAO + VBO for the wall shader: position (3) + UV (2) + brightness (1).
unsafe fn make_wall_vao(gl: &glow::Context) -> (glow::VertexArray, glow::Buffer) {
    let vao = gl.create_vertex_array().expect("create_vertex_array");
    let vbo = gl.create_buffer().expect("create_buffer");
    gl.bind_vertex_array(Some(vao));
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
    let stride = (FLOATS_PER_WALL_VERTEX * std::mem::size_of::<f32>()) as i32;
    gl.enable_vertex_attrib_array(0); // pos
    gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
    gl.enable_vertex_attrib_array(1); // uv
    gl.vertex_attrib_pointer_f32(
        1,
        2,
        glow::FLOAT,
        false,
        stride,
        (3 * std::mem::size_of::<f32>()) as i32,
    );
    gl.enable_vertex_attrib_array(2); // brightness
    gl.vertex_attrib_pointer_f32(
        2,
        1,
        glow::FLOAT,
        false,
        stride,
        (5 * std::mem::size_of::<f32>()) as i32,
    );
    gl.bind_vertex_array(None);
    (vao, vbo)
}

/// Upload an RGBA8 image to a fresh GL_TEXTURE_2D with nearest filtering.
/// Wrap mode is per-call so sprites can clamp while walls/flats tile.
unsafe fn upload_rgba_texture(
    gl: &glow::Context,
    w: u32,
    h: u32,
    pixels: &[u8],
    wrap: TexWrap,
) -> glow::Texture {
    let tex = gl.create_texture().expect("create_texture");
    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA as i32,
        w as i32,
        h as i32,
        0,
        glow::RGBA,
        glow::UNSIGNED_BYTE,
        Some(pixels),
    );
    let wrap_gl = match wrap {
        TexWrap::Repeat => glow::REPEAT,
        TexWrap::ClampToEdge => glow::CLAMP_TO_EDGE,
    } as i32;
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, wrap_gl);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, wrap_gl);
    tex
}

unsafe fn link_program(gl: &glow::Context, vs_src: &str, fs_src: &str) -> glow::Program {
    let program = gl.create_program().expect("create_program");
    let stages = [
        (glow::VERTEX_SHADER, vs_src),
        (glow::FRAGMENT_SHADER, fs_src),
    ];
    let mut shaders = Vec::with_capacity(stages.len());
    for (kind, src) in stages {
        let sh = gl.create_shader(kind).expect("create_shader");
        gl.shader_source(sh, src);
        gl.compile_shader(sh);
        if !gl.get_shader_compile_status(sh) {
            let log = gl.get_shader_info_log(sh);
            panic!("3D shader compile failed:\n{log}");
        }
        gl.attach_shader(program, sh);
        shaders.push(sh);
    }
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        let log = gl.get_program_info_log(program);
        panic!("3D shader link failed:\n{log}");
    }
    for sh in shaders {
        gl.detach_shader(program, sh);
        gl.delete_shader(sh);
    }
    program
}

fn flatten_mat4(m: [[f32; 4]; 4]) -> [f32; 16] {
    // Column-major as GL expects (the input is rows of a row-major matrix,
    // but our build_view_proj produces column-major already).
    let mut out = [0.0_f32; 16];
    for c in 0..4 {
        for r in 0..4 {
            out[c * 4 + r] = m[c][r];
        }
    }
    out
}

/// Reinterpret a &[f32] as &[u8] for glow's byte-slice buffer API.
fn bytemuck_cast(src: &[f32]) -> &[u8] {
    let byte_len = std::mem::size_of_val(src);
    // SAFETY: f32 is plain-old-data; reinterpreting as bytes is well-defined.
    unsafe { std::slice::from_raw_parts(src.as_ptr() as *const u8, byte_len) }
}
