// ABOUTME: Modal dialog renderer — VGA-styled centered window with inverse title bar.
// ABOUTME: One match arm per Dialog variant; result is dispatched as commands when OK clicked.

use eframe::egui::{self, Align2, Color32, RichText};

use super::commands;
use super::state::{Dialog, EditorState};
use crate::theme;

pub fn draw(ctx: &egui::Context, state: &mut EditorState) {
    let Some(dialog) = state.dialog.clone() else { return };
    let mut close = false;

    let title = title_for(&dialog);
    let screen_rect = ctx.screen_rect();

    egui::Area::new(egui::Id::new("dialog_area"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(theme::MENU_BG)
                .stroke(egui::Stroke::new(1.0, theme::VGA_BLACK))
                .inner_margin(egui::Margin::same(2.0))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    let max_width = (screen_rect.width() * 0.6).min(560.0).max(280.0);
                    ui.set_max_width(max_width);

                    // Bevel the whole window frame so it feels raised off the viewport.
                    let frame_rect = ui.max_rect();
                    theme::draw_bevel(&ui.painter().clone(), frame_rect, false);

                    title_bar(ui, title);
                    egui::Frame::none()
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                            close = body(ui, state, dialog);
                        });
                });
        });

    if close {
        state.dialog = None;
    }
}

fn title_bar(ui: &mut egui::Ui, title: &str) {
    let desired = egui::vec2(ui.available_width().max(ui.min_size().x), 16.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::MENU_HILITE_BG);
    painter.text(
        egui::pos2(rect.left() + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        title,
        egui::FontId::new(12.0, egui::FontFamily::Monospace),
        theme::MENU_HILITE_FG,
    );
    let _ = Color32::TRANSPARENT;
}

fn title_for(dialog: &Dialog) -> &'static str {
    match dialog {
        Dialog::About => "About EdMap",
        Dialog::MapInformation => "Map Information",
        Dialog::SystemInformation => "System Information",
        Dialog::SnapGrid { .. } => "Grid/snap sizes",
        Dialog::GotoObject { .. } => "Goto object",
        Dialog::WadList => "Active PWADs",
        Dialog::OpenMapPicker { .. } => "Open Map",
        Dialog::Notice { .. } => "Notice",
    }
}

/// Returns true if the dialog should close after this frame.
fn body(ui: &mut egui::Ui, state: &mut EditorState, dialog: Dialog) -> bool {
    match dialog {
        Dialog::About => about_body(ui),
        Dialog::MapInformation => map_info_body(ui, state),
        Dialog::SystemInformation => system_info_body(ui, state),
        Dialog::SnapGrid { grid, snap } => snap_grid_body(ui, state, grid, snap),
        Dialog::GotoObject { input } => goto_body(ui, state, input),
        Dialog::WadList => wad_list_body(ui, state),
        Dialog::OpenMapPicker { maps, selected } => open_map_body(ui, state, maps, selected),
        Dialog::Notice { title: _, message } => notice_body(ui, message),
    }
}

fn about_body(ui: &mut egui::Ui) -> bool {
    ui.colored_label(theme::VGA_WHITE, "EdMap v1.40");
    ui.colored_label(theme::MENU_FG, "DOOM-I/-II/HERETIC map editor");
    ui.add_space(2.0);
    ui.colored_label(theme::MENU_FG, "Original 1994 by Jeff Rabenhorst");
    ui.colored_label(theme::MENU_FG, "araya@wam.umd.edu");
    ui.add_space(2.0);
    ui.colored_label(theme::MENU_FG, "Rust + egui rebuild, 2026");
    ui.add_space(6.0);
    ok_button(ui, "OK")
}

fn map_info_body(ui: &mut egui::Ui, state: &mut EditorState) -> bool {
    let Some(map) = &state.map else {
        ui.colored_label(theme::VGA_BRIGHT_RED, "No map loaded.");
        return ok_button(ui, "OK");
    };
    ui.colored_label(theme::VGA_WHITE, format!("Map name: {}", map.name));
    ui.colored_label(theme::MENU_FG, format!("  Vertices: {:>5}", map.vertices.len()));
    ui.colored_label(theme::MENU_FG, format!("  LineDefs: {:>5}", map.linedefs.len()));
    ui.colored_label(theme::MENU_FG, format!("  SideDefs: {:>5}", map.sidedefs.len()));
    ui.colored_label(theme::MENU_FG, format!("  Sectors:  {:>5}", map.sectors.len()));
    ui.colored_label(theme::MENU_FG, format!("  Things:   {:>5}", map.things.len()));
    ui.add_space(4.0);
    let bytes = estimate_unbuilt_size(map);
    ui.colored_label(
        theme::MENU_FG,
        format!("File size (unbuilt): {} bytes (~{:.1} KB)", bytes, bytes as f32 / 1024.0),
    );
    if let Some(path) = &state.wad_path {
        ui.add_space(2.0);
        ui.colored_label(theme::VGA_DARK_GRAY, format!("from {}", path.display()));
    }
    ui.add_space(6.0);
    ok_button(ui, "OK")
}

