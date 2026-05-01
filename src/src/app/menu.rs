// ABOUTME: Cascading menu rendering. Source of truth for menu items + keybindings is the UX spec.
// ABOUTME: Items either trigger commands directly or set state.open_menu to None when clicked.

use eframe::egui::{self, Color32};

use super::state::EditorState;
use crate::theme;
use crate::wad::Wad;

pub const MENU_ORDER: &[&str] = &[
    "Info",
    "File (map)",
    "WAD list",
    "Edit",
    "Map utilities",
    "Sectors",
    "Automatic",
    "Display",
    "Check",
];

/// Render the cascading panel for whichever menu is currently open.
/// Anchored to the right edge of the sidebar — matches the screenshot.
pub fn draw_open_menu(ctx: &egui::Context, state: &mut EditorState) {
    let Some(open) = state.open_menu else { return };

    let area_pos = egui::pos2(160.0, 16.0);
    let mut close_after = false;

    egui::Area::new(egui::Id::new(("menu_popup", open)))
        .order(egui::Order::Foreground)
        .fixed_pos(area_pos)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(theme::MENU_BG)
                .stroke(egui::Stroke::new(1.0, theme::VGA_BLACK))
                .inner_margin(egui::Margin::same(2.0))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    ui.set_min_width(220.0);

                    // Raised-window bevel framing the whole cascade panel.
                    let frame_rect = ui.max_rect();
                    theme::draw_bevel(&ui.painter().clone(), frame_rect, false);

                    cascade_header(ui, open);
                    let items = items_for(open);
                    for (label, hotkey) in items {
                        let resp = menu_row(ui, label, hotkey);
                        if resp.clicked() {
                            handle_command(state, open, label);
                            close_after = true;
                        }
                    }
                });
        });

    if close_after {
        state.open_menu = None;
    } else if ctx.input(|i| i.pointer.any_click()) {
        // Click outside any menu surface closes it.
        let pointer = ctx.input(|i| i.pointer.interact_pos()).unwrap_or_default();
        let in_top_bar = pointer.y < 18.0;
        let in_sidebar = pointer.x < 160.0 && pointer.y < 18.0 + (MENU_ORDER.len() as f32) * 14.0;
        if !in_top_bar && !in_sidebar {
            state.open_menu = None;
        }
    }
}

fn cascade_header(ui: &mut egui::Ui, name: &str) {
    let desired = egui::vec2(ui.available_width(), 15.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::MENU_HILITE_BG);
    let font = egui::FontId::new(12.0, egui::FontFamily::Monospace);
    painter.text(
        egui::pos2(rect.left() + 4.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        name,
        font,
        theme::MENU_HILITE_FG,
    );
}

fn menu_row(ui: &mut egui::Ui, label: &str, hotkey: &str) -> egui::Response {
    let desired = egui::vec2(ui.available_width(), 16.0);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());
    let painter = ui.painter_at(rect);

    let hovered = resp.hovered();
    let bg = if hovered { theme::MENU_HILITE_BG } else { theme::MENU_BG };
    let fg = if hovered { theme::MENU_HILITE_FG } else { theme::MENU_FG };

    painter.rect_filled(rect, 0.0, bg);
    // Pressed bevel when hovered (about-to-click look) so the row "depresses".
    theme::draw_bevel(&painter, rect, hovered);

    let font = egui::FontId::new(12.0, egui::FontFamily::Monospace);
    let label_pos = egui::pos2(rect.left() + 8.0, rect.center().y);
    painter.text(label_pos, egui::Align2::LEFT_CENTER, label, font.clone(), fg);

    if !hotkey.is_empty() {
        let hk_pos = egui::pos2(rect.right() - 8.0, rect.center().y);
        painter.text(hk_pos, egui::Align2::RIGHT_CENTER, hotkey, font, fg);
    }
    resp
}

