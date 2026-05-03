// ABOUTME: Phase 2.0a GL-backed 3D walk/fly view. Walls + floors/ceilings + sprites built on CPU,
// ABOUTME: handed to view3d_gl::Renderer3D via egui PaintCallback for proper depth-buffer rendering.

use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32};
use eframe::egui_glow;

use super::state::EditorState;
use super::textures::TextureBank;
use super::view3d_gl::{RenderInput, Renderer3D, TexWrap, WallBatch, WallUpload};
use crate::theme;
use crate::wad::{LineDef, MapData};

const NEAR: f32 = 4.0;
const PITCH_LIMIT: f32 = 1.45; // ~83°, leaves a sliver to avoid gimbal flip
const BASE_MOVE_SPEED: f32 = 320.0; // doom units / second
const BASE_LOOK_SENS: f32 = 0.005;

#[derive(Clone, Debug)]
pub struct Cam3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,   // rotation around +Z
    pub pitch: f32, // rotation around camera-right
}

impl Cam3D {
    fn forward2d(&self) -> (f32, f32) {
        let (s, c) = self.yaw.sin_cos();
        (s, c)
    }
    fn right2d(&self) -> (f32, f32) {
        let (s, c) = self.yaw.sin_cos();
        (c, -s)
    }
}

pub fn draw(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    bank: &TextureBank,
    renderer: Arc<Mutex<Renderer3D>>,
) {
    let available = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(available, egui::Sense::click_and_drag());

    if state.view3d_cam.is_none() {
        state.view3d_cam = Some(initial_camera(state));
    }
    let dt = ui.ctx().input(|i| i.stable_dt).clamp(0.0, 0.1);
    handle_input(ui, &response, state, dt);

    let painter = ui.painter_at(available);

    let Some(map) = state.map.as_ref() else {
        painter.rect_filled(available, 0.0, Color32::from_rgb(18, 22, 34));
        painter.text(
            available.center(),
            egui::Align2::CENTER_CENTER,
            "no map loaded — open one to enter 3D mode",
            egui::FontId::new(14.0, egui::FontFamily::Monospace),
            theme::VGA_DARK_GRAY,
        );
        draw_hud(&painter, available);
        return;
    };

    let cam = state.view3d_cam.as_ref().unwrap().clone();
    let fov_y = state.config.view3d.fov_degrees.clamp(30.0, 130.0).to_radians();
    ensure_geometry_cache(map, &mut state.view3d_geom_cache);

    // Decode any wall/flat textures the renderer hasn't received yet for this map.
    let mut wall_uploads: Vec<WallUpload> = Vec::new();
    if let Some(wad) = state.wad.as_ref() {
        for (kind, name) in &state.view3d_geom_cache.wall_tex_names {
            let id = texture_name_id(*kind, name);
            if state.view3d_geom_cache.uploaded_textures.contains(&id) {
                continue;
            }
            let decoded = match kind {
                TextureKind::Wall => bank.wall_rgba(wad, name),
                TextureKind::Flat => bank.flat_rgba(wad, name),
                TextureKind::Sprite => None, // sprites decoded below
            };
            if let Some((w, h, pixels)) = decoded {
                wall_uploads.push(WallUpload {
                    id,
                    width: w,
                    height: h,
                    pixels,
                    wrap: TexWrap::Repeat,
                });
            }
            state.view3d_geom_cache.uploaded_textures.insert(id);
        }
    }

    // Build per-frame sprite billboards (camera-yaw-dependent) into their own
    // textured batch map, then decode any sprite textures not yet uploaded.
    let mut sprite_by_tex: TexBatchMap = std::collections::HashMap::new();
    let mut sprite_picks: Vec<SpritePick> = Vec::new();
    build_thing_sprite_batches(map, &state.thing_filter, &cam, &mut sprite_by_tex, &mut sprite_picks);
    let (sprite_verts, sprite_batches, sprite_tex_names) = flatten_batches(sprite_by_tex);
    if let Some(wad) = state.wad.as_ref() {
        for (kind, name) in &sprite_tex_names {
            let id = texture_name_id(*kind, name);
            if state.view3d_geom_cache.uploaded_textures.contains(&id) {
                continue;
            }
            if let Some((w, h, pixels)) = bank.sprite_rgba(wad, name) {
                wall_uploads.push(WallUpload {
                    id,
                    width: w,
                    height: h,
                    pixels,
                    wrap: TexWrap::ClampToEdge,
                });
            }
            state.view3d_geom_cache.uploaded_textures.insert(id);
        }
    }

    // Sprite picking: if the user clicked, find the nearest billboard whose
    // projected screen-space bounding box contains the cursor.
    if response.clicked_by(egui::PointerButton::Primary) {
        if let Some(click_pos) = response.interact_pointer_pos() {
            let aspect = (available.width() / available.height().max(1.0)).max(0.01);
            let view_proj = build_view_proj(&cam, fov_y, aspect);
            if let Some(picked_idx) = pick_sprite(&sprite_picks, &view_proj, available, click_pos) {
                handle_sprite_pick(state, picked_idx);
            }
        }
    }

    // Legacy color-shader inputs (currently unused — kept as empty slices).
    let static_verts: Vec<f32> = Vec::new();
    let dynamic_verts: Vec<f32> = Vec::new();

    let aspect = (available.width() / available.height().max(1.0)).max(0.01);
    let view_proj = build_view_proj(&cam, fov_y, aspect);

    let cb_data = CallbackData {
        renderer: renderer.clone(),
        view_proj,
        static_verts,
        static_fp: state.view3d_geom_cache.fingerprint,
        dynamic_verts,
        wall_verts: state.view3d_geom_cache.wall_verts.clone(),
        wall_batches: state.view3d_geom_cache.wall_batches.clone(),
        wall_uploads,
        wall_fp: state.view3d_geom_cache.fingerprint,
        sprite_verts,
        sprite_batches,
    };
    let callback = egui::PaintCallback {
        rect: available,
        callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
            let vp = info.viewport_in_pixels();
            let viewport = (vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
            if let Ok(mut r) = cb_data.renderer.lock() {
                r.render(
                    painter.gl(),
                    RenderInput {
                        viewport,
                        view_proj: cb_data.view_proj,
                        static_verts: &cb_data.static_verts,
                        static_fp: cb_data.static_fp,
                        dynamic_verts: &cb_data.dynamic_verts,
                        wall_verts: &cb_data.wall_verts,
                        wall_batches: &cb_data.wall_batches,
                        wall_uploads: &cb_data.wall_uploads,
                        wall_fp: cb_data.wall_fp,
                        sprite_verts: &cb_data.sprite_verts,
                        sprite_batches: &cb_data.sprite_batches,
                    },
                );
            }
        })),
    };
    painter.add(egui::Shape::Callback(callback));

    // HUD overlay still uses the egui painter, drawn on top of the GL output.
    draw_hud(&painter, available);
    ui.ctx().request_repaint();
}

