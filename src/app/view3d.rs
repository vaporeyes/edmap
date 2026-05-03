// ABOUTME: Phase 1.5 software 3D walk/fly view. Walls + floors/ceilings, flat-shaded by sector light.
// ABOUTME: Per-sector loop walking + ear clipping for fills; near-plane triangle clipping; painter's-algorithm sort.

use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

use super::state::EditorState;
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

pub fn draw(ui: &mut egui::Ui, state: &mut EditorState) {
    let available = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(available, egui::Sense::click_and_drag());

    if state.view3d_cam.is_none() {
        state.view3d_cam = Some(initial_camera(state));
    }
    let dt = ui.ctx().input(|i| i.stable_dt).clamp(0.0, 0.1);
    handle_input(ui, &response, state, dt);

    let painter = ui.painter_at(available);
    painter.rect_filled(available, 0.0, Color32::from_rgb(18, 22, 34));

    let Some(map) = state.map.as_ref() else {
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
    // Sprites are camera-yaw-dependent so they're rebuilt every frame.
    let mut sprites: Vec<Tri3D> = Vec::new();
    build_thing_sprites(map, &state.thing_filter, &cam, &mut sprites);
    let mut projected = project_and_sort(
        &state.view3d_geom_cache.tris,
        &sprites,
        &cam,
        available,
        fov_y,
    );

    // Painter's algorithm: farthest first. While painting we also track the
    // nearest triangle covering a click point — projected is far→near, so the
    // last covering triangle we walk past is the topmost one at that pixel.
    let click_pos = if response.clicked_by(egui::PointerButton::Primary) {
        response.interact_pointer_pos()
    } else {
        None
    };
    let mut topmost_hit: Option<Option<u32>> = None;
    for tri in projected.drain(..) {
        let pts = vec![tri.screen[0], tri.screen[1], tri.screen[2]];
        if let Some(p) = click_pos {
            if point_in_tri_pos(p, tri.screen[0], tri.screen[1], tri.screen[2]) {
                topmost_hit = Some(tri.pick);
            }
        }
        painter.add(egui::Shape::convex_polygon(pts, tri.color, Stroke::NONE));
    }

    if let Some(hit) = topmost_hit {
        handle_pick(state, hit);
    }

    draw_hud(&painter, available);
    // Continuous repaint while in 3D mode so movement is smooth.
    ui.ctx().request_repaint();
}

fn handle_pick(state: &mut EditorState, pick: Option<u32>) {
    use super::state::SelectionMode;
    match pick {
        Some(thing_idx) => {
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
        None => {
            // Hit a wall/floor/ceiling — clear selection silently so the user
            // sees feedback that the click was registered.
            state.selection.clear();
        }
    }
}

fn point_in_tri_pos(p: Pos2, a: Pos2, b: Pos2, c: Pos2) -> bool {
    let d1 = (p.x - a.x) * (b.y - a.y) - (p.y - a.y) * (b.x - a.x);
    let d2 = (p.x - b.x) * (c.y - b.y) - (p.y - b.y) * (c.x - b.x);
    let d3 = (p.x - c.x) * (a.y - c.y) - (p.y - c.y) * (a.x - c.x);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
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

/// Cache of map-derived geometry (walls + floor/ceiling fills). Keyed by a
/// coarse fingerprint of the map data so it auto-rebuilds on edits.
#[derive(Clone, Default)]
pub struct GeometryCache {
    fingerprint: u64,
    tris: Vec<Tri3D>,
}

struct ProjectedTri {
    screen: [Pos2; 3],
    depth: f32,
    color: Color32,
    pick: Option<u32>,
}

fn build_walls(map: &MapData) -> Vec<Tri3D> {
    let mut out: Vec<Tri3D> = Vec::with_capacity(map.linedefs.len() * 4);
    for ld in &map.linedefs {
        let (Some(a), Some(b)) = (
            map.vertices.get(ld.start_vertex as usize),
            map.vertices.get(ld.end_vertex as usize),
        ) else { continue };
        let ax = a.x as f32;
        let ay = a.y as f32;
        let bx = b.x as f32;
        let by = b.y as f32;

        let front_sec = sidedef_sector(map, ld.front_sidedef);
        let back_sec = sidedef_sector(map, ld.back_sidedef);

        match (front_sec, back_sec) {
            (Some(fi), None) => {
                let s = &map.sectors[fi];
                push_quad(
                    &mut out,
                    ax, ay, bx, by,
                    s.floor_height as f32, s.ceiling_height as f32,
                    wall_color(s.light_level, WallKind::Solid),
                );
            }
            (None, Some(bi)) => {
                let s = &map.sectors[bi];
                push_quad(
                    &mut out,
                    bx, by, ax, ay, // reverse orientation since this side is the back
                    s.floor_height as f32, s.ceiling_height as f32,
                    wall_color(s.light_level, WallKind::Solid),
                );
            }
            (Some(fi), Some(bi)) => {
                let f = &map.sectors[fi];
                let b = &map.sectors[bi];
                let lower_top = f.floor_height.max(b.floor_height) as f32;
                let lower_bot = f.floor_height.min(b.floor_height) as f32;
                let upper_top = f.ceiling_height.max(b.ceiling_height) as f32;
                let upper_bot = f.ceiling_height.min(b.ceiling_height) as f32;
                // Lower step is visible from whichever side has the LOWER floor.
                // Wind so the wall normal points into that visible side.
                if lower_top > lower_bot {
                    let face_front = b.floor_height >= f.floor_height;
                    let light = if face_front { f.light_level } else { b.light_level };
                    let (x1, y1, x2, y2) = if face_front { (ax, ay, bx, by) } else { (bx, by, ax, ay) };
                    push_quad(
                        &mut out,
                        x1, y1, x2, y2,
                        lower_bot, lower_top,
                        wall_color(light, WallKind::Lower),
                    );
                }
                // Upper step is visible from whichever side has the HIGHER ceiling.
                if upper_top > upper_bot {
                    let face_front = f.ceiling_height >= b.ceiling_height;
                    let light = if face_front { f.light_level } else { b.light_level };
                    let (x1, y1, x2, y2) = if face_front { (ax, ay, bx, by) } else { (bx, by, ax, ay) };
                    push_quad(
                        &mut out,
                        x1, y1, x2, y2,
                        upper_bot, upper_top,
                        wall_color(light, WallKind::Upper),
                    );
                }
            }
            (None, None) => {}
        }
    }
    out
}

fn push_quad(
    out: &mut Vec<Tri3D>,
    x1: f32, y1: f32, x2: f32, y2: f32,
    z_lo: f32, z_hi: f32,
    color: Color32,
) {
    let p00 = [x1, y1, z_lo];
    let p10 = [x2, y2, z_lo];
    let p11 = [x2, y2, z_hi];
    let p01 = [x1, y1, z_hi];
    out.push(Tri3D { a: p00, b: p10, c: p11, color, pick: None });
    out.push(Tri3D { a: p00, b: p11, c: p01, color, pick: None });
}

#[derive(Clone, Copy)]
enum WallKind {
    Solid,
    Upper,
    Lower,
}

fn wall_color(light: i16, kind: WallKind) -> Color32 {
    let brightness = (light.clamp(0, 255) as f32 / 255.0).max(0.12);
    let (r, g, b) = match kind {
        WallKind::Solid => (180.0, 180.0, 180.0),
        WallKind::Upper => (160.0, 170.0, 200.0),
        WallKind::Lower => (200.0, 170.0, 140.0),
    };
    Color32::from_rgb(
        (r * brightness) as u8,
        (g * brightness) as u8,
        (b * brightness) as u8,
    )
}

fn sidedef_sector(map: &MapData, sd_idx: u16) -> Option<usize> {
    if sd_idx == LineDef::NO_SIDEDEF {
        return None;
    }
    let sd = map.sidedefs.get(sd_idx as usize)?;
    let i = sd.sector as usize;
    if i < map.sectors.len() { Some(i) } else { None }
}

fn project_and_sort(
    static_tris: &[Tri3D],
    dynamic_tris: &[Tri3D],
    cam: &Cam3D,
    screen: egui::Rect,
    fov_y: f32,
) -> Vec<ProjectedTri> {
    let (sin_y, cos_y) = cam.yaw.sin_cos();
    let (sin_p, cos_p) = cam.pitch.sin_cos();
    let half_w = screen.width() * 0.5;
    let half_h = screen.height() * 0.5;
    let focal = half_h / (fov_y * 0.5).tan();
    let cx = screen.center().x;
    let cy = screen.center().y;

    let mut out: Vec<ProjectedTri> = Vec::with_capacity(static_tris.len() + dynamic_tris.len());
    let to_cam = |p: [f32; 3]| -> [f32; 3] {
        // Translate to camera origin
        let dx = p[0] - cam.x;
        let dy = p[1] - cam.y;
        let dz = p[2] - cam.z;
        // Yaw: rotate world around Z so camera-forward aligns with +Y axis
        let rx = dx * cos_y - dy * sin_y;
        let ry = dx * sin_y + dy * cos_y;
        // Pitch: rotate around camera-right (X) so looking up tilts +Y down toward +Z
        let ry2 = ry * cos_p + dz * sin_p;
        let rz2 = -ry * sin_p + dz * cos_p;
        [rx, ry2, rz2]
    };

    let project = |p: [f32; 3]| -> Pos2 {
        let sx = (p[0] / p[1]) * focal + cx;
        let sy = -(p[2] / p[1]) * focal + cy;
        egui::pos2(sx, sy)
    };
    let mut emit = |a: [f32; 3], b: [f32; 3], c: [f32; 3], color: Color32, pick: Option<u32>| {
        let pa = project(a);
        let pb = project(b);
        let pc = project(c);
        let bbox = bounding(&[pa, pb, pc]);
        if !bbox.intersects(screen) {
            return;
        }
        let depth = (a[1] + b[1] + c[1]) / 3.0;
        out.push(ProjectedTri { screen: [pa, pb, pc], depth, color, pick });
    };

    // Cull triangles whose CCW normal points away from the camera. Backfaces
    // cost roughly half the world geometry on every frame; dropping them here
    // is the single biggest speedup before clipping/projection.
    let visible = |a: [f32; 3], b: [f32; 3], c: [f32; 3]| -> bool {
        let nx = (b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1]);
        let ny = (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]);
        let nz = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        // Front-facing if normal points toward camera origin (dot(n, a) < 0).
        nx * a[0] + ny * a[1] + nz * a[2] < 0.0
    };

    for slice in [static_tris, dynamic_tris] {
        for t in slice {
            let ca = to_cam(t.a);
            let cb = to_cam(t.b);
            let cc = to_cam(t.c);
            if !visible(ca, cb, cc) {
                continue;
            }
            clip_near_and_emit(ca, cb, cc, t.color, t.pick, &mut emit);
        }
    }
    let _ = half_w;
    // Far-to-near so painter overdraws correctly.
    out.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal));
    out
}

