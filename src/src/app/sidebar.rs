// ABOUTME: Left sidebar — title, menu list, MAP info box, status fields, mode tabs, compass, LD# table.
// ABOUTME: Layout matches the second screenshot top-to-bottom.

use eframe::egui::{self, Color32, RichText};

use super::menu::MENU_ORDER;
use super::state::{EditorState, SelectionMode};
use super::textures::TextureBank;
use crate::theme;

/// Which family the texture belongs to — controls bank lookup + popup label.
#[derive(Clone, Copy)]
enum TexKind {
    Wall,
    Flat,
}

pub fn draw(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    bank: &mut TextureBank,
    mem_free_kb: u64,
    mem_total_kb: u64,
) {
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

    menu_list(ui, state);
    separator(ui);
    info_box(ui, state, mem_free_kb, mem_total_kb);
    separator(ui);
    status_block(ui, state);
    separator(ui);
    mode_tabs(ui, state);
    separator(ui);
    counter_line(ui, state);
    separator(ui);
    match state.mode {
        SelectionMode::LineDef => linedef_panel(ui, state, bank),
        SelectionMode::Vertex => vertex_panel(ui, state),
        SelectionMode::Sector => sector_panel(ui, state, bank),
        SelectionMode::Thing => thing_panel(ui, state),
    }
    separator(ui);
    compass_rosette(ui, state);

    if let Some(msg) = &state.status_message {
        separator(ui);
        ui.add_space(2.0);
        ui.colored_label(theme::VGA_BRIGHT_RED, msg);
    }
}

fn separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let y = rect.top();
    ui.painter().hline(rect.left()..=rect.right(), y, egui::Stroke::new(1.0, theme::VGA_GRAY));
    ui.add_space(1.0);
}

fn menu_list(ui: &mut egui::Ui, state: &mut EditorState) {
    for &name in MENU_ORDER {
        let is_open = state.open_menu == Some(name);

        let desired = egui::vec2(ui.available_width(), 16.0);
        let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());
        let painter = ui.painter_at(rect);

        let bg = if is_open { theme::MENU_HILITE_BG } else { theme::MENU_BG };
        let fg = if is_open { theme::MENU_HILITE_FG } else { theme::MENU_FG };
        painter.rect_filled(rect, 0.0, bg);

        // Turbo-Vision raised-button bevel: bright top+left, dark bottom+right.
        // Inverted when the row is "pressed" (active/open).
        crate::theme::draw_bevel(&painter, rect, is_open);

        let font = egui::FontId::new(13.0, egui::FontFamily::Proportional);
        painter.text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            name,
            font,
            fg,
        );
        // Filled right-pointing triangle — Turbo-Vision cascade indicator.
        let tri_right = egui::pos2(rect.right() - 5.0, rect.center().y);
        let tri = vec![
            egui::pos2(tri_right.x - 4.0, tri_right.y - 3.0),
            egui::pos2(tri_right.x - 4.0, tri_right.y + 3.0),
            tri_right,
        ];
        painter.add(egui::Shape::convex_polygon(tri, fg, egui::Stroke::NONE));

        if resp.clicked() {
            state.open_menu = if is_open { None } else { Some(name) };
        } else if resp.hovered() && state.open_menu.is_some() && !is_open {
            state.open_menu = Some(name);
        }
    }
}

