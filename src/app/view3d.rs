// ABOUTME: Phase-1 software 3D walk/fly view. Walls only, flat-shaded by sector light.
// ABOUTME: Builds wall quads from the current map, projects per-frame, painter's-algorithm sort.

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
    let tris = build_walls(map);
    let mut projected = project_and_sort(&tris, &cam, available, fov_y);

    // Painter's algorithm: farthest first.
    for tri in projected.drain(..) {
        let pts = vec![tri.screen[0], tri.screen[1], tri.screen[2]];
        painter.add(egui::Shape::convex_polygon(
            pts,
            tri.color,
            Stroke::new(1.0, darken(tri.color, 0.55)),
        ));
    }

    draw_hud(&painter, available);
    // Continuous repaint while in 3D mode so movement is smooth.
    ui.ctx().request_repaint();
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
struct Tri3D {
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    color: Color32,
}

struct ProjectedTri {
    screen: [Pos2; 3],
    depth: f32,
    color: Color32,
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
                if lower_top > lower_bot {
                    let light = if (f.floor_height as f32) > (b.floor_height as f32) { f.light_level } else { b.light_level };
                    push_quad(
                        &mut out,
                        ax, ay, bx, by,
                        lower_bot, lower_top,
                        wall_color(light, WallKind::Lower),
                    );
                }
                if upper_top > upper_bot {
                    let light = if (f.ceiling_height as f32) < (b.ceiling_height as f32) { f.light_level } else { b.light_level };
                    push_quad(
                        &mut out,
                        ax, ay, bx, by,
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
    out.push(Tri3D { a: p00, b: p10, c: p11, color });
    out.push(Tri3D { a: p00, b: p11, c: p01, color });
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

fn darken(c: Color32, factor: f32) -> Color32 {
    Color32::from_rgb(
        (c.r() as f32 * factor) as u8,
        (c.g() as f32 * factor) as u8,
        (c.b() as f32 * factor) as u8,
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
    tris: &[Tri3D],
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

    let mut out: Vec<ProjectedTri> = Vec::with_capacity(tris.len());
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

    for t in tris {
        let ca = to_cam(t.a);
        let cb = to_cam(t.b);
        let cc = to_cam(t.c);
        // Skip if any vertex is behind/near the camera (Phase 1: hard cull, no clip).
        if ca[1] < NEAR || cb[1] < NEAR || cc[1] < NEAR {
            continue;
        }
        let project = |p: [f32; 3]| -> Pos2 {
            let sx = (p[0] / p[1]) * focal + cx;
            let sy = -(p[2] / p[1]) * focal + cy;
            egui::pos2(sx, sy)
        };
        let pa = project(ca);
        let pb = project(cb);
        let pc = project(cc);
        // Cheap off-screen reject
        let bbox = bounding(&[pa, pb, pc]);
        if !bbox.intersects(screen) {
            continue;
        }
        let depth = (ca[1] + cb[1] + cc[1]) / 3.0;
        out.push(ProjectedTri {
            screen: [pa, pb, pc],
            depth,
            color: t.color,
        });
        let _ = half_w; // used below for potential clip; silence unused on pessimistic path
    }
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

/// Convenience used by keybindings — toggle 3D mode and reset camera each entry.
pub fn toggle(state: &mut EditorState) {
    state.view3d_open = !state.view3d_open;
    if state.view3d_open {
        state.view3d_cam = None; // re-init from map on first draw
        state.status_message = Some("3D mode — WASD/Space/E/Shift/drag, Q to exit".into());
    } else {
        state.status_message = None;
    }
}