fn bounding(pts: &[Pos2]) -> egui::Rect {
    let mut min = pts[0];
    let mut max = pts[0];
    for p in pts.iter().skip(1) {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    egui::Rect::from_min_max(min, max)
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
    if cache.fingerprint == fp && !cache.tris.is_empty() {
        return;
    }
    let mut tris = build_walls(map);
    build_floors_ceilings(map, &mut tris);
    cache.fingerprint = fp;
    cache.tris = tris;
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

// ---------------------------------------------------------------------------
// Phase 1.5b: near-plane clipping
// ---------------------------------------------------------------------------

/// Clip a triangle against the near plane (cam-space y = NEAR) and emit the
/// resulting in-front triangles via `emit`. Preserves winding.
fn clip_near_and_emit<F>(
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    color: Color32,
    pick: Option<u32>,
    emit: &mut F,
)
where
    F: FnMut([f32; 3], [f32; 3], [f32; 3], Color32, Option<u32>),
{
    let ya = a[1] >= NEAR;
    let yb = b[1] >= NEAR;
    let yc = c[1] >= NEAR;
    let n_in = (ya as u8) + (yb as u8) + (yc as u8);
    match n_in {
        0 => {}
        3 => emit(a, b, c, color, pick),
        1 => {
            let (p, q, r) = if ya {
                (a, b, c)
            } else if yb {
                (b, c, a)
            } else {
                (c, a, b)
            };
            let pq = lerp_to_near(p, q);
            let rp = lerp_to_near(r, p);
            emit(p, pq, rp, color, pick);
        }
        2 => {
            let (p, q, r) = if !ya {
                (a, b, c)
            } else if !yb {
                (b, c, a)
            } else {
                (c, a, b)
            };
            let pq = lerp_to_near(p, q);
            let rp = lerp_to_near(r, p);
            emit(pq, q, r, color, pick);
            emit(pq, r, rp, color, pick);
        }
        _ => unreachable!(),
    }
}

fn lerp_to_near(behind: [f32; 3], front: [f32; 3]) -> [f32; 3] {
    // Solve for t where behind + t*(front - behind) has y == NEAR.
    let denom = front[1] - behind[1];
    let t = if denom.abs() < 1.0e-6 {
        0.0
    } else {
        ((NEAR - behind[1]) / denom).clamp(0.0, 1.0)
    };
    [
        behind[0] + t * (front[0] - behind[0]),
        NEAR,
        behind[2] + t * (front[2] - behind[2]),
    ]
}

// ---------------------------------------------------------------------------
// Phase 1.5a: floor + ceiling triangulation
// ---------------------------------------------------------------------------

/// For every sector, walk its boundary edges into closed loops, classify
/// outer rings vs holes, splice holes into their containing outer using a
/// rightmost-vertex bridge, then ear-clip and emit floor + ceiling triangles.
fn build_floors_ceilings(map: &MapData, out: &mut Vec<Tri3D>) {
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
        let f_color = floor_color(sector.light_level);
        let c_color = ceiling_color(sector.light_level);

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
            for [i0, i1, i2] in triangles {
                let (a, b, c) = (outer[i0], outer[i1], outer[i2]);
                out.push(Tri3D {
                    a: [a.0, a.1, floor_h],
                    b: [b.0, b.1, floor_h],
                    c: [c.0, c.1, floor_h],
                    color: f_color,
                    pick: None,
                });
                out.push(Tri3D {
                    a: [a.0, a.1, ceil_h],
                    b: [c.0, c.1, ceil_h],
                    c: [b.0, b.1, ceil_h],
                    color: c_color,
                    pick: None,
                });
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

fn floor_color(light: i16) -> Color32 {
    let brightness = (light.clamp(0, 255) as f32 / 255.0).max(0.12);
    Color32::from_rgb(
        (140.0 * brightness) as u8,
        (120.0 * brightness) as u8,
        (95.0 * brightness) as u8,
    )
}

fn ceiling_color(light: i16) -> Color32 {
    let brightness = (light.clamp(0, 255) as f32 / 255.0).max(0.12);
    Color32::from_rgb(
        (95.0 * brightness) as u8,
        (105.0 * brightness) as u8,
        (130.0 * brightness) as u8,
    )
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
// Phase 1.5c: thing sprite billboards
// ---------------------------------------------------------------------------

const THING_HEIGHT: f32 = 56.0; // DOOM-ish stand-up marker height

/// Emit two billboard triangles per thing (filtered by category) into `out`.
/// Billboards face the camera horizontally (yaw-aligned), upright in world Z.
fn build_thing_sprites(
    map: &MapData,
    thing_filter: &[bool; 11],
    cam: &Cam3D,
    out: &mut Vec<Tri3D>,
) {
    use super::things_table;
    // Camera-right vector projected onto the world XY plane (horizontal billboard).
    // World right = perpendicular to camera forward in the XY plane.
    let (sin_y, cos_y) = cam.yaw.sin_cos();
    let right_x = cos_y;
    let right_y = -sin_y;
    for (idx, t) in map.things.iter().enumerate() {
        let cat = things_table::category_of(t.thing_type);
        if !thing_filter[cat.idx()] {
            continue;
        }
        let radius = (things_table::radius_of(t.thing_type) as f32).max(8.0);
        let half_w = radius;
        let height = THING_HEIGHT.max(radius * 1.5);
        let foot_z = sector_floor_at(map, t.x as f32, t.y as f32).unwrap_or(0.0);
        let head_z = foot_z + height;
        let cx = t.x as f32;
        let cy = t.y as f32;
        let lx = cx - right_x * half_w;
        let ly = cy - right_y * half_w;
        let rx = cx + right_x * half_w;
        let ry = cy + right_y * half_w;
        let color = thing_color(cat);
        let pick = Some(idx as u32);
        let bl = [lx, ly, foot_z];
        let br = [rx, ry, foot_z];
        let tr = [rx, ry, head_z];
        let tl = [lx, ly, head_z];
        out.push(Tri3D { a: bl, b: br, c: tr, color, pick });
        out.push(Tri3D { a: bl, b: tr, c: tl, color, pick });
    }
}

fn thing_color(cat: super::things_table::Category) -> Color32 {
    use super::things_table::Category as C;
    match cat {
        C::PlayerStart => Color32::from_rgb(80, 240, 80),
        C::Teleport => Color32::from_rgb(255, 100, 240),
        C::Monster => Color32::from_rgb(240, 80, 80),
        C::Weapon => Color32::from_rgb(220, 200, 90),
        C::Ammo => Color32::from_rgb(180, 140, 60),
        C::Health => Color32::from_rgb(220, 90, 90),
        C::Powerup => Color32::from_rgb(80, 200, 240),
        C::Key => Color32::from_rgb(240, 220, 60),
        C::Obstacle => Color32::from_rgb(150, 150, 150),
        C::Light => Color32::from_rgb(255, 240, 160),
        C::Decoration => Color32::from_rgb(170, 130, 200),
    }
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