fn info_box(ui: &mut egui::Ui, state: &EditorState, mem_free_kb: u64, mem_total_kb: u64) {
    let frame = egui::Frame::none()
        .fill(theme::INFO_BOX_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let base = state.map.as_ref().map(|m| m.name.as_str()).unwrap_or("untitled");
        let map_name = if state.is_dirty { format!("{base} *") } else { base.to_string() };
        ui.label(RichText::new(map_name).color(theme::VGA_YELLOW).strong());
        let origin = if state.wad_path.is_some() { "from PWAD" } else { "original map" };
        ui.label(RichText::new(origin).color(theme::VGA_WHITE).size(11.0));

        // Real memory line: "<free system RAM> free" plus the in-memory map
        // size on a second line. Hover-tooltip shows total/used breakdown.
        let free_label = format!("{} free", super::mem_probe::fmt_kb(mem_free_kb));
        let free_resp = ui.label(
            RichText::new(free_label).color(theme::VGA_BRIGHT_GREEN).size(11.0),
        );
        let used_kb = mem_total_kb.saturating_sub(mem_free_kb);
        free_resp.on_hover_text(format!(
            "System RAM\nfree:  {}\nused:  {}\ntotal: {}",
            super::mem_probe::fmt_kb(mem_free_kb),
            super::mem_probe::fmt_kb(used_kb),
            super::mem_probe::fmt_kb(mem_total_kb),
        ));

        if let Some(map) = state.map.as_ref() {
            let bytes = super::mem_probe::map_data_bytes(map);
            let kb = bytes as f64 / 1024.0;
            ui.label(
                RichText::new(format!("{kb:.2}k map"))
                    .color(theme::VGA_BRIGHT_CYAN)
                    .size(11.0),
            )
            .on_hover_text(format!("Map data in memory: {bytes} bytes\n(record sizes only — no headers)"));
        }
        ui.add_space(2.0);
        ui.label(RichText::new("press F1").color(theme::VGA_WHITE).size(11.0));
        ui.label(RichText::new("for help").color(theme::VGA_WHITE).size(11.0));
    });
}

fn status_block(ui: &mut egui::Ui, state: &EditorState) {
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.colored_label(theme::VGA_BRIGHT_GREEN, format!("G:{:>3}", state.grid_size));
            ui.colored_label(theme::VGA_BRIGHT_GREEN, format!("S:{:>2}", state.snap_size));
        });
        ui.colored_label(theme::VGA_BRIGHT_GREEN, format!("Z:{:>5.2}x", state.view_zoom));
        ui.colored_label(theme::VGA_BRIGHT_GREEN, format!("X: {:>5}", state.cursor_world.x as i32));
        ui.colored_label(theme::VGA_BRIGHT_GREEN, format!("Y: {:>5}", state.cursor_world.y as i32));
    });
}

fn mode_tabs(ui: &mut egui::Ui, state: &mut EditorState) {
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(2.0, 2.0));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for mode in [SelectionMode::Vertex, SelectionMode::LineDef, SelectionMode::Sector, SelectionMode::Thing] {
                let active = state.mode == mode;
                let fg = if active { theme::VGA_YELLOW } else { theme::VGA_BRIGHT_CYAN };
                let label = if active {
                    RichText::new(mode.label()).color(fg).underline()
                } else {
                    RichText::new(mode.label()).color(fg)
                };
                if ui.add(egui::Label::new(label).sense(egui::Sense::click())).clicked() {
                    state.mode = mode;
                }
            }
        });
    });
}

fn counter_line(ui: &mut egui::Ui, state: &EditorState) {
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        let total = state.total_for_mode();
        let sel = state.selection.len();
        ui.colored_label(theme::VGA_WHITE, format!("{sel}/{total}"));
    });
}

/// One row in the "Supported LineDefs" list — a linedef that contains the
/// selected vertex. `letter` is its label in the compass + LD# table; A-Z
/// in order of discovery.
#[derive(Debug, Clone, Copy)]
struct SupportedLine {
    letter: char,
    linedef_idx: usize,
    length: f32,
    /// Angle in radians from the selected vertex to the OTHER endpoint of this
    /// linedef. `0` = +X (east), `+pi/2` = +Y (north). Used to position the
    /// letter on the compass dial.
    angle: f32,
}

fn supported_linedefs(map: &crate::wad::MapData, vertex_idx: usize) -> Vec<SupportedLine> {
    let Some(v) = map.vertices.get(vertex_idx) else { return Vec::new() };
    let mut out = Vec::new();
    for (i, ld) in map.linedefs.iter().enumerate() {
        let other = if ld.start_vertex as usize == vertex_idx {
            Some(ld.end_vertex as usize)
        } else if ld.end_vertex as usize == vertex_idx {
            Some(ld.start_vertex as usize)
        } else {
            None
        };
        let Some(other_idx) = other else { continue };
        let Some(other_v) = map.vertices.get(other_idx) else { continue };
        let dx = (other_v.x - v.x) as f32;
        let dy = (other_v.y - v.y) as f32;
        let length = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);
        let letter = (b'A' + (out.len() as u8 % 26)) as char;
        out.push(SupportedLine { letter, linedef_idx: i, length, angle });
        if out.len() >= 26 {
            break;
        }
    }
    out
}