/// Captured by the PaintCallback closure. All fields owned, no borrows back
/// into state, since the closure runs later in the egui paint pass.
struct CallbackData {
    renderer: Arc<Mutex<Renderer3D>>,
    view_proj: [[f32; 4]; 4],
    static_verts: Vec<f32>,
    static_fp: u64,
    dynamic_verts: Vec<f32>,
    wall_verts: Vec<f32>,
    wall_batches: Vec<WallBatch>,
    wall_uploads: Vec<WallUpload>,
    wall_fp: u64,
    sprite_verts: Vec<f32>,
    sprite_batches: Vec<WallBatch>,
}

/// Pack a Tri3D slice into the (x,y,z,r,g,b,a) float layout the renderer expects.
/// CPU sprite picking: project each billboard's center to screen space, build
/// a screen-space half-extent rect, return the nearest hit.
fn pick_sprite(
    picks: &[SpritePick],
    view_proj: &[[f32; 4]; 4],
    screen: egui::Rect,
    click: egui::Pos2,
) -> Option<u32> {
    let half_w_px = screen.width() * 0.5;
    let half_h_px = screen.height() * 0.5;
    let cx = screen.center().x;
    let cy = screen.center().y;
    let mut best: Option<(f32, u32)> = None;
    for p in picks {
        // Project center to clip space, perspective divide, viewport map.
        let [x, y, z] = p.center;
        let cw = view_proj[0][3] * x
            + view_proj[1][3] * y
            + view_proj[2][3] * z
            + view_proj[3][3];
        if cw <= 0.001 {
            continue; // behind camera
        }
        let cx_clip = view_proj[0][0] * x
            + view_proj[1][0] * y
            + view_proj[2][0] * z
            + view_proj[3][0];
        let cy_clip = view_proj[0][1] * x
            + view_proj[1][1] * y
            + view_proj[2][1] * z
            + view_proj[3][1];
        let cz_clip = view_proj[0][2] * x
            + view_proj[1][2] * y
            + view_proj[2][2] * z
            + view_proj[3][2];
        let ndc_x = cx_clip / cw;
        let ndc_y = cy_clip / cw;
        let ndc_z = cz_clip / cw;
        if !(-1.0..=1.0).contains(&ndc_z) {
            continue;
        }
        let sx = cx + ndc_x * half_w_px;
        let sy = cy - ndc_y * half_h_px; // y inverts (NDC up vs screen down)
        // Scale half-extents from world units to screen pixels using cw as
        // the post-perspective denominator (already incorporates depth).
        let scale = half_h_px / cw;
        let half_w_screen = p.half_w * scale;
        let half_h_screen = p.height * 0.5 * scale;
        let dx = (click.x - sx).abs();
        let dy = (click.y - sy).abs();
        if dx <= half_w_screen && dy <= half_h_screen {
            // Nearer billboards (smaller cw) win on tie.
            if best.map_or(true, |(d, _)| cw < d) {
                best = Some((cw, p.thing_idx));
            }
        }
    }
    best.map(|(_, i)| i)
}

fn handle_sprite_pick(state: &mut EditorState, thing_idx: u32) {
    use super::state::SelectionMode;
    let idx = thing_idx as usize;
    let Some(map) = state.map.as_ref() else { return };
    let Some(t) = map.things.get(idx) else { return };
    let type_no = t.thing_type;
    state.mode = SelectionMode::Thing;
    state.selection = vec![idx];
    state.status_message = Some(format!(
        "3D pick: thing #{idx} (type {type_no}) — Q to exit and edit"
    ));
}

fn pack_tris(tris: &[Tri3D], out: &mut Vec<f32>) {
    for t in tris {
        let r = t.color.r() as f32 / 255.0;
        let g = t.color.g() as f32 / 255.0;
        let b = t.color.b() as f32 / 255.0;
        let a = t.color.a() as f32 / 255.0;
        for v in [t.a, t.b, t.c] {
            out.push(v[0]);
            out.push(v[1]);
            out.push(v[2]);
            out.push(r);
            out.push(g);
            out.push(b);
            out.push(a);
        }
    }
}

/// Build a column-major OpenGL view-projection matrix for the current camera
/// pose and FOV. Maps world (X east, Y north, Z up) into clip space with the
/// usual GL conventions (camera looks down -Z, +Y screen up, depth in [-1, 1]).
fn build_view_proj(cam: &Cam3D, fov_y: f32, aspect: f32) -> [[f32; 4]; 4] {
    let (sy, cy_) = cam.yaw.sin_cos();
    let (sp, cp) = cam.pitch.sin_cos();
    let forward = [sy * cp, cy_ * cp, sp];
    let right = [cy_, -sy, 0.0];
    let up = [
        right[1] * forward[2] - right[2] * forward[1],
        right[2] * forward[0] - right[0] * forward[2],
        right[0] * forward[1] - right[1] * forward[0],
    ];
    let eye = [cam.x, cam.y, cam.z];
    let dot_re = right[0] * eye[0] + right[1] * eye[1] + right[2] * eye[2];
    let dot_ue = up[0] * eye[0] + up[1] * eye[1] + up[2] * eye[2];
    let dot_fe = forward[0] * eye[0] + forward[1] * eye[1] + forward[2] * eye[2];
    let view: [[f32; 4]; 4] = [
        [right[0], up[0], -forward[0], 0.0],
        [right[1], up[1], -forward[1], 0.0],
        [right[2], up[2], -forward[2], 0.0],
        [-dot_re, -dot_ue, dot_fe, 1.0],
    ];
    let near = NEAR;
    let far = 16384.0;
    let fy_inv = 1.0 / (fov_y * 0.5).tan();
    let fx_inv = fy_inv / aspect;
    let a = (far + near) / (near - far);
    let b = (2.0 * far * near) / (near - far);
    let proj: [[f32; 4]; 4] = [
        [fx_inv, 0.0, 0.0, 0.0],
        [0.0, fy_inv, 0.0, 0.0],
        [0.0, 0.0, a, -1.0],
        [0.0, 0.0, b, 0.0],
    ];
    mat4_mul(proj, view)
}

fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut r = [[0.0_f32; 4]; 4];
    for c in 0..4 {
        for row in 0..4 {
            r[c][row] = a[0][row] * b[c][0]
                + a[1][row] * b[c][1]
                + a[2][row] * b[c][2]
                + a[3][row] * b[c][3];
        }
    }
    r
}

