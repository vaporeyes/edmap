// ABOUTME: Map viewport — renders grid, linedefs, vertices, and things on a black canvas.
// ABOUTME: Handles pan/zoom, cursor tracking, hover preview, and click-to-select per current mode.

use eframe::egui::{self, Pos2, Stroke};

use super::commands;
use super::hittest;
use super::state::{EditorState, SelectionMode};
use super::textures::TextureBank;
use crate::theme;

/// Screen-pixel pick tolerance — how forgiving the click hit test is.
/// Translated to world units by dividing by the current zoom.
const PICK_TOL_PIXELS: f32 = 8.0;

pub fn draw(ui: &mut egui::Ui, state: &mut EditorState, bank: &mut TextureBank) {
    let available = ui.available_rect_before_wrap();
    let response = ui.allocate_rect(available, egui::Sense::click_and_drag());

    let hover = compute_hover(&response, state);
    // Publish the hover to state so the sidebar can show preview details when
    // nothing is selected. Read on the next frame.
    state.hover_object = match hover {
        Some(Hover::Vertex(i)) | Some(Hover::LineDef(i)) | Some(Hover::Thing(i)) => Some(i),
        None => None,
    };
    handle_input(ui, &response, state, hover);

    let painter = ui.painter_at(available);
    let bg = state.theme_overrides.viewport_bg.unwrap_or(theme::VIEWPORT_BG);
    painter.rect_filled(available, 0.0, bg);

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
        // LineDefs first (so vertex dots draw on top).
        for (i, ld) in map.linedefs.iter().enumerate() {
            let (Some(a), Some(b)) = (
                map.vertices.get(ld.start_vertex as usize),
                map.vertices.get(ld.end_vertex as usize),
            ) else { continue };
            let selected = state.mode == SelectionMode::LineDef && state.selection.contains(&i);
            let highlighted_sector = state.mode == SelectionMode::Sector
                && sidedef_in_selected_sector(map, ld, &state.selection);
            let hovered = matches!(hover, Some(Hover::LineDef(h)) if h == i);

            let color = if selected || hovered {
                theme::LINEDEF_SELECTED
            } else if highlighted_sector {
                theme::VGA_YELLOW
            } else if ld.is_two_sided() {
                theme::LINEDEF_TWO_SIDED
            } else {
                theme::LINEDEF_NORMAL
            };
            let width = if selected || hovered { 2.0 } else { 1.0 };
            let pa = to_screen(egui::pos2(a.x as f32, a.y as f32));
            let pb = to_screen(egui::pos2(b.x as f32, b.y as f32));
            painter.line_segment([pa, pb], Stroke::new(width, color));
        }
        // Vertex dots. In Vertex mode, hovered vertex gets a yellow auto-highlight
        // square (matches original EdMap behavior); selected vertex is bright red.
        for (i, v) in map.vertices.iter().enumerate() {
            let p = to_screen(egui::pos2(v.x as f32, v.y as f32));
            let selected = state.mode == SelectionMode::Vertex && state.selection.contains(&i);
            let hovered = matches!(hover, Some(Hover::Vertex(h)) if h == i);
            let (color, size) = if selected {
                (theme::LINEDEF_SELECTED, 5.0)
            } else if hovered {
                (theme::VERTEX_HOVER, 5.0)
            } else {
                (theme::VERTEX_DOT, 2.0)
            };
            painter.rect_filled(
                egui::Rect::from_center_size(p, egui::vec2(size, size)),
                0.0,
                color,
            );
        }
        // Things (X markers). Filtered by category if state.thing_filter has any
        // unchecked entries; bbox overlay rendered when state.things_bbox_visible.
        for (i, t) in map.things.iter().enumerate() {
            let cat = super::things_table::category_of(t.thing_type);
            if !state.thing_filter[cat.idx()] {
                continue;
            }
            let p = to_screen(egui::pos2(t.x as f32, t.y as f32));
            let selected = state.mode == SelectionMode::Thing && state.selection.contains(&i);
            let hovered = matches!(hover, Some(Hover::Thing(h)) if h == i);
            let color = if selected || hovered { theme::LINEDEF_SELECTED } else { theme::THING_MARK };
            let s = if selected || hovered { 6.0 } else { 4.0 };
            let stroke = Stroke::new(if selected || hovered { 2.0 } else { 1.0 }, color);
            painter.line_segment([p + egui::vec2(-s, -s), p + egui::vec2(s, s)], stroke);
            painter.line_segment([p + egui::vec2(-s, s), p + egui::vec2(s, -s)], stroke);

            if state.things_bbox_visible {
                let r = super::things_table::radius_of(t.thing_type) as f32;
                let r_screen = r * state.view_zoom;
                let bbox = egui::Rect::from_center_size(p, egui::vec2(r_screen * 2.0, r_screen * 2.0));
                painter.rect_stroke(
                    bbox,
                    0.0,
                    Stroke::new(1.0, theme::VGA_DARK_GRAY),
                );
            }
        }
    } else {
        let center = available.center();
        let font = egui::FontId::new(14.0, egui::FontFamily::Monospace);
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            "no map loaded — File (map) > Open map file",
            font,
            theme::VGA_DARK_GRAY,
        );
    }

    // Hover-preview a thing's sprite — only meaningful in Thing mode (where the
    // hover hit test runs against things). Drawn before line-draw so the
    // rubber-band still appears on top if both are active.
    if state.mode == SelectionMode::Thing {
        if let (Some(map), Some(Hover::Thing(idx)), Some(cursor)) =
            (state.map.as_ref(), hover, response.hover_pos())
        {
            if let Some(t) = map.things.get(idx) {
                paint_thing_sprite_popup(ui.ctx(), state, bank, cursor, t.thing_type);
            }
        }
    }

    // Line-draw mode: rubber-band from last placed vertex to cursor.
    if let (Some(map), Some(line_draw)) = (state.map.as_ref(), state.line_draw.as_ref()) {
        if let Some(&last) = line_draw.chain.last() {
            if let Some(v) = map.vertices.get(last as usize) {
                let from = to_screen(egui::pos2(v.x as f32, v.y as f32));
                let to_pos = to_screen(state.cursor_world);
                painter.line_segment([from, to_pos], Stroke::new(1.0, theme::VGA_YELLOW));
            }
        }
        // Highlight the chain start so user knows where to click to close.
        if let Some(&first) = line_draw.chain.first() {
            if let Some(v) = map.vertices.get(first as usize) {
                let p = to_screen(egui::pos2(v.x as f32, v.y as f32));
                painter.circle_stroke(p, 6.0, Stroke::new(1.0, theme::VGA_BRIGHT_GREEN));
            }
        }
    }
}