fn compass_rosette(ui: &mut egui::Ui, state: &EditorState) {
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin {
            left: 4.0,
            right: 4.0,
            top: 4.0,
            bottom: 4.0,
        });
    frame.show(ui, |ui| {
        // Use the full sidebar width so the dial centers under the column,
        // not against the left edge. Height kept tight (60 px) so the rosette
        // fits in the sidebar's tail when the panels above are tall.
        let avail_w = ui.available_width().max(80.0);
        let size = egui::vec2(avail_w, 60.0);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let center = rect.center();
        // Pick the dial radius so labels (placed at r + 8) plus a 12 px font
        // glyph stay strictly inside the rect on the limiting axis.
        let max_label_r = (size.y * 0.5 - 6.0).min(size.x * 0.5 - 6.0).max(10.0);
        let label_r = max_label_r;
        let r = (label_r - 8.0).max(8.0);
        let stroke = egui::Stroke::new(1.0, theme::VGA_BRIGHT_CYAN);
        let font = egui::FontId::new(11.0, egui::FontFamily::Monospace);

        // Center circle is always there.
        painter.circle_stroke(center, 6.0, stroke);

        // Vertex mode + a vertex selected: draw a stub line per supported
        // linedef at its angle, with its letter at the tip.
        if state.mode == SelectionMode::Vertex {
            if let (Some(map), Some(&vidx)) = (state.map.as_ref(), state.selection.first()) {
                let lines = supported_linedefs(map, vidx);
                if !lines.is_empty() {
                    for line in &lines {
                        // Note: world Y increases UP, screen Y increases DOWN, so flip sin.
                        let dx = line.angle.cos();
                        let dy = -line.angle.sin();
                        let tip = egui::pos2(center.x + dx * r, center.y + dy * r);
                        let label_pos =
                            egui::pos2(center.x + dx * label_r, center.y + dy * label_r);
                        painter.line_segment([center, tip], stroke);
                        painter.text(
                            label_pos,
                            egui::Align2::CENTER_CENTER,
                            line.letter.to_string(),
                            font.clone(),
                            theme::VGA_BRIGHT_CYAN,
                        );
                    }
                    let _ = Color32::TRANSPARENT;
                    return;
                }
            }
        }

        // Fallback: static cross + cardinal letters.
        painter.line_segment(
            [egui::pos2(center.x - r, center.y), egui::pos2(center.x + r, center.y)],
            stroke,
        );
        painter.line_segment(
            [egui::pos2(center.x, center.y - r), egui::pos2(center.x, center.y + r)],
            stroke,
        );
        // Anchor each cardinal toward the dial's center so glyphs stay inside
        // the rect even when label_r is at the rect's edge.
        for (label, offset, anchor) in [
            ("N", egui::vec2(0.0, -label_r), egui::Align2::CENTER_TOP),
            ("S", egui::vec2(0.0, label_r), egui::Align2::CENTER_BOTTOM),
            ("W", egui::vec2(-label_r, 0.0), egui::Align2::LEFT_CENTER),
            ("E", egui::vec2(label_r, 0.0), egui::Align2::RIGHT_CENTER),
        ] {
            painter.text(
                center + offset,
                anchor,
                label,
                font.clone(),
                theme::VGA_BRIGHT_CYAN,
            );
        }
        let _ = Color32::TRANSPARENT;
    });
}