fn initial_camera(state: &EditorState) -> Cam3D {
    // Place camera at the player-1 start if we have one, else map centroid + 64 height.
    if let Some(map) = &state.map {
        if let Some(p1) = map.things.iter().find(|t| t.thing_type == 1) {
            // Sit camera at player eye height (DOOM player height ~56, eye ~41).
            let z = sector_floor_at(map, p1.x as f32, p1.y as f32).unwrap_or(0.0) + 41.0;
            return Cam3D {
                x: p1.x as f32,
                y: p1.y as f32,
                z,
                yaw: (p1.angle as f32).to_radians(),
                pitch: 0.0,
            };
        }
        let (cx, cy) = map_centroid(map);
        let z = sector_floor_at(map, cx, cy).unwrap_or(0.0) + 64.0;
        return Cam3D { x: cx, y: cy, z, yaw: 0.0, pitch: 0.0 };
    }
    Cam3D { x: 0.0, y: 0.0, z: 64.0, yaw: 0.0, pitch: 0.0 }
}

fn handle_input(ui: &mut egui::Ui, response: &egui::Response, state: &mut EditorState, dt: f32) {
    let cam = state.view3d_cam.as_mut().unwrap();

    let cfg = state.config.view3d.clone();
    let look_sens = BASE_LOOK_SENS * cfg.mouse_sensitivity;
    let yaw_sign = if cfg.invert_mouse_x { 1.0 } else { -1.0 };
    let pitch_sign = if cfg.invert_mouse_y { 1.0 } else { -1.0 };

    // Drag-to-look: hold left or right button and drag the cursor.
    // (Click-to-capture / true pointer-lock was attempted but hangs on macOS;
    // revisit with proper raw mouse motion in Phase 2's GL path.)
    if response.dragged_by(egui::PointerButton::Primary)
        || response.dragged_by(egui::PointerButton::Secondary)
    {
        let d = response.drag_delta();
        cam.yaw += d.x * look_sens * yaw_sign;
        cam.pitch += d.y * look_sens * pitch_sign;
        cam.pitch = cam.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    ui.ctx().input(|input| {
        let mut speed = BASE_MOVE_SPEED * cfg.move_speed * dt;
        if input.modifiers.shift {
            speed *= cfg.sprint_multiplier;
        }
        let (fx, fy) = cam.forward2d();
        let (rx, ry) = cam.right2d();
        if input.key_down(egui::Key::W) {
            cam.x += fx * speed;
            cam.y += fy * speed;
        }
        if input.key_down(egui::Key::S) {
            cam.x -= fx * speed;
            cam.y -= fy * speed;
        }
        if input.key_down(egui::Key::A) {
            cam.x -= rx * speed;
            cam.y -= ry * speed;
        }
        if input.key_down(egui::Key::D) {
            cam.x += rx * speed;
            cam.y += ry * speed;
        }
        if input.key_down(egui::Key::Space) {
            cam.z += speed;
        }
        // Use E for down so we don't fight Ctrl-modifier shortcuts.
        if input.key_down(egui::Key::E) {
            cam.z -= speed;
        }
    });
}

#[derive(Clone, Copy)]
pub struct Tri3D {
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    color: Color32,
    /// Optional pickable identity — Some(thing_index) for thing billboards,
    /// None for walls/floors/ceilings. The click handler walks near→far and
    /// stops at the first hit; opaque non-pickable triangles correctly occlude.
    pick: Option<u32>,
}

/// Cache of map-derived geometry. Keyed by a coarse fingerprint of the map
/// data so it auto-rebuilds on edits. After Phase 2.0c, all static surfaces
/// (walls + floors + ceilings) flow through the textured wall shader.
#[derive(Clone, Default)]
pub struct GeometryCache {
    fingerprint: u64,
    /// Reserved for any future flat-shaded static fallback (currently unused).
    tris: Vec<Tri3D>,
    /// Packed textured vertex stream: (x, y, z, u_px, v_px, brightness) per vertex.
    pub wall_verts: Vec<f32>,
    /// Per-texture draw groups within `wall_verts`.
    pub wall_batches: Vec<super::view3d_gl::WallBatch>,
    /// Textures referenced by `wall_batches`, parallel-indexed. The kind
    /// dictates which TextureBank decoder to call.
    pub wall_tex_names: Vec<(TextureKind, String)>,
    /// Texture IDs (= hash of (kind, name)) already uploaded to the renderer.
    /// Cleared whenever the cache is invalidated to force re-uploads.
    pub uploaded_textures: std::collections::HashSet<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureKind {
    Wall,
    Flat,
    Sprite,
}

type TexBatchMap = std::collections::HashMap<(TextureKind, String), Vec<f32>>;

/// Build textured wall geometry into the shared (kind, name) batch map.
fn build_wall_batches_into(map: &MapData, by_tex: &mut TexBatchMap) {

    for ld in &map.linedefs {
        let (Some(a), Some(b)) = (
            map.vertices.get(ld.start_vertex as usize),
            map.vertices.get(ld.end_vertex as usize),
        ) else { continue };
        let ax = a.x as f32;
        let ay = a.y as f32;
        let bx = b.x as f32;
        let by = b.y as f32;
        let length = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();

        let front_sd = sidedef_for_idx(map, ld.front_sidedef);
        let back_sd = sidedef_for_idx(map, ld.back_sidedef);
        let front_sec = front_sd.and_then(|sd| map.sectors.get(sd.sector as usize));
        let back_sec = back_sd.and_then(|sd| map.sectors.get(sd.sector as usize));

        let lower_unpegged = (ld.flags & LineDef::FLAG_LOWER_UNPEGGED) != 0;
        let upper_unpegged = (ld.flags & LineDef::FLAG_UPPER_UNPEGGED) != 0;

        match (front_sd, back_sd, front_sec, back_sec) {
            (Some(sd), None, Some(s), None) => {
                let z_lo = s.floor_height as f32;
                let z_hi = s.ceiling_height as f32;
                // Solid wall: default pegs the texture's TOP at the wall top.
                // LOWER_UNPEGGED pegs it at the wall bottom (texture grows up).
                let v_anchor = if lower_unpegged { z_lo } else { z_hi };
                push_textured_quad(
                    by_tex,
                    &sd.middle_texture,
                    ax, ay, bx, by,
                    z_lo, z_hi,
                    sd.x_offset as f32, sd.y_offset as f32, length, s.light_level,
                    v_anchor,
                );
            }
            (None, Some(sd), None, Some(s)) => {
                let z_lo = s.floor_height as f32;
                let z_hi = s.ceiling_height as f32;
                let v_anchor = if lower_unpegged { z_lo } else { z_hi };
                push_textured_quad(
                    by_tex,
                    &sd.middle_texture,
                    bx, by, ax, ay,
                    z_lo, z_hi,
                    sd.x_offset as f32, sd.y_offset as f32, length, s.light_level,
                    v_anchor,
                );
            }
            (Some(fsd), Some(bsd), Some(f), Some(bs)) => {
                let lower_top = f.floor_height.max(bs.floor_height) as f32;
                let lower_bot = f.floor_height.min(bs.floor_height) as f32;
                let upper_top = f.ceiling_height.max(bs.ceiling_height) as f32;
                let upper_bot = f.ceiling_height.min(bs.ceiling_height) as f32;

                if lower_top > lower_bot {
                    let face_front = bs.floor_height >= f.floor_height;
                    let (sd, sec, light) = if face_front {
                        (fsd, f, f.light_level)
                    } else {
                        (bsd, bs, bs.light_level)
                    };
                    let (x1, y1, x2, y2) = if face_front { (ax, ay, bx, by) } else { (bx, by, ax, ay) };
                    // Lower step: default pegs at the step's TOP. LOWER_UNPEGGED
                    // pegs at the front (visible-side) sector's CEILING — this
                    // is the "stairs look continuous" trick.
                    let v_anchor = if lower_unpegged {
                        sec.ceiling_height as f32
                    } else {
                        lower_top
                    };
                    push_textured_quad(
                        by_tex,
                        &sd.lower_texture,
                        x1, y1, x2, y2,
                        lower_bot, lower_top,
                        sd.x_offset as f32, sd.y_offset as f32, length, light,
                        v_anchor,
                    );
                }
                if upper_top > upper_bot {
                    let face_front = f.ceiling_height >= bs.ceiling_height;
                    let (sd, light) = if face_front {
                        (fsd, f.light_level)
                    } else {
                        (bsd, bs.light_level)
                    };
                    let (x1, y1, x2, y2) = if face_front { (ax, ay, bx, by) } else { (bx, by, ax, ay) };
                    // Upper step: default pegs at the BOTTOM of the wall (so
                    // the texture's bottom row aligns with the lower ceiling).
                    // UPPER_UNPEGGED pegs at the wall TOP instead.
                    let v_anchor = if upper_unpegged { upper_top } else { upper_bot };
                    push_textured_quad(
                        by_tex,
                        &sd.upper_texture,
                        x1, y1, x2, y2,
                        upper_bot, upper_top,
                        sd.x_offset as f32, sd.y_offset as f32, length, light,
                        v_anchor,
                    );
                }
                // Two-sided middle textures (railings/grates) deferred — needs
                // alpha sort, not worth the complexity for Phase 2.0b.
            }
            _ => {}
        }
    }
}

/// Flatten a (kind, name) → vertex buffer map into the renderer's wall batch
/// format: a single packed vertex stream + per-batch metadata + parallel name list.
fn flatten_batches(
    by_tex: TexBatchMap,
) -> (Vec<f32>, Vec<super::view3d_gl::WallBatch>, Vec<(TextureKind, String)>) {
    let mut verts: Vec<f32> = Vec::new();
    let mut batches: Vec<super::view3d_gl::WallBatch> = Vec::new();
    let mut names: Vec<(TextureKind, String)> = Vec::new();
    for ((kind, name), vs) in by_tex {
        if vs.is_empty() {
            continue;
        }
        let id = texture_name_id(kind, &name);
        let vertex_offset = (verts.len() / 6) as i32;
        let vertex_count = (vs.len() / 6) as i32;
        verts.extend(vs);
        batches.push(super::view3d_gl::WallBatch { texture_id: id, vertex_offset, vertex_count });
        names.push((kind, name));
    }
    (verts, batches, names)
}

/// Append two CCW triangles for a wall quad with per-vertex pixel UVs and brightness.
/// Skips outright if the texture is empty / "-" (DOOM's "no texture" sentinel).
///
/// `v_anchor` is the world-Z that maps to V = y_off (V increases downward as
/// we descend the wall). Choosing this per surface type implements DOOM's
/// LOWER_UNPEGGED / UPPER_UNPEGGED pegging flags: stairs become continuous
/// when their lower walls all anchor to the shared sector ceiling, etc.
#[allow(clippy::too_many_arguments)]
fn push_textured_quad(
    by_tex: &mut TexBatchMap,
    tex_name: &str,
    x1: f32, y1: f32, x2: f32, y2: f32,
    z_lo: f32, z_hi: f32,
    x_off: f32, y_off: f32,
    length: f32,
    light_level: i16,
    v_anchor: f32,
) {
    if tex_name.is_empty() || tex_name == "-" {
        return;
    }
    let brightness = (light_level.clamp(0, 255) as f32 / 255.0).max(0.12);
    let u0 = x_off;
    let u1 = x_off + length;
    // V at world height z = (v_anchor - z) + y_off. Negative values are fine —
    // GL_REPEAT wraps them transparently in the shader.
    let v_top = (v_anchor - z_hi) + y_off;
    let v_bot = (v_anchor - z_lo) + y_off;

    let entry = by_tex.entry((TextureKind::Wall, tex_name.to_string())).or_default();
    let mut push_v = |x: f32, y: f32, z: f32, u: f32, v: f32| {
        entry.push(x);
        entry.push(y);
        entry.push(z);
        entry.push(u);
        entry.push(v);
        entry.push(brightness);
    };
    // Triangle 1: p00, p10, p11
    push_v(x1, y1, z_lo, u0, v_bot);
    push_v(x2, y2, z_lo, u1, v_bot);
    push_v(x2, y2, z_hi, u1, v_top);
    // Triangle 2: p00, p11, p01
    push_v(x1, y1, z_lo, u0, v_bot);
    push_v(x2, y2, z_hi, u1, v_top);
    push_v(x1, y1, z_hi, u0, v_top);
}

/// Stable in-process hash of a (kind, name) pair for use as the renderer
/// cache key. Kind is part of the hash so a wall texture and a flat with the
/// same name don't collide on the GL side.
pub fn texture_name_id(kind: TextureKind, name: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    match kind {
        TextureKind::Wall => "W:".hash(&mut h),
        TextureKind::Flat => "F:".hash(&mut h),
        TextureKind::Sprite => "S:".hash(&mut h),
    }
    name.to_ascii_uppercase().hash(&mut h);
    h.finish()
}

fn sidedef_for_idx(map: &MapData, sd_idx: u16) -> Option<&crate::wad::SideDef> {
    if sd_idx == LineDef::NO_SIDEDEF {
        return None;
    }
    map.sidedefs.get(sd_idx as usize)
}

fn sidedef_sector(map: &MapData, sd_idx: u16) -> Option<usize> {
    let sd = sidedef_for_idx(map, sd_idx)?;
    let i = sd.sector as usize;
    if i < map.sectors.len() { Some(i) } else { None }
}

fn map_centroid(map: &MapData) -> (f32, f32) {
    if map.vertices.is_empty() {
        return (0.0, 0.0);
    }
    let (mut sx, mut sy) = (0.0_f32, 0.0_f32);
    for v in &map.vertices {
        sx += v.x as f32;
        sy += v.y as f32;
    }
    let n = map.vertices.len() as f32;
    (sx / n, sy / n)
}

/// Best-effort: the floor height of the sector containing (x,y), via nearest linedef.
fn sector_floor_at(map: &MapData, x: f32, y: f32) -> Option<f32> {
    use super::hittest;
    let idx = hittest::sector_under(map, (x, y), 1.0e6)?;
    Some(map.sectors[idx].floor_height as f32)
}

fn draw_hud(painter: &egui::Painter, rect: egui::Rect) {
    let pad = 6.0;
    let bar_h = 18.0;
    let bar = egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.bottom() - bar_h),
        rect.right_bottom(),
    );
    painter.rect_filled(bar, 0.0, Color32::from_black_alpha(180));
    painter.text(
        egui::pos2(bar.left() + pad, bar.center().y),
        egui::Align2::LEFT_CENTER,
        "3D MODE  WASD move  Space/E up/down  Drag to look  Shift sprint  Q exit",
        egui::FontId::new(12.0, egui::FontFamily::Monospace),
        theme::VGA_WHITE,
    );
}