/// What the cursor is currently hovering over for the active mode.
#[derive(Clone, Copy)]
enum Hover {
    Vertex(usize),
    LineDef(usize),
    Thing(usize),
}

fn compute_hover(response: &egui::Response, state: &EditorState) -> Option<Hover> {
    let _ = response.hover_pos()?;
    let map = state.map.as_ref()?;
    let cursor = (state.cursor_world.x, state.cursor_world.y);
    let tol = (PICK_TOL_PIXELS / state.view_zoom).max(0.5);
    match state.mode {
        SelectionMode::Vertex => hittest::nearest_vertex(map, cursor, tol).map(Hover::Vertex),
        SelectionMode::LineDef => hittest::nearest_linedef(map, cursor, tol).map(Hover::LineDef),
        SelectionMode::Sector => {
            // Hover preview for sector mode highlights the nearest LineDef so user
            // sees which line they're "facing" before the click resolves to a sector.
            hittest::nearest_linedef(map, cursor, tol).map(Hover::LineDef)
        }
        SelectionMode::Thing => hittest::nearest_thing(map, cursor, tol).map(Hover::Thing),
    }
}

fn hover_index_for_mode(mode: SelectionMode, hover: Option<Hover>) -> Option<usize> {
    match (mode, hover) {
        (SelectionMode::Vertex, Some(Hover::Vertex(i))) => Some(i),
        (SelectionMode::LineDef, Some(Hover::LineDef(i))) => Some(i),
        (SelectionMode::Thing, Some(Hover::Thing(i))) => Some(i),
        _ => None,
    }
}