fn estimate_unbuilt_size(map: &crate::wad::MapData) -> usize {
    use crate::wad::{LineDef, Sector, SideDef, Thing, Vertex};
    map.vertices.len() * Vertex::SIZE
        + map.linedefs.len() * LineDef::SIZE
        + map.sidedefs.len() * SideDef::SIZE
        + map.sectors.len() * Sector::SIZE
        + map.things.len() * Thing::SIZE
}

fn system_info_body(ui: &mut egui::Ui, state: &mut EditorState) -> bool {
    ui.colored_label(theme::VGA_WHITE, "Configuration");
    let game = match state.map.as_ref().and_then(|m| crate::wad::MapName::parse(&m.name)) {
        Some(crate::wad::MapName::Episode { .. }) => "DOOM I / Heretic",
        Some(crate::wad::MapName::Map { .. }) => "DOOM II",
        None => "(unknown)",
    };
    ui.colored_label(theme::MENU_FG, format!("  Game: {game}"));
    if let Some(path) = &state.wad_path {
        ui.colored_label(theme::MENU_FG, format!("  Data: {}", path.display()));
    } else {
        ui.colored_label(theme::MENU_FG, "  Data: (no WAD loaded)");
    }
    ui.add_space(4.0);
    ui.colored_label(theme::VGA_WHITE, "Resources");
    if let Some(map) = &state.map {
        ui.colored_label(theme::MENU_FG, format!("  Vertices: {}", map.vertices.len()));
        ui.colored_label(theme::MENU_FG, format!("  LineDefs: {}", map.linedefs.len()));
        ui.colored_label(theme::MENU_FG, format!("  SideDefs: {}", map.sidedefs.len()));
        ui.colored_label(theme::MENU_FG, format!("  Sectors:  {}", map.sectors.len()));
        ui.colored_label(theme::MENU_FG, format!("  Things:   {}", map.things.len()));
    }
    ui.add_space(6.0);
    ok_button(ui, "OK")
}

fn snap_grid_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    grid: String,
    snap: String,
) -> bool {
    let mut grid = grid;
    let mut snap = snap;
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Grid size:");
        ui.add(text_box(&mut grid, 6));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Snap size:");
        ui.add(text_box(&mut snap, 6));
    });
    ui.add_space(6.0);

    let close = ui
        .horizontal(|ui| {
            let ok = button(ui, "OK").clicked();
            let cancel = button(ui, "cancel").clicked();
            if ok {
                if let Ok(g) = grid.trim().parse::<i32>() {
                    if g > 0 && g <= 4096 {
                        state.grid_size = g;
                    }
                }
                if let Ok(s) = snap.trim().parse::<i32>() {
                    if s > 0 && s <= 4096 {
                        state.snap_size = s;
                    }
                }
                return true;
            }
            cancel
        })
        .inner;

    if !close {
        state.dialog = Some(Dialog::SnapGrid { grid, snap });
    }
    close
}

fn goto_body(ui: &mut egui::Ui, state: &mut EditorState, input: String) -> bool {
    let total = state.total_for_mode();
    ui.colored_label(
        theme::MENU_FG,
        format!("Goto {} (0..{}):", state.mode.label_long(), total.saturating_sub(1)),
    );
    let mut input = input;
    ui.add(text_box(&mut input, 8));
    ui.add_space(6.0);

    let close = ui
        .horizontal(|ui| {
            let ok = button(ui, "OK").clicked();
            let cancel = button(ui, "cancel").clicked();
            if ok {
                if let Ok(n) = input.trim().parse::<usize>() {
                    if n < total {
                        state.selection.clear();
                        state.selection.push(n);
                        commands::focus_on_selection(state);
                    } else {
                        state.dialog = Some(Dialog::Notice {
                            title: "Goto".into(),
                            message: "Invalid object number.".into(),
                        });
                        return false;
                    }
                }
                return true;
            }
            cancel
        })
        .inner;

    if !close && state.dialog.is_some() {
        // Notice was set above — leave it.
    } else if !close {
        state.dialog = Some(Dialog::GotoObject { input });
    }
    close
}