// ---------------------------------------------------------------------------
// Geometry cache (Phase 1.5 polish A)
// ---------------------------------------------------------------------------

/// Rebuild the cached static geometry if the map fingerprint has changed.
fn ensure_geometry_cache(map: &MapData, cache: &mut GeometryCache) {
    let fp = map_fingerprint(map);
    if cache.fingerprint == fp && !cache.wall_verts.is_empty() {
        return;
    }
    let mut by_tex: TexBatchMap = std::collections::HashMap::new();
    build_wall_batches_into(map, &mut by_tex);
    build_floors_ceilings_into(map, &mut by_tex);
    let (wall_verts, wall_batches, wall_tex_names) = flatten_batches(by_tex);
    cache.fingerprint = fp;
    cache.tris.clear();
    cache.wall_verts = wall_verts;
    cache.wall_batches = wall_batches;
    cache.wall_tex_names = wall_tex_names;
    // Renderer dedupes by id, so we re-send all referenced textures and let
    // it skip the ones already cached.
    cache.uploaded_textures.clear();
}

/// Cheap fingerprint that catches every edit we care about (vertex move,
/// linedef rebind, sector height/light change). Not cryptographic; collisions
/// are acceptable since a stale cache only delays rebuild by one user action.
fn map_fingerprint(map: &MapData) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    let mix = |h: u64, v: u64| -> u64 {
        let mut h = h ^ v;
        h = h.wrapping_mul(0x100000001b3);
        h
    };
    h = mix(h, map.vertices.len() as u64);
    h = mix(h, map.linedefs.len() as u64);
    h = mix(h, map.sidedefs.len() as u64);
    h = mix(h, map.sectors.len() as u64);
    for v in &map.vertices {
        h = mix(h, ((v.x as i32 as u32 as u64) << 16) ^ (v.y as i32 as u32 as u64));
    }
    for ld in &map.linedefs {
        let packed = ((ld.start_vertex as u64) << 48)
            | ((ld.end_vertex as u64) << 32)
            | ((ld.front_sidedef as u64) << 16)
            | (ld.back_sidedef as u64);
        h = mix(h, packed);
    }
    for sd in &map.sidedefs {
        h = mix(h, sd.sector as u64);
    }
    for s in &map.sectors {
        let packed = ((s.floor_height as i32 as u32 as u64) << 32)
            | ((s.ceiling_height as i32 as u32 as u64) << 16)
            | (s.light_level as i32 as u32 as u64);
        h = mix(h, packed);
    }
    h
}

