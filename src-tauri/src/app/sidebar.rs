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
    selection_block(ui, state);
    separator(ui);
    compass_rosette(ui);
    separator(ui);
    linedef_table(ui, state);

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
        let bg = if is_open { theme::VGA_GRAY } else { theme::SIDEBAR_BG };
        let fg = if is_open { theme::VGA_BLACK } else { theme::VGA_WHITE };
        let chevron_color = if is_open { theme::VGA_BLACK } else { theme::VGA_GRAY };

        let desired = egui::vec2(ui.available_width(), 14.0);
        let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, bg);

        let font = egui::FontId::new(12.0, egui::FontFamily::Monospace);
        painter.text(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            name,
            font.clone(),
            fg,
        );
        painter.text(
            egui::pos2(rect.right() - 4.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            ">",
            font,
            chevron_color,
        );

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
        let map_name = state.map.as_ref().map(|m| m.name.as_str()).unwrap_or("untitled");
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

fn selection_block(ui: &mut egui::Ui, state: &EditorState) {
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        let total = state.total_for_mode();
        let sel = state.selection.len();
        ui.colored_label(theme::VGA_WHITE, format!("{sel}/{total}"));
        ui.colored_label(theme::VGA_BRIGHT_CYAN, "Supported");
        ui.colored_label(theme::VGA_BRIGHT_CYAN, "LineDefs:");
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

fn linedef_table(ui: &mut egui::Ui, state: &EditorState) {
    let frame = egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(4.0, 2.0));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.colored_label(theme::VGA_BRIGHT_CYAN, "LD#");
            ui.colored_label(theme::VGA_BRIGHT_CYAN, "length");
        });
        let Some(map) = &state.map else {
            ui.colored_label(theme::VGA_DARK_GRAY, "(no map)");
            return;
        };
        // Show first 3 selected LineDefs by length, or first 3 in the map if no selection.
        let take: Vec<usize> = if state.selection.is_empty() {
            (0..map.linedefs.len().min(3)).collect()
        } else {
            state.selection.iter().take(3).copied().collect()
        };
        for idx in take {
            let Some(ld) = map.linedefs.get(idx) else { continue };
            let (Some(a), Some(b)) = (
                map.vertices.get(ld.start_vertex as usize),
                map.vertices.get(ld.end_vertex as usize),
            ) else { continue };
            let dx = (a.x - b.x) as f32;
            let dy = (a.y - b.y) as f32;
            let len = (dx * dx + dy * dy).sqrt();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.colored_label(theme::VGA_WHITE, format!("{idx:>3}"));
                ui.colored_label(theme::VGA_WHITE, format!("{len:>7.3}"));
            });
        }
    });
}