pub fn items_for(menu: &str) -> &'static [(&'static str, &'static str)] {
    match menu {
        "Info" => &[
            ("About EdMap", ""),
            ("Help", "F1"),
            ("Calculator", "Num Lock"),
            ("Map Information", ""),
            ("System Information", ""),
            ("Load config file", ""),
            ("Edit config (EDMAPCFG)", ""),
            ("Preferences", ""),
        ],
        "File (map)" => &[
            ("New map", ""),
            ("Open map file", "F3"),
            ("Save map data", "F2"),
            ("Load PWAD map", "Shift-F3"),
            ("Rename map", ""),
            ("Build & save map", "F9"),
            ("Alternate build", "Alt-F9"),
            ("Play map", "Ctrl-F9"),
            ("Quit to DOS", "Alt-X"),
        ],
        "WAD list" => &[
            ("List WADs", "F4"),
            ("Save as PWAD...", "Ctrl-F2"),
            ("Add PWAD file", "Ctrl-F4"),
            ("Remove PWAD", ""),
            ("Write ADD file", ""),
        ],
        "Edit" => &[
            ("Add/split", "Ins"),
            ("Delete/merge", "BkSp"),
            ("Undo from last save", ""),
            ("Shift object", ""),
            ("Find objects", "Ctrl-F"),
            ("Goto object", "Ctrl-G"),
            ("Next object", ">"),
            ("Previous object", "<"),
            ("Tag line to sector", "F7"),
        ],
        "Map utilities" => &[
            ("Shift Map (X/Y/Z)", ""),
            ("Expand/reduce map", ""),
            ("Light adjustment", ""),
            ("Texture replace", ""),
        ],
        "Sectors" => &[
            ("Polygon", "Ctrl-P"),
            ("Rotate", "R"),
            ("Size", "Z"),
            ("Texture style", "Alt-F8"),
            ("Edit styles", "Ctrl-F8"),
            ("Grab style", "Shift-F8"),
            ("Align textures (X,Y)", "F8"),
            ("Configure align", ""),
        ],
        "Automatic" => &[
            ("Lift", "Alt-L"),
            ("Door", "Alt-D"),
            ("Stairs", "Alt-S"),
            ("Teleporter", "Alt-T"),
        ],
        "Display" => &[
            ("Enhance map", "Ctrl-E"),
            ("Full screen", "Ctrl-S"),
            ("Snap/grid", ""),
            ("Grid on/off", ""),
            ("Origin on/off", "Ctrl-O"),
            ("Center map", ""),
            ("Viewer", "F10"),
            ("Refresh display", ""),
        ],
        "Check" => &[
            ("Error list", "Ctrl-L"),
            ("Quick check", "F5"),
            ("Check all", "Ctrl-F5"),
            ("Textures", ""),
            ("Associations", ""),
            ("Heights/widths", ""),
            ("LineDefs", ""),
            ("Begin & end", ""),
            ("Sector integrety", ""),
        ],
        _ => &[],
    }
}

pub fn handle_command(state: &mut EditorState, menu: &str, item: &str) {
    use super::state::Dialog;
    match (menu, item) {
        ("Info", "About EdMap") => state.dialog = Some(Dialog::About),
        ("Info", "Map Information") => state.dialog = Some(Dialog::MapInformation),
        ("Info", "System Information") => state.dialog = Some(Dialog::SystemInformation),
        ("File (map)", "New map") => {
            state.map = None;
            state.wad = None;
            state.wad_path = None;
            state.selection.clear();
            state.view_center = egui::pos2(0.0, 0.0);
            state.view_zoom = 1.0;
        }
        ("File (map)", "Open map file") => open_wad_picker(state),
        ("File (map)", "Quit to DOS") => std::process::exit(0),
        ("WAD list", "List WADs") => state.dialog = Some(Dialog::WadList),
        ("WAD list", "Add PWAD file") => open_wad_picker(state),
        ("Edit", "Next object") => super::commands::cycle_selection(state, 1),
        ("Edit", "Previous object") => super::commands::cycle_selection(state, -1),
        ("Edit", "Goto object") => {
            state.dialog = Some(Dialog::GotoObject { input: String::new() });
        }
        ("Display", "Grid on/off") => state.grid_visible = !state.grid_visible,
        ("Display", "Origin on/off") => state.origin_visible = !state.origin_visible,
        ("Display", "Center map") => super::commands::center_map(state),
        ("Display", "Refresh display") => state.status_message = None,
        ("Display", "Viewer") => {
            state.viewer_open = !state.viewer_open;
        }
        ("Display", "Snap/grid") => {
            state.dialog = Some(Dialog::SnapGrid {
                grid: state.grid_size.to_string(),
                snap: state.snap_size.to_string(),
            });
        }
        _ => {
            state.status_message = Some(format!("[{menu}] {item}: not implemented yet"));
        }
    }
}