// (Phase 1.5b near-plane CPU clipper removed — GL handles it via NEAR plane in projection.)

// ---------------------------------------------------------------------------
// Phase 1.5a: floor + ceiling triangulation
// ---------------------------------------------------------------------------

/// For every sector, walk its boundary edges into closed loops, classify
/// outer rings vs holes, splice holes into their containing outer using a
/// rightmost-vertex bridge, then ear-clip and emit floor + ceiling triangles
/// as textured batches keyed by the sector's FLAT names.
fn build_floors_ceilings_into(map: &MapData, by_tex: &mut TexBatchMap) {
    // DOOM convention: the front sidedef sits on the RIGHT of edge start→end.
    // To walk each sector's boundary CCW (sector on the LEFT of every walking
    // edge, so signed_area > 0 marks outer rings), we reverse the front-edge
    // direction and use back-edges in their natural direction.
    let mut edges_by_sector: Vec<Vec<(u16, u16)>> = vec![Vec::new(); map.sectors.len()];
    for ld in &map.linedefs {
        if let Some(s) = sidedef_sector(map, ld.front_sidedef) {
            edges_by_sector[s].push((ld.end_vertex, ld.start_vertex));
        }
        if let Some(s) = sidedef_sector(map, ld.back_sidedef) {
            edges_by_sector[s].push((ld.start_vertex, ld.end_vertex));
        }
    }

    for (sector_idx, edges) in edges_by_sector.iter().enumerate() {
        if edges.len() < 3 {
            continue;
        }
        let sector = &map.sectors[sector_idx];
        let floor_h = sector.floor_height as f32;
        let ceil_h = sector.ceiling_height as f32;
        let brightness = (sector.light_level.clamp(0, 255) as f32 / 255.0).max(0.12);
        // Skip sectors with no FLAT name on either face (sentinel "-" / empty).
        let floor_tex = sector.floor_texture.clone();
        let ceil_tex = sector.ceiling_texture.clone();
        let want_floor = !floor_tex.is_empty() && floor_tex != "-";
        let want_ceil = !ceil_tex.is_empty() && ceil_tex != "-";
        if !want_floor && !want_ceil {
            continue;
        }

        // Materialize each closed boundary loop as a Vec<(x, y)>, dropping any
        // dangling vertex indices. A loop is "outer" (CCW after walk_loops) or
        // a hole (CW); we partition them so we can splice holes into outers.
        let mut outers: Vec<Vec<(f32, f32)>> = Vec::new();
        let mut holes: Vec<Vec<(f32, f32)>> = Vec::new();
        for verts in walk_loops(edges) {
            let pts: Option<Vec<(f32, f32)>> = verts
                .iter()
                .map(|&vi| {
                    map.vertices
                        .get(vi as usize)
                        .map(|v| (v.x as f32, v.y as f32))
                })
                .collect();
            let Some(pts) = pts else { continue };
            if pts.len() < 3 {
                continue;
            }
            if signed_area(&pts) >= 0.0 {
                outers.push(pts);
            } else {
                holes.push(pts);
            }
        }

        // Assign each hole to the smallest outer that contains it. (Smallest
        // so that nested cases like outer→hole→outer-island still work.)
        let mut holes_for: Vec<Vec<usize>> = vec![Vec::new(); outers.len()];
        for (hi, hole) in holes.iter().enumerate() {
            let probe = hole[0];
            let mut best: Option<(usize, f32)> = None;
            for (oi, outer) in outers.iter().enumerate() {
                if !point_in_polygon(probe, outer) {
                    continue;
                }
                let area = signed_area(outer).abs();
                match best {
                    None => best = Some((oi, area)),
                    Some((_, a)) if area < a => best = Some((oi, area)),
                    _ => {}
                }
            }
            if let Some((oi, _)) = best {
                holes_for[oi].push(hi);
            }
            // Holes with no containing outer are dropped (malformed sector).
        }

        for (oi, mut outer) in outers.into_iter().enumerate() {
            // Splice each assigned hole into the outer polygon using a bridge
            // edge from the hole's rightmost vertex to the closest visible
            // outer vertex. After all holes are absorbed, the result is one
            // simple polygon ready for ear clipping.
            let mut sorted_holes: Vec<&Vec<(f32, f32)>> = holes_for[oi]
                .iter()
                .map(|&i| &holes[i])
                .collect();
            // Process rightmost holes first so earlier bridges stay valid.
            sorted_holes.sort_by(|a, b| {
                let ax = a.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
                let bx = b.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
                bx.partial_cmp(&ax).unwrap_or(std::cmp::Ordering::Equal)
            });
            for hole in sorted_holes {
                splice_hole(&mut outer, hole);
            }

            let triangles = ear_clip(&outer);
            // FLAT textures are world-aligned 64×64 — UV at world (x, y) is
            // just (x, y) in pixel space; the wall shader normalizes by texture
            // size (64) and GL_REPEAT tiles across the floor.
            if want_floor {
                let entry = by_tex
                    .entry((TextureKind::Flat, floor_tex.clone()))
                    .or_default();
                for &[i0, i1, i2] in &triangles {
                    let (a, b, c) = (outer[i0], outer[i1], outer[i2]);
                    // Floor: CCW from above, normal up.
                    push_flat_triangle(entry, a, b, c, floor_h, brightness);
                }
            }
            if want_ceil {
                let entry = by_tex
                    .entry((TextureKind::Flat, ceil_tex.clone()))
                    .or_default();
                for &[i0, i1, i2] in &triangles {
                    let (a, b, c) = (outer[i0], outer[i1], outer[i2]);
                    // Ceiling: reversed winding so normal points down.
                    push_flat_triangle(entry, a, c, b, ceil_h, brightness);
                }
            }
        }
    }
}