/// Numbered LineDef flag list. Order matches the EdMap UX spec verbatim.
/// Each entry: (number 1..9, label, bitmask).
const LINEDEF_FLAGS: &[(u8, &str, u16)] = &[
    (1, "block all",     crate::wad::LineDef::FLAG_BLOCK_ALL),
    (2, "block enemy",   crate::wad::LineDef::FLAG_BLOCK_MONSTERS),
    (3, "two-sided",     crate::wad::LineDef::FLAG_TWO_SIDED),
    (4, "upper pegged",  crate::wad::LineDef::FLAG_UPPER_UNPEGGED),
    (5, "lower pegged",  crate::wad::LineDef::FLAG_LOWER_UNPEGGED),
    (6, "secret wall",   crate::wad::LineDef::FLAG_SECRET),
    (7, "block sound",   crate::wad::LineDef::FLAG_BLOCK_SOUND),
    (8, "never  map",    crate::wad::LineDef::FLAG_NEVER_ON_MAP),
    (9, "start on map",  crate::wad::LineDef::FLAG_ALWAYS_ON_MAP),
];

/// Pick the index to display in a mode panel: prefer selection.first(), then
/// the cursor-hovered object, then 0 as a stable fallback.
fn focus_index(state: &EditorState) -> usize {
    state
        .selection
        .first()
        .copied()
        .or(state.hover_object)
        .unwrap_or(0)
}

fn linedef_panel(ui: &mut egui::Ui, state: &EditorState, bank: &mut TextureBank) {
    let Some(map) = &state.map else {
        return placeholder(ui, "(no map)");
    };
    let ld_idx = focus_index(state);
    let Some(ld) = map.linedefs.get(ld_idx) else {
        return placeholder(ui, "(no linedef)");
    };
    // When nothing is selected, surface the index so the user knows the panel
    // is showing a hover preview, not a stable selection.
    let header_color = if state.selection.is_empty() {
        theme::VGA_BRIGHT_CYAN
    } else {
        theme::VGA_WHITE
    };

    // Header line — shows which linedef this panel reflects. Cyan when the
    // value is a hover preview, white when it's the actual selection.
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        ui.colored_label(header_color, format!("LineDef {ld_idx}"));
    });

    // Numbered flag list.
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        for (num, label, mask) in LINEDEF_FLAGS {
            let set = ld.flags & mask != 0;
            let bullet = if set { "\u{2022}" } else { "\u{25CB}" }; // • / ○
            let bullet_color = if set { theme::VGA_YELLOW } else { theme::VGA_BRIGHT_CYAN };
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.colored_label(theme::VGA_WHITE, format!("{num}"));
                ui.colored_label(bullet_color, bullet);
                ui.colored_label(theme::VGA_WHITE, *label);
            });
        }
    });

    separator(ui);

    // Action / length / texture-offset block.
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let action = if ld.special_type == 0 {
            "(no action)".to_string()
        } else {
            format!("action {}", ld.special_type)
        };
        ui.colored_label(theme::VGA_WHITE, action);

        // length — Euclidean distance between endpoint vertices.
        if let (Some(a), Some(b)) = (
            map.vertices.get(ld.start_vertex as usize),
            map.vertices.get(ld.end_vertex as usize),
        ) {
            let dx = (a.x - b.x) as f32;
            let dy = (a.y - b.y) as f32;
            let len = (dx * dx + dy * dy).sqrt();
            ui.colored_label(theme::VGA_WHITE, format!("length {len:.3}"));
        }

        // SD#: x_offset, y_offset (front sidedef).
        if ld.front_sidedef != crate::wad::LineDef::NO_SIDEDEF {
            if let Some(sd) = map.sidedefs.get(ld.front_sidedef as usize) {
                ui.colored_label(
                    theme::VGA_WHITE,
                    format!("{}: {},{}", ld.front_sidedef, sd.x_offset, sd.y_offset),
                );
            }
        }
    });

    separator(ui);

    // Texture name rows. Front sidedef gets U/M/L; back sidedef (when present) gets N/B/R.
    // The EdMap convention groups the upper/main/lower per side; we use distinct letters
    // for the back side so labels stay short and unique.
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        if ld.front_sidedef != crate::wad::LineDef::NO_SIDEDEF {
            if let Some(sd) = map.sidedefs.get(ld.front_sidedef as usize) {
                texture_row(ui, state, bank, "U", &sd.upper_texture, TexKind::Wall);
                texture_row(ui, state, bank, "M", &sd.middle_texture, TexKind::Wall);
                texture_row(ui, state, bank, "L", &sd.lower_texture, TexKind::Wall);
            }
        }
        if ld.is_two_sided() && ld.back_sidedef != crate::wad::LineDef::NO_SIDEDEF {
            if let Some(sd) = map.sidedefs.get(ld.back_sidedef as usize) {
                texture_row(ui, state, bank, "N", &sd.upper_texture, TexKind::Wall);
                texture_row(ui, state, bank, "B", &sd.middle_texture, TexKind::Wall);
                texture_row(ui, state, bank, "R", &sd.lower_texture, TexKind::Wall);
            }
        }
    });
}

