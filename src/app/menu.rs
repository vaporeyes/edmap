// ABOUTME: Cascading menu rendering. Source of truth for menu items + keybindings is the UX spec.
// ABOUTME: Items either trigger commands directly or set state.open_menu to None when clicked.

use eframe::egui::{self, Color32};

use super::state::EditorState;
use crate::theme;
#[cfg(not(target_arch = "wasm32"))]
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
pub fn draw_open_menu(ctx: &egui::Context, state: &mut EditorState, tx: &std::sync::mpsc::Sender<crate::app::AsyncCommand>) {
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
                            handle_command(state, open, label, tx);
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
    let desired = egui::vec2(ui.available_width(), 16.0);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::MENU_HILITE_BG);
    let font = egui::FontId::new(13.0, egui::FontFamily::Proportional);
    painter.text(
        egui::pos2(rect.left() + 6.0, rect.center().y),
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

    let font = egui::FontId::new(13.0, egui::FontFamily::Proportional);
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
            ("Calculator", "Ctrl-K"),
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
            ("Test map settings", ""),
            ("Quit to DOS", "Alt-X"),
        ],
        "WAD list" => &[
            ("List WADs", "F4"),
            ("Save as PWAD...", "Ctrl-F2"),
            ("Add PWAD file", "Ctrl-F4"),
            ("Remove PWAD", ""),
            ("Write ADD file", ""),
            ("Save selection as prefab", ""),
            ("Load prefab", ""),
        ],
        "Edit" => &[
            ("Add/split", "Ins"),
            ("Delete/merge", "BkSp"),
            ("Line-draw mode", ""),
            ("Undo from last save", ""),
            ("Shift object", ""),
            ("Copy", "Ctrl-C"),
            ("Paste", "Ctrl-V"),
            ("Flip selection horizontal", ""),
            ("Flip selection vertical", ""),
            ("Rotate selection", ""),
            ("Scale selection", ""),
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
            ("Fix zero-length linedefs", ""),
            ("Fix missing textures", ""),
            ("Remove unused textures", ""),
            ("Enhance map", ""),
        ],
        "Sectors" => &[
            ("Polygon", "Ctrl-P"),
            ("Curve LineDef", ""),
            ("Join sectors", ""),
            ("Merge sectors", ""),
            ("Gradient floors", ""),
            ("Gradient ceilings", ""),
            ("Gradient brightness", ""),
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
            ("Grid on/off", "G"),
            ("Grid intensity", ""),
            ("Origin on/off", "Ctrl-O"),
            ("Center map", ""),
            ("Viewer", "F10"),
            ("Things filter", ""),
            ("Thing bounding boxes", ""),
            ("Export picture", ""),
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

pub fn handle_command(state: &mut EditorState, menu: &str, item: &str, tx: &std::sync::mpsc::Sender<crate::app::AsyncCommand>) {
    use super::state::Dialog;
    match (menu, item) {
        ("Info", "About EdMap") => state.dialog = Some(Dialog::About),
        ("Info", "Calculator") => super::calculator::toggle(state),
        ("Info", "Preferences") => state.dialog = Some(Dialog::Preferences),
        ("Info", "Map Information") => state.dialog = Some(Dialog::MapInformation),
        ("Info", "System Information") => state.dialog = Some(Dialog::SystemInformation),
        ("File (map)", "New map") => {
            if super::commands::dirty_guard(state, super::state::PendingAction::NewMap) {
                super::commands::new_map(state);
            }
        }
        ("File (map)", "Open map file") => {
            if super::commands::dirty_guard(state, super::state::PendingAction::OpenWad) {
                open_wad_picker(state, tx);
            }
        }
        ("File (map)", "Load PWAD map") => {
            let Some(wad) = &state.wad else {
                state.status_message = Some("Load PWAD map: no WAD loaded.".into());
                return;
            };
            let maps = wad.map_names();
            if maps.is_empty() {
                state.status_message = Some("Load PWAD map: this WAD has no maps.".into());
                return;
            }
            if !super::commands::dirty_guard(state, super::state::PendingAction::OpenWad) {
                return;
            }
            // Preselect the current map so the picker opens on it.
            let selected = state
                .map
                .as_ref()
                .and_then(|m| maps.iter().position(|n| n == &m.name))
                .unwrap_or(0);
            state.dialog = Some(Dialog::OpenMapPicker { maps, selected });
        }
        ("File (map)", "Save map data") => super::commands::save_map(state),
        ("File (map)", "Play map") => super::commands::test_map(state),
        ("File (map)", "Test map settings") => super::commands::open_test_map_settings(state),
        ("File (map)", "Quit to DOS") => {
            if super::commands::dirty_guard(state, super::state::PendingAction::Quit) {
                #[cfg(not(target_arch = "wasm32"))]
                std::process::exit(0);
            }
        }
        ("WAD list", "Save as PWAD...") => super::commands::save_map_as(state),
        ("WAD list", "List WADs") => state.dialog = Some(Dialog::WadList),
        ("WAD list", "Save selection as prefab") => super::commands::save_selection_as_prefab(state),
        ("WAD list", "Load prefab") => super::commands::load_prefab_at_cursor(state),
        ("WAD list", "Add PWAD file") => {
            if super::commands::dirty_guard(state, super::state::PendingAction::OpenWad) {
                open_wad_picker(state, tx);
            }
        }
        ("Edit", "Next object") => super::commands::cycle_selection(state, 1),
        ("Edit", "Previous object") => super::commands::cycle_selection(state, -1),
        ("Edit", "Goto object") => {
            state.dialog = Some(Dialog::GotoObject { input: String::new() });
        }
        ("Edit", "Find objects") => {
            state.dialog = Some(Dialog::FindReplace {
                kind: super::state::FindKind::LineDefTexture,
                find: String::new(),
                replace: String::new(),
                replace_mode: false,
            });
        }
        ("Edit", "Tag line to sector") => super::commands::begin_tag_link(state),
        ("Map utilities", "Shift Map (X/Y/Z)") => {
            state.dialog = Some(Dialog::ShiftMap {
                dx: "0".into(),
                dy: "0".into(),
                dz: "0".into(),
            });
        }
        ("Map utilities", "Expand/reduce map") => {
            state.dialog = Some(Dialog::ExpandMap {
                sx: "1.0".into(),
                sy: "1.0".into(),
                sz: "1.0".into(),
            });
        }
        ("Map utilities", "Fix zero-length linedefs") => {
            super::commands::fix_zero_length_linedefs(state);
        }
        ("Map utilities", "Fix missing textures") => {
            super::commands::fix_missing_textures(state);
        }
        ("Map utilities", "Remove unused textures") => {
            super::commands::remove_unused_textures(state);
        }
        ("Map utilities", "Enhance map") => super::commands::enhance_map(state),
        ("Map utilities", "Light adjustment") => {
            state.dialog = Some(Dialog::LightAdjust {
                a: "100".into(),
                b: "0".into(),
            });
        }
        ("Sectors", "Join sectors") => {
            super::commands::join_sectors(state);
        }
        ("Sectors", "Merge sectors") => {
            super::commands::merge_sectors(state);
        }
        ("Sectors", "Gradient floors") => {
            super::commands::gradient_sector_field(state, super::commands::GradientField::Floor);
        }
        ("Sectors", "Gradient ceilings") => {
            super::commands::gradient_sector_field(state, super::commands::GradientField::Ceiling);
        }
        ("Sectors", "Gradient brightness") => {
            super::commands::gradient_sector_field(state, super::commands::GradientField::Brightness);
        }
        ("Sectors", "Grab style") => super::commands::grab_sector_style(state),
        ("Sectors", "Texture style") => super::commands::apply_sector_style_textures(state),
        ("Sectors", "Edit styles") => super::commands::apply_sector_style_all(state),
        ("Sectors", "Curve LineDef") => {
            state.dialog = Some(Dialog::CurveLineDef {
                vertices_per_line: "8".into(),
                curve_distance: "64".into(),
                delta_angle: "180".into(),
            });
        }
        ("Display", "Export picture") => {
            state.dialog = Some(Dialog::ExportPicture {
                width: "1024".into(),
                height: "1024".into(),
                with_grid: false,
                with_vertices: true,
                with_things: true,
                with_thing_bboxes: false,
            });
        }
        ("Display", "Things filter") => {
            state.dialog = Some(Dialog::ThingsFilter {
                categories: state.thing_filter,
            });
        }
        ("Display", "Thing bounding boxes") => {
            state.things_bbox_visible = !state.things_bbox_visible;
        }
        ("Sectors", "Polygon") => {
            state.dialog = Some(Dialog::Polygon {
                sides: "8".into(),
                radius: "128".into(),
            });
        }
        ("Automatic", "Door") => {
            state.dialog = Some(Dialog::Door {
                key: super::state::DoorKey::Keyless,
                fast: false,
            });
        }
        ("Automatic", "Lift") => {
            state.dialog = Some(Dialog::Lift {
                repeatable: true,
                fast: false,
            });
        }
        ("Automatic", "Teleporter") => {
            state.dialog = Some(Dialog::Teleporter);
        }
        ("Automatic", "Stairs") => {
            state.dialog = Some(Dialog::Stairs {
                steps: "8".into(),
                rise: "16".into(),
                depth: "32".into(),
                width: "128".into(),
                direction: super::state::StairsDirection::North,
                top_texture: "FLOOR4_8".into(),
                side_texture: "STEP1".into(),
            });
        }
        ("Edit", "Copy") => super::commands::copy_selection(state),
        ("Edit", "Paste") => super::commands::paste_clipboard(state),
        ("Edit", "Flip selection horizontal") => super::commands::flip_selection_axis(state, true),
        ("Edit", "Flip selection vertical") => super::commands::flip_selection_axis(state, false),
        ("Edit", "Rotate selection") => {
            state.dialog = Some(Dialog::RotateSelection { degrees: "90".into() });
        }
        ("Edit", "Scale selection") => {
            state.dialog = Some(Dialog::ScaleSelection { percent: "100".into() });
        }
        ("Edit", "Add/split") => super::commands::add_at_cursor(state),
        ("Edit", "Line-draw mode") => super::commands::toggle_line_draw(state),
        ("Edit", "Delete/merge") => super::commands::delete_selected(state),
        ("Edit", "Undo from last save") => super::commands::undo_to_baseline(state),
        ("Display", "Grid on/off") => {
            state.grid_visible = !state.grid_visible;
            state.status_message = Some(format!(
                "Grid: {}",
                if state.grid_visible { "on" } else { "off" }
            ));
        }
        ("Display", "Grid intensity") => {
            state.grid_intensity = state.grid_intensity.cycle();
            state.grid_visible = true;
            state.status_message = Some(format!(
                "Grid intensity: {}",
                state.grid_intensity.label()
            ));
        }
        ("Display", "Origin on/off") => state.origin_visible = !state.origin_visible,
        ("Display", "Center map") => super::commands::center_map(state),
        ("Display", "Refresh display") => state.status_message = None,
        ("Display", "Viewer") => {
            state.viewer_open = !state.viewer_open;
        }
        ("Check", "Quick check") => {
            super::commands::run_checks(state, super::checks::CheckSet::Quick);
        }
        ("Check", "Check all") => {
            super::commands::run_checks(state, super::checks::CheckSet::All);
        }
        ("Check", "Error list") => super::commands::reopen_error_list(state),
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

#[cfg(not(target_arch = "wasm32"))]
fn open_wad_picker(state: &mut EditorState, _tx: &std::sync::mpsc::Sender<crate::app::AsyncCommand>) {
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
            state.is_dirty = false;
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
                            state.undo_baseline = state.map.clone();
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

#[cfg(target_arch = "wasm32")]
fn open_wad_picker(_state: &mut EditorState, tx: &std::sync::mpsc::Sender<crate::app::AsyncCommand>) {
    let tx = tx.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let file = rfd::AsyncFileDialog::new()
            .add_filter("WAD files", &["wad", "WAD"])
            .pick_file()
            .await;
        if let Some(file) = file {
            let bytes = file.read().await;
            let name = file.file_name();
            let _ = tx.send(crate::app::AsyncCommand::LoadWad { name, bytes });
        }
    });
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