/// Splice a hole loop into an outer loop using a bridge edge. Uses the canonical
/// earcut-with-holes algorithm: rightmost hole vertex, ray-cast right to find
/// the first outer edge, then pick a visible outer vertex (the edge endpoint
/// or, if any reflex outer vertices sit inside the candidate triangle, the
/// smallest-angle one). This avoids the bridge crossing other edges, which the
/// previous closest-vertex heuristic produced for non-trivial sector shapes.
fn splice_hole(outer: &mut Vec<(f32, f32)>, hole: &[(f32, f32)]) {
    if hole.len() < 3 || outer.len() < 3 {
        return;
    }
    let h_idx = rightmost_vertex(hole);
    let m = hole[h_idx];
    let Some(o_idx) = find_visible_outer_vertex(outer, m) else {
        return;
    };

    // Build spliced polygon: outer[..=o_idx], hole rotated to start at h_idx
    // (CW orientation preserved so it carves out the hole inside the CCW outer),
    // hole[h_idx] again, outer[o_idx] again, outer[o_idx+1..]. The bridge edge
    // is traversed twice (each direction), keeping the polygon simple.
    let mut spliced: Vec<(f32, f32)> = Vec::with_capacity(outer.len() + hole.len() + 2);
    spliced.extend_from_slice(&outer[..=o_idx]);
    for k in 0..hole.len() {
        spliced.push(hole[(h_idx + k) % hole.len()]);
    }
    spliced.push(m);
    spliced.push(outer[o_idx]);
    spliced.extend_from_slice(&outer[o_idx + 1..]);
    *outer = spliced;
}

fn rightmost_vertex(poly: &[(f32, f32)]) -> usize {
    let mut best = 0;
    let mut best_x = poly[0].0;
    for (i, p) in poly.iter().enumerate().skip(1) {
        if p.0 > best_x {
            best_x = p.0;
            best = i;
        }
    }
    best
}