/// True if `ld`'s front or back sidedef belongs to any currently-selected sector.
/// Used to highlight an entire sector's boundary linedefs in sector mode.
fn sidedef_in_selected_sector(
    map: &crate::wad::MapData,
    ld: &crate::wad::LineDef,
    selection: &[usize],
) -> bool {
    use crate::wad::LineDef;
    for sd_idx in [ld.front_sidedef, ld.back_sidedef] {
        if sd_idx == LineDef::NO_SIDEDEF {
            continue;
        }
        let Some(sd) = map.sidedefs.get(sd_idx as usize) else { continue };
        if selection.iter().any(|&s| s == sd.sector as usize) {
            return true;
        }
    }
    false
}

fn handle_input(
    ui: &mut egui::Ui,
    response: &egui::Response,
    state: &mut EditorState,
    hover: Option<Hover>,
) {
    let rect = response.rect;
    if let Some(pos) = response.hover_pos() {
        state.cursor_world = screen_to_world(state, rect, pos);
    }
    // Pan: middle-button or right-button drag.
    if response.dragged_by(egui::PointerButton::Middle)
        || response.dragged_by(egui::PointerButton::Secondary)
    {
        let delta = response.drag_delta();
        state.view_center.x -= delta.x / state.view_zoom;
        // egui Y goes down; world Y goes up — invert.
        state.view_center.y += delta.y / state.view_zoom;
    }
    // Primary-button drag = move selected objects. Sector mode dragging would
    // require translating an entire sector boundary; skipped for now.
    let primary_drag = response.dragged_by(egui::PointerButton::Primary);
    if primary_drag && state.mode != SelectionMode::Sector {
        // Drag start on an unselected object: auto-select it so the drag has
        // something to act on. Avoids the "click first, then drag" two-step.
        if !state.drag_active {
            state.drag_active = true;
            state.drag_residual = egui::Vec2::ZERO;
            if state.selection.is_empty() {
                if let Some(idx) = hover_index_for_mode(state.mode, hover) {
                    state.selection.push(idx);
                }
            }
            // One snapshot per drag (not per mutation frame).
            commands::push_undo(state);
        }
        if !state.selection.is_empty() {
            let scrn_delta = response.drag_delta();
            // Screen → world delta: invert Y because world Y axis points up.
            let world_delta = egui::vec2(
                scrn_delta.x / state.view_zoom,
                -scrn_delta.y / state.view_zoom,
            );
            let (dx, dy) = commands::snap_drag_delta(state, world_delta);
            if dx != 0 || dy != 0 {
                commands::translate_selection(state, dx, dy);
            }
        }
    } else if !primary_drag && state.drag_active {
        state.drag_active = false;
        state.drag_residual = egui::Vec2::ZERO;
    }
    // Zoom: scroll wheel.
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll != 0.0 && response.hovered() {
        let factor = if scroll > 0.0 { 1.1 } else { 1.0 / 1.1 };
        state.view_zoom = (state.view_zoom * factor).clamp(0.01, 16.0);
    }
    // Line-draw mode: right-click places vertex, left-click closes (or also
    // places when not near the start). Bypasses normal selection clicks.
    if state.line_draw.is_some() {
        if response.clicked_by(egui::PointerButton::Secondary) {
            commands::line_draw_place_vertex(state);
            return;
        }
        if response.clicked() {
            if !commands::line_draw_try_close(state) {
                commands::line_draw_place_vertex(state);
            }
            return;
        }
    }
    // Click-to-select. Shift extends the selection; plain click replaces it.
    if response.clicked() {
        let shift = ui.input(|i| i.modifiers.shift);
        apply_click(state, hover, shift);
    }
}