fn wad_list_body(ui: &mut egui::Ui, state: &mut EditorState) -> bool {
    let Some(wad) = &state.wad else {
        ui.colored_label(theme::VGA_BRIGHT_RED, "No WAD loaded.");
        return ok_button(ui, "OK");
    };
    let kind = match wad.header.kind {
        crate::wad::WadKind::Iwad => "IWAD",
        crate::wad::WadKind::Pwad => "PWAD",
    };
    let path_str = state
        .wad_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(in memory)".into());
    ui.colored_label(theme::VGA_WHITE, format!("{kind}: {path_str}"));
    let map_count = wad.map_names().len();
    ui.colored_label(theme::MENU_FG, format!("  {} lumps total", wad.directory.len()));
    ui.colored_label(theme::MENU_FG, format!("  {map_count} maps"));
    ui.add_space(6.0);
    ok_button(ui, "OK")
}

fn open_map_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    maps: Vec<String>,
    selected: usize,
) -> bool {
    if maps.is_empty() {
        ui.colored_label(theme::VGA_BRIGHT_RED, "No maps in this WAD.");
        return ok_button(ui, "OK");
    }
    ui.colored_label(theme::MENU_FG, "Select a map:");
    let mut selected = selected.min(maps.len() - 1);
    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
        for (i, name) in maps.iter().enumerate() {
            let is_active = i == selected;
            let bg = if is_active { theme::MENU_HILITE_BG } else { theme::MENU_BG };
            let fg = if is_active { theme::MENU_HILITE_FG } else { theme::MENU_FG };
            let row = egui::Frame::none()
                .fill(bg)
                .inner_margin(egui::Margin::symmetric(6.0, 1.0))
                .show(ui, |ui| ui.label(RichText::new(name).color(fg)))
                .response
                .interact(egui::Sense::click());
            if row.clicked() {
                selected = i;
            }
            if row.double_clicked() {
                load_selected_map(state, maps[i].clone());
                return;
            }
        }
    });
    ui.add_space(6.0);

    let close = ui
        .horizontal(|ui| {
            let ok = button(ui, "OK").clicked();
            let cancel = button(ui, "cancel").clicked();
            if ok {
                let name = maps[selected].clone();
                load_selected_map(state, name);
                return true;
            }
            cancel
        })
        .inner;

    if !close {
        state.dialog = Some(Dialog::OpenMapPicker { maps, selected });
    }
    close
}

fn notice_body(ui: &mut egui::Ui, message: String) -> bool {
    ui.colored_label(theme::VGA_BRIGHT_RED, message);
    ui.add_space(6.0);
    ok_button(ui, "OK")
}

fn load_selected_map(state: &mut EditorState, name: String) {
    let Some(wad) = &state.wad else { return };
    match wad.load_map(&name) {
        Ok(map) => {
            state.map = Some(map);
            state.selection.clear();
            commands::center_map(state);
        }
        Err(e) => {
            state.dialog = Some(Dialog::Notice {
                title: "Open Map".into(),
                message: format!("Load failed: {e}"),
            });
        }
    }
}

fn ok_button(ui: &mut egui::Ui, label: &str) -> bool {
    button(ui, label).clicked()
}

fn button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let desired = egui::vec2(label.len() as f32 * 8.0 + 20.0, 18.0);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());
    let painter = ui.painter_at(rect);
    let hovered = resp.hovered();
    let pressed = resp.is_pointer_button_down_on();
    let bg = if hovered { theme::MENU_HILITE_BG } else { theme::MENU_BG };
    let fg = if hovered { theme::MENU_HILITE_FG } else { theme::MENU_FG };
    painter.rect_filled(rect, 0.0, bg);
    theme::draw_bevel(&painter, rect, pressed);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        egui::FontId::new(12.0, egui::FontFamily::Monospace),
        fg,
    );
    resp
}

fn text_box<'a>(buf: &'a mut String, char_width: usize) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(buf)
        .desired_width(char_width as f32 * 9.0)
        .text_color(theme::VGA_BLACK)
        .font(egui::FontId::new(12.0, egui::FontFamily::Monospace))
}

/// Helpers for SelectionMode that only matter inside dialogs.
trait SelectionModeExt {
    fn label_long(self) -> &'static str;
}

impl SelectionModeExt for super::SelectionMode {
    fn label_long(self) -> &'static str {
        match self {
            super::SelectionMode::Vertex => "vertex",
            super::SelectionMode::LineDef => "LineDef",
            super::SelectionMode::Sector => "sector",
            super::SelectionMode::Thing => "thing",
        }
    }
}