/// Standard "find a mutually visible outer vertex" routine for earcut bridges.
/// Returns the index of an outer vertex that can be connected to `m` by a
/// straight segment that doesn't cross any other outer edge.
fn find_visible_outer_vertex(outer: &[(f32, f32)], m: (f32, f32)) -> Option<usize> {
    let n = outer.len();

    // 1. Ray-cast from m in the +x direction. Find the closest outer edge
    //    intersection strictly to the right of m on the line y = m.y.
    let mut best_x = f32::INFINITY;
    let mut best_edge: Option<usize> = None;
    for i in 0..n {
        let v1 = outer[i];
        let v2 = outer[(i + 1) % n];
        // Standard half-open crossing test (catches one endpoint, ignores the other).
        let crosses = (v1.1 <= m.1 && v2.1 > m.1) || (v2.1 <= m.1 && v1.1 > m.1);
        if !crosses {
            continue;
        }
        let dy = v2.1 - v1.1;
        if dy.abs() < f32::EPSILON {
            continue;
        }
        let t = (m.1 - v1.1) / dy;
        let ix = v1.0 + t * (v2.0 - v1.0);
        if ix > m.0 && ix < best_x {
            best_x = ix;
            best_edge = Some(i);
        }
    }
    let edge = best_edge?;

    // 2. The edge endpoint with the larger x is the initial candidate (P).
    let v1 = outer[edge];
    let v2 = outer[(edge + 1) % n];
    let (mut p_idx, mut p) = if v1.0 >= v2.0 {
        (edge, v1)
    } else {
        ((edge + 1) % n, v2)
    };

    // 3. If no reflex outer vertex lies inside triangle (m, intersection, p),
    //    p is directly visible. Otherwise find the reflex vertex inside that
    //    triangle whose angle to the m→p direction is smallest (and tie-break
    //    by closest distance), and use it instead.
    let inter = (best_x, m.1);
    let mut best_tan = ((p.1 - m.1).abs() / (p.0 - m.0).max(f32::EPSILON)).abs();
    let mut best_dist2 = sqr_dist(p, m);
    let mut found_reflex = false;

    for (i, &v) in outer.iter().enumerate() {
        if i == p_idx {
            continue;
        }
        if !point_in_tri(v, m, inter, p) {
            continue;
        }
        // Reflex test: with CCW outer winding, a reflex (concave) vertex turns right.
        let prev = outer[(i + n - 1) % n];
        let next = outer[(i + 1) % n];
        let turn = cross2(sub(v, prev), sub(next, v));
        if turn > 0.0 {
            continue; // convex, skip
        }
        let tan = ((v.1 - m.1).abs() / (v.0 - m.0).max(f32::EPSILON)).abs();
        let d2 = sqr_dist(v, m);
        if tan < best_tan || (tan == best_tan && d2 < best_dist2) {
            best_tan = tan;
            best_dist2 = d2;
            p_idx = i;
            p = v;
            found_reflex = true;
        }
    }
    let _ = (p, found_reflex); // names retained for readability above
    Some(p_idx)
}

fn sqr_dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    dx * dx + dy * dy
}

fn point_in_polygon(p: (f32, f32), poly: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        let crosses = (yi > p.1) != (yj > p.1);
        if crosses {
            let dy = yj - yi;
            if dy.abs() > f32::EPSILON {
                let x_cross = (xj - xi) * (p.1 - yi) / dy + xi;
                if p.0 < x_cross {
                    inside = !inside;
                }
            }
        }
        j = i;
    }
    inside
}

/// Push one floor/ceiling triangle into the textured batch buffer.
/// UV at world (x, y) is just (x, y) — DOOM FLATs are 64×64 world-aligned,
/// and the wall shader normalizes by the bound texture's size.
fn push_flat_triangle(
    out: &mut Vec<f32>,
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
    z: f32,
    brightness: f32,
) {
    for (x, y) in [a, b, c] {
        out.push(x);
        out.push(y);
        out.push(z);
        out.push(x); // u_px
        out.push(y); // v_px
        out.push(brightness);
    }
}

/// Greedy chain-walker: pull edges into closed loops by matching tail→head.
/// Returns each loop as an ordered vertex list (no duplicate close vertex).
fn walk_loops(edges: &[(u16, u16)]) -> Vec<Vec<u16>> {
    use std::collections::HashMap;
    let mut by_start: HashMap<u16, Vec<usize>> = HashMap::new();
    for (i, &(s, _)) in edges.iter().enumerate() {
        by_start.entry(s).or_default().push(i);
    }
    let mut used = vec![false; edges.len()];
    let mut loops: Vec<Vec<u16>> = Vec::new();

    for start_edge in 0..edges.len() {
        if used[start_edge] {
            continue;
        }
        let loop_start = edges[start_edge].0;
        let mut current = start_edge;
        let mut path: Vec<u16> = vec![edges[current].0];
        let mut safety = edges.len() + 4;
        loop {
            used[current] = true;
            let (_, end) = edges[current];
            if end == loop_start {
                if path.len() >= 3 {
                    loops.push(path);
                }
                break;
            }
            path.push(end);
            // Pick the next available edge starting at `end`.
            let next_idx = by_start
                .get(&end)
                .and_then(|cands| cands.iter().find(|&&i| !used[i]).copied());
            let Some(n) = next_idx else { break };
            current = n;
            safety -= 1;
            if safety == 0 {
                break;
            }
        }
    }
    loops
}

fn signed_area(pts: &[(f32, f32)]) -> f32 {
    let mut s = 0.0;
    let n = pts.len();
    for i in 0..n {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[(i + 1) % n];
        s += x0 * y1 - x1 * y0;
    }
    s * 0.5
}

/// Ear-clip a CCW simple polygon. Returns triangle vertex indices into `pts`.
fn ear_clip(pts: &[(f32, f32)]) -> Vec<[usize; 3]> {
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }
    if n == 3 {
        return vec![[0, 1, 2]];
    }
    let mut prev: Vec<usize> = (0..n).map(|i| (i + n - 1) % n).collect();
    let mut next: Vec<usize> = (0..n).map(|i| (i + 1) % n).collect();
    let mut active = n;
    let mut tris: Vec<[usize; 3]> = Vec::with_capacity(n - 2);
    let mut i = 0usize;
    let mut safety = n * n + 16;

    while active > 3 && safety > 0 {
        safety -= 1;
        let p = prev[i];
        let q = next[i];
        if is_ear(pts, p, i, q, &next) {
            tris.push([p, i, q]);
            next[p] = q;
            prev[q] = p;
            active -= 1;
            i = p;
        } else {
            i = q;
        }
    }
    if active == 3 {
        let i0 = i;
        let i1 = next[i0];
        let i2 = next[i1];
        tris.push([i0, i1, i2]);
    }
    tris
}

fn is_ear(
    pts: &[(f32, f32)],
    p: usize,
    i: usize,
    q: usize,
    next: &[usize],
) -> bool {
    let a = pts[p];
    let b = pts[i];
    let c = pts[q];
    // Convex on a CCW polygon means the turn at b is a left turn -> cross > 0.
    if cross2(sub(b, a), sub(c, b)) <= 0.0 {
        return false;
    }
    // No other vertex inside this triangle.
    let mut k = next[q];
    while k != p {
        if point_in_tri(pts[k], a, b, c) {
            return false;
        }
        k = next[k];
    }
    true
}

fn sub(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    (a.0 - b.0, a.1 - b.1)
}

fn cross2(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.1 - a.1 * b.0
}

fn point_in_tri(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let d1 = cross2(sub(p, a), sub(b, a));
    let d2 = cross2(sub(p, b), sub(c, b));
    let d3 = cross2(sub(p, c), sub(a, c));
    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    !(has_neg && has_pos)
}

// ---------------------------------------------------------------------------
// Phase 2.0d: textured thing sprite billboards + sprite picking metadata
// ---------------------------------------------------------------------------

const FALLBACK_THING_HEIGHT: f32 = 56.0;