fn texture_row(
    ui: &mut egui::Ui,
    state: &EditorState,
    bank: &mut TextureBank,
    prefix: &str,
    name: &str,
    kind: TexKind,
) {
    let display = if name.is_empty() || name == "-" { "-".to_string() } else { name.to_string() };
    let resp = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.colored_label(theme::VGA_BRIGHT_CYAN, format!("{prefix}:"));
            ui.add(
                egui::Label::new(RichText::new(&display).color(theme::VGA_WHITE))
                    .sense(egui::Sense::hover()),
            )
        })
        .inner;

    // Hover popup near the cursor: a small, pixelated render of the actual
    // texture so the user can see what M: MODWALL2 (etc.) looks like without
    // opening the full F10 viewer.
    if resp.hovered() && display != "-" {
        if let Some(pos) = ui.ctx().pointer_hover_pos() {
            paint_texture_popup(ui.ctx(), state, bank, pos, &display, kind);
        }
    }
}

fn paint_texture_popup(
    ctx: &egui::Context,
    state: &EditorState,
    bank: &mut TextureBank,
    cursor: egui::Pos2,
    name: &str,
    kind: TexKind,
) {
    let Some(wad) = state.wad.as_ref() else { return };
    let handle = match kind {
        TexKind::Wall => bank.wall(ctx, wad, name),
        TexKind::Flat => bank.flat(ctx, wad, name),
    };
    let Some(handle) = handle else { return };
    let [w, h] = handle.size();
    if w == 0 || h == 0 {
        return;
    }
    // Scale so the longest edge is at most 128 px on screen — pixel-pure
    // power-of-two scaling gives the cleanest look on bitmap textures.
    let max_edge = 128.0_f32;
    let scale = (max_edge / w as f32).min(max_edge / h as f32).max(1.0);
    let img_size = egui::vec2(w as f32 * scale, h as f32 * scale);
    let pad = 4.0;
    let popup_size = egui::vec2(img_size.x + pad * 2.0, img_size.y + pad * 2.0 + 14.0);
    // Anchor offset from cursor — bottom-right of the popup near the cursor.
    let mut origin = cursor + egui::vec2(16.0, 16.0);
    // Keep on-screen.
    let screen = ctx.screen_rect();
    if origin.x + popup_size.x > screen.right() {
        origin.x = cursor.x - 16.0 - popup_size.x;
    }
    if origin.y + popup_size.y > screen.bottom() {
        origin.y = cursor.y - 16.0 - popup_size.y;
    }

    egui::Area::new(egui::Id::new(("tex_preview", name)))
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
                    let img_rect = egui::Rect::from_min_size(
                        ui.cursor().min,
                        img_size,
                    );
                    egui::Image::new(handle).fit_to_exact_size(img_size).paint_at(ui, img_rect);
                    ui.advance_cursor_after_rect(img_rect);
                    ui.colored_label(
                        theme::VGA_WHITE,
                        format!("{name}  {w}x{h}"),
                    );
                });
        });
}

