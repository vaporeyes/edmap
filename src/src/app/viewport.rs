// ABOUTME: Map viewport — renders grid, linedefs, vertices, and things on a black canvas.
// ABOUTME: Handles pan (middle drag), zoom (scroll wheel), and cursor world-space tracking.

use eframe::egui::{self, Pos2, Stroke};

use super::state::EditorState;
use crate::theme;

pub fn draw(ui: &mut egui::Ui, state: &mut EditorState) {
    let available = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(available, egui::Sense::click_and_drag());

    handle_input(ui, &response, state);

    let painter = ui.painter_at(available);
    painter.rect_filled(available, 0.0, theme::VIEWPORT_BG);

    let to_screen = |world: Pos2| world_to_screen(state, available, world);

    if state.grid_visible {
        draw_grid(&painter, available, state);
    }
    if state.origin_visible {
        let origin = to_screen(egui::pos2(0.0, 0.0));
        painter.line_segment(
            [origin - egui::vec2(8.0, 0.0), origin + egui::vec2(8.0, 0.0)],
            Stroke::new(1.0, theme::VGA_BRIGHT_GREEN),
        );
        painter.line_segment(
            [origin - egui::vec2(0.0, 8.0), origin + egui::vec2(0.0, 8.0)],
            Stroke::new(1.0, theme::VGA_BRIGHT_GREEN),
        );
    }

    if let Some(map) = &state.map {
        for ld in &map.linedefs {
            let (Some(a), Some(b)) = (
                map.vertices.get(ld.start_vertex as usize),
                map.vertices.get(ld.end_vertex as usize),
            ) else { continue };
            let color = if ld.is_two_sided() { theme::LINEDEF_TWO_SIDED } else { theme::LINEDEF_NORMAL };
            let pa = to_screen(egui::pos2(a.x as f32, a.y as f32));
            let pb = to_screen(egui::pos2(b.x as f32, b.y as f32));
            painter.line_segment([pa, pb], Stroke::new(1.0, color));
        }
        for v in &map.vertices {
            let p = to_screen(egui::pos2(v.x as f32, v.y as f32));
            painter.rect_filled(egui::Rect::from_center_size(p, egui::vec2(2.0, 2.0)), 0.0, theme::VERTEX_DOT);
        }
        for t in &map.things {
            let p = to_screen(egui::pos2(t.x as f32, t.y as f32));
            // X marker — matches the original's thing glyph.
            let s = 4.0;
            painter.line_segment(
                [p + egui::vec2(-s, -s), p + egui::vec2(s, s)],
                Stroke::new(1.0, theme::THING_MARK),
            );
            painter.line_segment(
                [p + egui::vec2(-s, s), p + egui::vec2(s, -s)],
                Stroke::new(1.0, theme::THING_MARK),
            );
        }
    } else {
        let center = available.center();
        let font = egui::FontId::new(13.0, egui::FontFamily::Monospace);
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            "no map loaded — File (map) > Open map file",
            font,
            theme::VGA_DARK_GRAY,
        );
    }
}

fn handle_input(ui: &mut egui::Ui, response: &egui::Response, state: &mut EditorState) {
    let rect = response.rect;
    if let Some(pos) = response.hover_pos() {
        state.cursor_world = screen_to_world(state, rect, pos);
    }
    // Pan: middle-button or right-button drag.
    if response.dragged_by(egui::PointerButton::Middle) || response.dragged_by(egui::PointerButton::Secondary) {
        let delta = response.drag_delta();
        state.view_center.x -= delta.x / state.view_zoom;
        // egui Y goes down; world Y goes up — invert.
        state.view_center.y += delta.y / state.view_zoom;
    }
    // Zoom: scroll wheel.
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll != 0.0 && response.hovered() {
        let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
        state.view_zoom = (state.view_zoom * factor).clamp(0.01, 16.0);
    }
}

fn world_to_screen(state: &EditorState, rect: egui::Rect, world: Pos2) -> Pos2 {
    let center = rect.center();
    let dx = (world.x - state.view_center.x) * state.view_zoom;
    let dy = -(world.y - state.view_center.y) * state.view_zoom;
    egui::pos2(center.x + dx, center.y + dy)
}

fn screen_to_world(state: &EditorState, rect: egui::Rect, screen: Pos2) -> Pos2 {
    let center = rect.center();
    let dx = (screen.x - center.x) / state.view_zoom;
    let dy = -(screen.y - center.y) / state.view_zoom;
    egui::pos2(state.view_center.x + dx, state.view_center.y + dy)
}

fn draw_grid(painter: &egui::Painter, rect: egui::Rect, state: &EditorState) {
    let g = state.grid_size as f32;
    if g <= 0.0 || state.view_zoom < 0.05 {
        return;
    }
    // Scale the visible grid step so dots don't pile up at low zoom.
    let mut step = g;
    while step * state.view_zoom < 6.0 {
        step *= 2.0;
        if step > 4096.0 {
            return;
        }
    }
    let center = rect.center();
    let half_w = rect.width() * 0.5 / state.view_zoom;
    let half_h = rect.height() * 0.5 / state.view_zoom;
    let min_x = (state.view_center.x - half_w).floor();
    let max_x = (state.view_center.x + half_w).ceil();
    let min_y = (state.view_center.y - half_h).floor();
    let max_y = (state.view_center.y + half_h).ceil();

    let start_x = (min_x / step).floor() * step;
    let start_y = (min_y / step).floor() * step;

    let mut x = start_x;
    while x <= max_x {
        let mut y = start_y;
        while y <= max_y {
            let sx = center.x + (x - state.view_center.x) * state.view_zoom;
            let sy = center.y - (y - state.view_center.y) * state.view_zoom;
            painter.rect_filled(
                egui::Rect::from_center_size(egui::pos2(sx, sy), egui::vec2(1.0, 1.0)),
                0.0,
                theme::GRID_DOT,
            );
            y += step;
        }
        x += step;
    }
}