/// Picking record kept alongside each emitted sprite billboard so a click on
/// the 3D viewport can be resolved back to a thing index without re-running
/// the billboard math.
#[derive(Clone, Copy)]
pub struct SpritePick {
    pub thing_idx: u32,
    /// World-space center of the billboard (for distance/depth tie-break).
    pub center: [f32; 3],
    /// Half-width in world units (the right-facing radius).
    pub half_w: f32,
    /// Vertical extent (foot to head) in world units.
    pub height: f32,
}

/// Build textured billboard quads for every unfiltered thing. Sprites are
/// looked up via `things_table::sprite_candidates` and the first available
/// frame name is used. Anchored at the sector floor with the sprite scaled
/// so it sits on the ground at its real radius.
///
/// Returns (`vertex_map_was_extended`, picks). The vertex stream is appended
/// to `by_tex` keyed by `(TextureKind::Sprite, sprite_name)` so identical
/// thing types batch together and we issue one draw call per sprite frame.
fn build_thing_sprite_batches(
    map: &MapData,
    thing_filter: &[bool; 11],
    cam: &Cam3D,
    by_tex: &mut TexBatchMap,
    picks: &mut Vec<SpritePick>,
) {
    use super::things_table;
    let (sin_y, cos_y) = cam.yaw.sin_cos();
    let right_x = cos_y;
    let right_y = -sin_y;

    for (idx, t) in map.things.iter().enumerate() {
        let cat = things_table::category_of(t.thing_type);
        if !thing_filter[cat.idx()] {
            continue;
        }
        let candidates = things_table::sprite_candidates(t.thing_type);
        let Some(&sprite_name) = candidates.first() else {
            continue; // unknown type — skip rather than render a fallback marker
        };

        // Use the radius for billboard half-width (matches the in-game silhouette
        // footprint). Height: monsters/things stand on the floor; we fall back
        // to a fixed pixel height because we don't know real sprite size yet.
        let radius = (things_table::radius_of(t.thing_type) as f32).max(8.0);
        let half_w = radius;
        let height = FALLBACK_THING_HEIGHT.max(radius * 1.5);
        let foot_z = sector_floor_at(map, t.x as f32, t.y as f32).unwrap_or(0.0);
        let head_z = foot_z + height;
        let cx = t.x as f32;
        let cy = t.y as f32;
        let lx = cx - right_x * half_w;
        let ly = cy - right_y * half_w;
        let rx = cx + right_x * half_w;
        let ry = cy + right_y * half_w;
        // Brightness: light from the sector the thing stands in.
        let brightness = sector_at(map, cx, cy)
            .map(|s| (s.light_level.clamp(0, 255) as f32 / 255.0).max(0.12))
            .unwrap_or(1.0);
        // UVs in pixel space; the wall shader normalizes by texture size.
        // (0,0) is top-left; sprite top sits at head_z, bottom at foot_z.
        // We don't yet know the texture's real width/height at this point —
        // pass nominal (1, 1) and rely on the shader's `u_tex_size` to map
        // per-pixel coords to [0,1] correctly. Instead, encode UVs as the
        // sprite-canvas extents we WANT to fill: u_left=0, u_right=tex_w,
        // v_top=0, v_bot=tex_h. We stash a sentinel of -1 for "use full
        // texture" — the shader can't see this so we use 0..1 directly via
        // a conventional 0..(tex_w/tex_w)=1 trick: emit u_left=0, u_right=1
        // and pass that as raw UV that the shader will divide by tex_size.
        // Since we want u in [0..1] to map exactly to one full texture, and
        // the shader does uv/tex_size, we need raw=tex_size. We don't have
        // tex_size yet. Solution: emit raw UV in canonical pixel space
        // assuming the sprite texture spans (0..0) to (W..H) and let the
        // shader divide. We approximate by pre-baking UVs at upload time.
        // ---- pragmatic cut: emit u in [0,1] range as 0 and a "stretch"
        // ---- marker handled by post-processing. Simpler: just use a
        // ---- known sprite size of 1 unit and let GL_REPEAT not kick in
        // ---- (sprites are CLAMP_TO_EDGE so going out of [0,1] clamps).
        // Final approach: emit per-vertex UV in pixel space using the
        // texture's *actual* size known at upload time. Here we pass the
        // half-known UV (0,0)→(W,H) where W,H come from the sprite texture
        // via a side-channel. Since we don't have access to those numbers
        // here, store the UVs as raw 0..1 and rely on the shader doing
        // uv * 1.0 / 1.0... no — that's not right either.
        //
        // SOLVED: use a special UV convention — store the UVs in pixel
        // space matching a NOMINAL "1×1" canvas (u in {0,1}, v in {0,1}).
        // The shader divides by `u_tex_size`, but if we set u_tex_size=1
        // when binding sprite textures, the divide is a no-op and we get
        // the natural sprite-spanning sampling. This requires per-batch
        // binding of u_tex_size = (1, 1) for sprites.
        // (Implemented in the renderer's sprite pass.)
        let entry = by_tex
            .entry((TextureKind::Sprite, sprite_name.to_string()))
            .or_default();
        let mut push = |x: f32, y: f32, z: f32, u: f32, v: f32| {
            entry.push(x);
            entry.push(y);
            entry.push(z);
            entry.push(u);
            entry.push(v);
            entry.push(brightness);
        };
        // Quad: bl (0,1) -> br (1,1) -> tr (1,0) -> tl (0,0)
        // Two CCW triangles when viewed from camera.
        push(lx, ly, foot_z, 0.0, 1.0);
        push(rx, ry, foot_z, 1.0, 1.0);
        push(rx, ry, head_z, 1.0, 0.0);
        push(lx, ly, foot_z, 0.0, 1.0);
        push(rx, ry, head_z, 1.0, 0.0);
        push(lx, ly, head_z, 0.0, 0.0);

        picks.push(SpritePick {
            thing_idx: idx as u32,
            center: [cx, cy, foot_z + height * 0.5],
            half_w,
            height,
        });
    }
}

fn sector_at(map: &MapData, x: f32, y: f32) -> Option<&crate::wad::Sector> {
    let idx = super::hittest::sector_under(map, (x, y), 1.0e6)?;
    map.sectors.get(idx)
}

/// Convenience used by keybindings — toggle 3D mode and reset camera each entry.
pub fn toggle(state: &mut EditorState) {
    state.view3d_open = !state.view3d_open;
    if state.view3d_open {
        state.view3d_cam = None; // re-init from map on first draw
        state.view3d_capture = false;
        state.status_message =
            Some("3D mode — WASD move, Space/E up/down, drag to look, Q to exit".into());
    } else {
        state.view3d_capture = false;
        state.status_message = None;
    }
}