fn vertex_panel(ui: &mut egui::Ui, state: &EditorState) {
    let Some(map) = &state.map else {
        return placeholder(ui, "(no map)");
    };
    let idx = focus_index(state);
    let Some(v) = map.vertices.get(idx) else {
        return placeholder(ui, "(no vertex)");
    };

    // Coords block.
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        ui.colored_label(theme::VGA_WHITE, format!("Vertex {idx}"));
        ui.colored_label(theme::VGA_WHITE, format!("X: {:>6}", v.x));
        ui.colored_label(theme::VGA_WHITE, format!("Y: {:>6}", v.y));
    });

    // "Supported LineDefs" header + LD# / length table.
    let lines = supported_linedefs(map, idx);
    separator(ui);
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        ui.colored_label(theme::VGA_BRIGHT_CYAN, "Supported");
        ui.colored_label(theme::VGA_BRIGHT_CYAN, "LineDefs:");
        if lines.is_empty() {
            ui.colored_label(theme::VGA_DARK_GRAY, "(none)");
            return;
        }
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.colored_label(theme::VGA_BRIGHT_CYAN, "LD#");
            ui.colored_label(theme::VGA_BRIGHT_CYAN, "length");
        });
        for line in &lines {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.colored_label(theme::VGA_YELLOW, line.letter.to_string());
                ui.colored_label(theme::VGA_WHITE, format!("{:>3}", line.linedef_idx));
                ui.colored_label(theme::VGA_WHITE, format!("{:>7.3}", line.length));
            });
        }
    });
}

fn sector_panel(ui: &mut egui::Ui, state: &EditorState, bank: &mut TextureBank) {
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let Some(map) = &state.map else {
            ui.colored_label(theme::VGA_DARK_GRAY, "(no map)");
            return;
        };
        let idx = focus_index(state);
        let Some(s) = map.sectors.get(idx) else {
            ui.colored_label(theme::VGA_DARK_GRAY, "(no sector)");
            return;
        };
        let header_color = if state.selection.is_empty() {
            theme::VGA_BRIGHT_CYAN
        } else {
            theme::VGA_WHITE
        };
        ui.colored_label(header_color, format!("Sector {idx}"));
        ui.add_space(2.0);

        // EdMap-style numbered/lettered shortcuts. The leading char in each row
        // is also a hotkey (Sector mode + selection only) — see keybindings.rs.
        sector_field_row(ui, "1", "ceiling", &format!("{:>4}", s.ceiling_height));
        sector_texture_row(ui, state, bank, "2", &s.ceiling_texture);
        sector_field_row(ui, "3", "floor",   &format!("{:>4}", s.floor_height));
        sector_texture_row(ui, state, bank, "4", &s.floor_texture);
        sector_field_row(ui, "5", "light",   &format!("{:>4}", s.light_level));
        sector_field_row(ui, "6", "type",    &format!("{:>4}", s.sector_type));
        sector_field_row(ui, "7", "tag",     &format!("{:>4}", s.tag));

        // K row: middle wall texture used by sidedefs in this sector. Shows
        // "(varies)" when sidedefs disagree so the user knows K will overwrite.
        let wall_tex = sector_wall_texture(map, idx);
        let display = wall_tex.clone().unwrap_or_else(|| "(varies)".to_string());
        sector_texture_row(ui, state, bank, "K", &display);
    });
}

/// Render a numbered/lettered sector property row: `<key> <label> <value>` so
/// pressing the key in Sector mode triggers the matching edit.
fn sector_field_row(ui: &mut egui::Ui, key: &str, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.colored_label(theme::VGA_YELLOW, key);
        ui.colored_label(theme::VGA_BRIGHT_CYAN, label);
        ui.colored_label(theme::VGA_WHITE, value);
    });
}

/// Texture row variant for sector panel — leading hotkey letter, then the
/// texture name with the same hover-preview popup as linedef texture rows.
fn sector_texture_row(
    ui: &mut egui::Ui,
    state: &EditorState,
    bank: &mut TextureBank,
    key: &str,
    name: &str,
) {
    let kind = match key {
        "K" => TexKind::Wall,
        _ => TexKind::Flat,
    };
    let display = if name.is_empty() || name == "-" {
        "-".to_string()
    } else {
        name.to_string()
    };
    let resp = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.colored_label(theme::VGA_YELLOW, key);
            ui.add(
                egui::Label::new(RichText::new(&display).color(theme::VGA_WHITE))
                    .sense(egui::Sense::hover()),
            )
        })
        .inner;

    if resp.hovered() && display != "-" && display != "(varies)" {
        if let Some(pos) = ui.ctx().pointer_hover_pos() {
            paint_texture_popup(ui.ctx(), state, bank, pos, &display, kind);
        }
    }
}

