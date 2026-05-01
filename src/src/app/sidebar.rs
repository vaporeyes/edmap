// ABOUTME: Left sidebar — title, menu list, MAP info box, status fields, mode tabs, compass, LD# table.
// ABOUTME: Layout matches the second screenshot top-to-bottom.

use eframe::egui::{self, Color32, RichText};

use super::menu::MENU_ORDER;
use super::state::{EditorState, SelectionMode};
use crate::theme;

pub fn draw(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

    menu_list(ui, state);
    separator(ui);
    info_box(ui, state);
    separator(ui);
    status_block(ui, state);
    separator(ui);
    mode_tabs(ui, state);
    separator(ui);
    counter_line(ui, state);
    separator(ui);
    match state.mode {
        SelectionMode::LineDef => linedef_panel(ui, state),
        SelectionMode::Vertex => vertex_panel(ui, state),
        SelectionMode::Sector => sector_panel(ui, state),
        SelectionMode::Thing => thing_panel(ui, state),
    }

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

        let font = egui::FontId::new(12.0, egui::FontFamily::Monospace);
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

fn info_box(ui: &mut egui::Ui, state: &EditorState) {
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
        ui.label(RichText::new(format!("{:.2}k free", free_memory_estimate(state))).color(theme::VGA_BRIGHT_GREEN).size(11.0));
        ui.add_space(2.0);
        ui.label(RichText::new("press F1").color(theme::VGA_WHITE).size(11.0));
        ui.label(RichText::new("for help").color(theme::VGA_WHITE).size(11.0));
    });
}

fn free_memory_estimate(_state: &EditorState) -> f32 {
    // Cosmetic — modern systems have GB free. Show a believable VGA-era number.
    187.82
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

fn compass_rosette(ui: &mut egui::Ui) {
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 4.0));
    frame.show(ui, |ui| {
        let size = egui::vec2(ui.available_width().min(80.0), 60.0);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let center = rect.center();
        let r = size.y * 0.4;
        // Cross arms
        painter.line_segment(
            [egui::pos2(center.x - r, center.y), egui::pos2(center.x + r, center.y)],
            egui::Stroke::new(1.0, theme::VGA_BRIGHT_CYAN),
        );
        painter.line_segment(
            [egui::pos2(center.x, center.y - r), egui::pos2(center.x, center.y + r)],
            egui::Stroke::new(1.0, theme::VGA_BRIGHT_CYAN),
        );
        // Center circle
        painter.circle_stroke(center, 6.0, egui::Stroke::new(1.0, theme::VGA_BRIGHT_CYAN));
        let font = egui::FontId::new(11.0, egui::FontFamily::Monospace);
        // Cardinal labels — H N high, L low etc per screenshot's visual cue.
        for (label, dx, dy) in [("H", 0.0, -r - 8.0), ("L", 0.0, r + 8.0), ("G", -r - 8.0, 0.0), ("E", r + 8.0, 0.0)] {
            painter.text(
                center + egui::vec2(dx, dy),
                egui::Align2::CENTER_CENTER,
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

fn linedef_panel(ui: &mut egui::Ui, state: &EditorState) {
    let Some(map) = &state.map else {
        return placeholder(ui, "(no map)");
    };
    let ld_idx = state.selection.first().copied().unwrap_or(0);
    let Some(ld) = map.linedefs.get(ld_idx) else {
        return placeholder(ui, "(no linedef)");
    };

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
                texture_row(ui, "U", &sd.upper_texture);
                texture_row(ui, "M", &sd.middle_texture);
                texture_row(ui, "L", &sd.lower_texture);
            }
        }
        if ld.is_two_sided() && ld.back_sidedef != crate::wad::LineDef::NO_SIDEDEF {
            if let Some(sd) = map.sidedefs.get(ld.back_sidedef as usize) {
                texture_row(ui, "N", &sd.upper_texture);
                texture_row(ui, "B", &sd.middle_texture);
                texture_row(ui, "R", &sd.lower_texture);
            }
        }
    });
}

fn texture_row(ui: &mut egui::Ui, prefix: &str, name: &str) {
    let display = if name.is_empty() || name == "-" { "-".to_string() } else { name.to_string() };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.colored_label(theme::VGA_BRIGHT_CYAN, format!("{prefix}:"));
        ui.colored_label(theme::VGA_WHITE, display);
    });
}

fn vertex_panel(ui: &mut egui::Ui, state: &EditorState) {
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let Some(map) = &state.map else {
            ui.colored_label(theme::VGA_DARK_GRAY, "(no map)");
            return;
        };
        let idx = state.selection.first().copied().unwrap_or(0);
        let Some(v) = map.vertices.get(idx) else {
            ui.colored_label(theme::VGA_DARK_GRAY, "(no vertex)");
            return;
        };
        ui.colored_label(theme::VGA_WHITE, format!("Vertex {idx}"));
        ui.colored_label(theme::VGA_WHITE, format!("X: {:>6}", v.x));
        ui.colored_label(theme::VGA_WHITE, format!("Y: {:>6}", v.y));
    });
}

fn sector_panel(ui: &mut egui::Ui, state: &EditorState) {
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let Some(map) = &state.map else {
            ui.colored_label(theme::VGA_DARK_GRAY, "(no map)");
            return;
        };
        let idx = state.selection.first().copied().unwrap_or(0);
        let Some(s) = map.sectors.get(idx) else {
            ui.colored_label(theme::VGA_DARK_GRAY, "(no sector)");
            return;
        };
        ui.colored_label(theme::VGA_WHITE, format!("Sector {idx}"));
        ui.colored_label(theme::VGA_WHITE, format!("ceiling: {}", s.ceiling_height));
        ui.colored_label(theme::VGA_WHITE, format!("floor:   {}", s.floor_height));
        ui.colored_label(theme::VGA_WHITE, format!("light:   {}", s.light_level));
        ui.colored_label(theme::VGA_WHITE, format!("type:    {}", s.sector_type));
        ui.colored_label(theme::VGA_WHITE, format!("tag:     {}", s.tag));
        ui.add_space(2.0);
        texture_row(ui, "C", &s.ceiling_texture);
        texture_row(ui, "F", &s.floor_texture);
    });
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
        let idx = state.selection.first().copied().unwrap_or(0);
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