fn apply_click(state: &mut EditorState, hover: Option<Hover>, shift: bool) {
    let Some(map) = state.map.as_ref() else { return };
    let cursor = (state.cursor_world.x, state.cursor_world.y);
    let tol = (PICK_TOL_PIXELS / state.view_zoom).max(0.5);

    let picked: Option<usize> = match state.mode {
        SelectionMode::Vertex => match hover {
            Some(Hover::Vertex(i)) => Some(i),
            _ => hittest::nearest_vertex(map, cursor, tol),
        },
        SelectionMode::LineDef => match hover {
            Some(Hover::LineDef(i)) => Some(i),
            _ => hittest::nearest_linedef(map, cursor, tol),
        },
        SelectionMode::Thing => match hover {
            Some(Hover::Thing(i)) => Some(i),
            _ => hittest::nearest_thing(map, cursor, tol),
        },
        SelectionMode::Sector => hittest::sector_under(map, cursor, tol),
    };

    let Some(idx) = picked else {
        if !shift {
            state.selection.clear();
        }
        return;
    };

    if shift {
        if let Some(pos) = state.selection.iter().position(|&s| s == idx) {
            state.selection.remove(pos); // toggle off
        } else {
            state.selection.push(idx);
        }
    } else {
        state.selection.clear();
        state.selection.push(idx);
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

    // Resolve color: user override beats the intensity setting.
    let color = state
        .theme_overrides
        .grid_dot
        .unwrap_or_else(|| state.grid_intensity.color());
    // Brighter intensities also get a slightly larger dot — easier to spot.
    let dot_size = match state.grid_intensity {
        super::state::GridIntensity::Dim => 1.0,
        super::state::GridIntensity::Normal => 1.5,
        super::state::GridIntensity::Bright => 2.0,
    };
    let dot_dim = egui::vec2(dot_size, dot_size);

    let mut x = start_x;
    while x <= max_x {
        let mut y = start_y;
        while y <= max_y {
            let sx = center.x + (x - state.view_center.x) * state.view_zoom;
            let sy = center.y - (y - state.view_center.y) * state.view_zoom;
            painter.rect_filled(
                egui::Rect::from_center_size(egui::pos2(sx, sy), dot_dim),
                0.0,
                color,
            );
            y += step;
        }
        x += step;
    }
}

/// Hover popup for a Thing — shows the matching DOOM sprite (if available in
/// the loaded WAD) plus the type number. Mirrors the wall/flat preview from
/// the sidebar so the look is consistent.
fn paint_thing_sprite_popup(
    ctx: &egui::Context,
    state: &EditorState,
    bank: &mut TextureBank,
    cursor: egui::Pos2,
    thing_type: u16,
) {
    let Some(wad) = state.wad.as_ref() else { return };
    let candidates = super::things_table::sprite_candidates(thing_type);
    let mut chosen: Option<(&'static str, [usize; 2])> = None;
    for &name in candidates {
        if let Some(handle) = bank.sprite(ctx, wad, name) {
            chosen = Some((name, handle.size()));
            break;
        }
    }
    let Some((name, [w, h])) = chosen else { return };
    if w == 0 || h == 0 {
        return;
    }

    // Re-fetch the handle now that the cache hit/miss is settled.
    let handle = bank.sprite(ctx, wad, name).unwrap().clone();

    // Scale up so small monster sprites are legible — cap longest edge at 160 px.
    let max_edge = 160.0_f32;
    let scale = (max_edge / w as f32).min(max_edge / h as f32).max(1.0);
    let img_size = egui::vec2(w as f32 * scale, h as f32 * scale);
    let pad = 4.0;
    let popup_size = egui::vec2(img_size.x + pad * 2.0, img_size.y + pad * 2.0 + 14.0);
    let mut origin = cursor + egui::vec2(16.0, 16.0);
    let screen = ctx.screen_rect();
    if origin.x + popup_size.x > screen.right() {
        origin.x = cursor.x - 16.0 - popup_size.x;
    }
    if origin.y + popup_size.y > screen.bottom() {
        origin.y = cursor.y - 16.0 - popup_size.y;
    }

    egui::Area::new(egui::Id::new(("thing_preview", thing_type)))
        .order(egui::Order::Tooltip)
        .fixed_pos(origin)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(theme::VIEWPORT_BG)
                .stroke(egui::Stroke::new(1.0, theme::VGA_GRAY))
                .inner_margin(egui::Margin::same(pad))
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(img_size.x, img_size.y + 14.0));
                    let img_rect = egui::Rect::from_min_size(ui.cursor().min, img_size);
                    egui::Image::new(&handle).fit_to_exact_size(img_size).paint_at(ui, img_rect);
                    ui.advance_cursor_after_rect(img_rect);
                    ui.colored_label(
                        theme::VGA_WHITE,
                        format!("type {thing_type}  {name}  {w}x{h}"),
                    );
                });
        });
}