fn open_wad_picker(state: &mut EditorState) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("WAD files", &["wad", "WAD"])
        .pick_file()
    else {
        return;
    };
    match Wad::from_path(&path) {
        Ok(wad) => {
            let maps = wad.map_names();
            state.wad_path = Some(path);
            state.wad = Some(wad);
            state.map = None;
            state.selection.clear();
            state.status_message = None;
            match maps.len() {
                0 => {
                    state.status_message = Some("WAD\\No maps in this file.".into());
                }
                1 => {
                    if let Some(wad) = &state.wad {
                        if let Ok(m) = wad.load_map(&maps[0]) {
                            state.map = Some(m);
                            state.view_center = compute_map_center(state);
                            state.view_zoom = compute_initial_zoom(state);
                        }
                    }
                }
                _ => {
                    state.dialog = Some(super::state::Dialog::OpenMapPicker {
                        maps,
                        selected: 0,
                    });
                }
            }
        }
        Err(e) => {
            state.status_message = Some(format!("PWAD Load: {e}"));
        }
    }
}

fn compute_map_center(state: &EditorState) -> egui::Pos2 {
    let Some(map) = &state.map else { return egui::pos2(0.0, 0.0) };
    if map.vertices.is_empty() {
        return egui::pos2(0.0, 0.0);
    }
    let mut min = egui::pos2(f32::INFINITY, f32::INFINITY);
    let mut max = egui::pos2(f32::NEG_INFINITY, f32::NEG_INFINITY);
    for v in &map.vertices {
        let p = egui::pos2(v.x as f32, v.y as f32);
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    egui::pos2((min.x + max.x) * 0.5, (min.y + max.y) * 0.5)
}

fn compute_initial_zoom(state: &EditorState) -> f32 {
    let Some(map) = &state.map else { return 1.0 };
    if map.vertices.is_empty() {
        return 1.0;
    }
    let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
    for v in &map.vertices {
        min_x = min_x.min(v.x as f32);
        max_x = max_x.max(v.x as f32);
        min_y = min_y.min(v.y as f32);
        max_y = max_y.max(v.y as f32);
    }
    let w = (max_x - min_x).max(1.0);
    let h = (max_y - min_y).max(1.0);
    // Target a viewport of roughly 600x500; pick the more constraining axis.
    (600.0 / w).min(500.0 / h).max(0.05)
}

pub fn draw_top_strip(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for &name in MENU_ORDER {
            let is_open = state.open_menu == Some(name);
            let bg = if is_open { theme::VGA_GRAY } else { theme::SIDEBAR_BG };
            let fg = if is_open { theme::VGA_BLACK } else { theme::VGA_WHITE };
            let label = egui::RichText::new(name).color(fg);
            let frame = egui::Frame::none().fill(bg).inner_margin(egui::Margin::symmetric(6.0, 1.0));
            let resp = frame.show(ui, |ui| ui.label(label)).response;
            let resp = resp.interact(egui::Sense::click());
            if resp.clicked() {
                state.open_menu = if is_open { None } else { Some(name) };
            } else if resp.hovered() && state.open_menu.is_some() && !is_open {
                state.open_menu = Some(name);
            }
        }
        let _ = Color32::TRANSPARENT;
    });
}