/// Find the wall texture used by sector `sector_idx`. Returns the middle
/// texture when every sidedef referencing the sector uses the same one;
/// returns None when they differ (so the panel can show "(varies)").
fn sector_wall_texture(map: &crate::wad::MapData, sector_idx: usize) -> Option<String> {
    let want: u16 = sector_idx as u16;
    let mut found: Option<&str> = None;
    for sd in &map.sidedefs {
        if sd.sector != want {
            continue;
        }
        let tex = sd.middle_texture.as_str();
        match found {
            None => found = Some(tex),
            Some(prev) if prev == tex => {}
            Some(_) => return None, // disagreement
        }
    }
    found.map(|s| s.to_string())
}

fn thing_panel(ui: &mut egui::Ui, state: &EditorState) {
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let Some(map) = &state.map else {
            ui.colored_label(theme::VGA_DARK_GRAY, "(no map)");
            return;
        };
        let idx = focus_index(state);
        let Some(t) = map.things.get(idx) else {
            ui.colored_label(theme::VGA_DARK_GRAY, "(no thing)");
            return;
        };
        ui.colored_label(theme::VGA_WHITE, format!("Thing {idx}"));
        ui.colored_label(theme::VGA_WHITE, format!("X: {:>6}", t.x));
        ui.colored_label(theme::VGA_WHITE, format!("Y: {:>6}", t.y));
        ui.colored_label(theme::VGA_WHITE, format!("angle: {}", t.angle));
        ui.colored_label(theme::VGA_WHITE, format!("type:  {}", t.thing_type));
        ui.colored_label(theme::VGA_WHITE, format!("flags: {:#06x}", t.flags));
    });
}

fn placeholder(ui: &mut egui::Ui, msg: &str) {
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 4.0));
    frame.show(ui, |ui| {
        ui.colored_label(theme::VGA_DARK_GRAY, msg);
    });
}

#[cfg(test)]
mod tests {
    use super::supported_linedefs;
    use crate::wad::{LineDef, MapData, Vertex};

    #[test]
    fn supported_linedefs_finds_both_endpoints_with_letters() {
        // Vertex 0 is shared by two linedefs: 0→1 horizontal, 0→2 vertical.
        let map = MapData {
            name: "T".into(),
            vertices: vec![
                Vertex { x: 0, y: 0 },
                Vertex { x: 100, y: 0 },
                Vertex { x: 0, y: 50 },
            ],
            linedefs: vec![
                LineDef {
                    start_vertex: 0, end_vertex: 1,
                    flags: 0, special_type: 0, sector_tag: 0,
                    front_sidedef: LineDef::NO_SIDEDEF, back_sidedef: LineDef::NO_SIDEDEF,
                },
                LineDef {
                    start_vertex: 2, end_vertex: 0,
                    flags: 0, special_type: 0, sector_tag: 0,
                    front_sidedef: LineDef::NO_SIDEDEF, back_sidedef: LineDef::NO_SIDEDEF,
                },
            ],
            sidedefs: vec![],
            sectors: vec![],
            things: vec![],
        };
        let lines = supported_linedefs(&map, 0);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].letter, 'A');
        assert_eq!(lines[0].linedef_idx, 0);
        assert!((lines[0].length - 100.0).abs() < 1e-3);
        // Linedef 0→1 points along +X, angle should be ~0.
        assert!(lines[0].angle.abs() < 1e-3);
        assert_eq!(lines[1].letter, 'B');
        assert_eq!(lines[1].linedef_idx, 1);
        // Linedef 2→0 — from vertex 0's perspective the OTHER endpoint is
        // vertex 2 at (0, 50), so direction is +Y → angle ≈ pi/2.
        assert!((lines[1].angle - std::f32::consts::FRAC_PI_2).abs() < 1e-3);
    }
}
