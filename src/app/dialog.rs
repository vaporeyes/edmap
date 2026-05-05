// ABOUTME: Modal dialog renderer — VGA-styled centered window with inverse title bar.
// ABOUTME: One match arm per Dialog variant; result is dispatched as commands when OK clicked.

use eframe::egui::{self, Align2, Color32, RichText};

use super::commands;
use super::state::{Dialog, EditorState, PendingAction};
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
                            // TextEdit fills with visuals.extreme_bg_color; force it
                            // white so black text stays readable on dark themes.
                            ui.visuals_mut().extreme_bg_color = theme::VGA_WHITE;
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
        egui::FontId::new(13.0, egui::FontFamily::Monospace),
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
        Dialog::SaveWarning { .. } => "Save warning",
        Dialog::ErrorList { .. } => "Error List",
        Dialog::Picker { kind, .. } => match kind {
            super::state::PickerKind::ThingType => "Choose Thing Type",
            super::state::PickerKind::LineDefAction => "Choose LineDef Action",
        },
        Dialog::RotateSelection { .. } => "Rotate Selection",
        Dialog::ScaleSelection { .. } => "Scale Selection",
        Dialog::FindReplace { .. } => "Find / Replace",
        Dialog::Preferences => "Preferences",
        Dialog::Polygon { .. } => "Polygon",
        Dialog::Door { .. } => "Door",
        Dialog::CurveLineDef { .. } => "Curve LineDef",
        Dialog::ThingsFilter { .. } => "Things filter",
        Dialog::Lift { .. } => "Lift",
        Dialog::Teleporter => "Teleporter",
        Dialog::Stairs { .. } => "Stairs",
        Dialog::ShiftMap { .. } => "Map shift",
        Dialog::ExpandMap { .. } => "Map expand/reduce",
        Dialog::LightAdjust { .. } => "Map light adjustment",
        Dialog::EditVertex { .. } => "Edit Vertex",
        Dialog::EditLineDef { .. } => "Edit LineDef",
        Dialog::EditSector { .. } => "Edit Sector",
        Dialog::EditThing { .. } => "Edit Thing",
        Dialog::TestMapSettings { .. } => "Test Map Settings",
        Dialog::ExportPicture { .. } => "Export Picture",
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
        Dialog::SaveWarning { pending } => save_warning_body(ui, state, pending),
        Dialog::ErrorList { results, cursor } => error_list_body(ui, state, results, cursor),
        Dialog::Picker { kind, expanded } => picker_body(ui, state, kind, expanded),
        Dialog::RotateSelection { degrees } => rotate_selection_body(ui, state, degrees),
        Dialog::ScaleSelection { percent } => scale_selection_body(ui, state, percent),
        Dialog::FindReplace { kind, find, replace, replace_mode } =>
            find_replace_body(ui, state, kind, find, replace, replace_mode),
        Dialog::Preferences => preferences_body(ui, state),
        Dialog::Polygon { sides, radius } => polygon_body(ui, state, sides, radius),
        Dialog::Door { key, fast } => door_body(ui, state, key, fast),
        Dialog::CurveLineDef { vertices_per_line, curve_distance, delta_angle } =>
            curve_linedef_body(ui, state, vertices_per_line, curve_distance, delta_angle),
        Dialog::ThingsFilter { categories } => things_filter_body(ui, state, categories),
        Dialog::Lift { repeatable, fast } => lift_body(ui, state, repeatable, fast),
        Dialog::Teleporter => teleporter_body(ui, state),
        Dialog::ShiftMap { dx, dy, dz } => shift_map_body(ui, state, dx, dy, dz),
        Dialog::ExpandMap { sx, sy, sz } => expand_map_body(ui, state, sx, sy, sz),
        Dialog::LightAdjust { a, b } => light_adjust_body(ui, state, a, b),
        Dialog::EditVertex { idx, x, y } => edit_vertex_body(ui, state, idx, x, y),
        Dialog::EditLineDef {
            idx, flags, special, tag, front_sidedef, back_sidedef,
        } => edit_linedef_body(ui, state, idx, flags, special, tag, front_sidedef, back_sidedef),
        Dialog::EditSector {
            idx, floor_height, ceiling_height, light, sector_type, tag, floor_texture, ceiling_texture,
        } => edit_sector_body(ui, state, idx, floor_height, ceiling_height, light, sector_type, tag, floor_texture, ceiling_texture),
        Dialog::EditThing { idx, x, y, angle, thing_type, flags } => {
            edit_thing_body(ui, state, idx, x, y, angle, thing_type, flags)
        }
        Dialog::Stairs {
            steps,
            rise,
            depth,
            width,
            direction,
            top_texture,
            side_texture,
        } => stairs_body(
            ui, state, steps, rise, depth, width, direction, top_texture, side_texture,
        ),
        Dialog::TestMapSettings { exe, args } => test_map_settings_body(ui, state, exe, args),
        Dialog::ExportPicture { width, height, with_grid, with_vertices, with_things, with_thing_bboxes } =>
            export_picture_body(ui, state, width, height, with_grid, with_vertices, with_things, with_thing_bboxes),
    }
}

fn polygon_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    sides: String,
    radius: String,
) -> bool {
    let mut sides = sides;
    let mut radius = radius;
    ui.colored_label(theme::MENU_FG, "Place center at cursor.");
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Number of sides:");
        ui.add(text_box(&mut sides, 6));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Radius:");
        ui.add(text_box(&mut radius, 6));
    });
    ui.add_space(6.0);

    let close = ui
        .horizontal(|ui| {
            let ok = button(ui, "OK").clicked();
            let cancel = button(ui, "cancel").clicked();
            if ok {
                let s: usize = sides.trim().parse().unwrap_or(8);
                let r: f32 = radius.trim().parse().unwrap_or(128.0);
                commands::create_polygon(state, s, r);
                return true;
            }
            cancel
        })
        .inner;

    if !close {
        state.dialog = Some(Dialog::Polygon { sides, radius });
    }
    close
}

fn door_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    key: super::state::DoorKey,
    fast: bool,
) -> bool {
    let mut key = key;
    let mut fast = fast;

    ui.colored_label(theme::MENU_FG, "Turns the selected sector into a door.");
    ui.colored_label(theme::VGA_DARK_GRAY, "(must be in Sector mode with one selected)");
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Key:");
        for k in [
            super::state::DoorKey::Keyless,
            super::state::DoorKey::Blue,
            super::state::DoorKey::Yellow,
            super::state::DoorKey::Red,
        ] {
            let active = key == k;
            let (label_color, deco) = match k {
                super::state::DoorKey::Keyless => (theme::MENU_FG, false),
                super::state::DoorKey::Blue => (theme::VGA_BRIGHT_BLUE, true),
                super::state::DoorKey::Yellow => (theme::VGA_YELLOW, true),
                super::state::DoorKey::Red => (theme::VGA_BRIGHT_RED, true),
            };
            let _ = deco;
            let label = if active {
                RichText::new(k.label()).color(label_color).underline()
            } else {
                RichText::new(k.label()).color(label_color)
            };
            if ui.add(egui::Label::new(label).sense(egui::Sense::click())).clicked() {
                key = k;
            }
        }
    });

    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Speed:");
        for (label_text, want_fast) in [("Normal", false), ("Fast", true)] {
            let active = fast == want_fast;
            let label = if active {
                RichText::new(label_text).color(theme::VGA_YELLOW).underline()
            } else {
                RichText::new(label_text).color(theme::VGA_BRIGHT_CYAN)
            };
            if ui.add(egui::Label::new(label).sense(egui::Sense::click())).clicked() {
                fast = want_fast;
            }
        }
    });

    ui.add_space(6.0);

    let close = ui
        .horizontal(|ui| {
            let ok = button(ui, "OK").clicked();
            let cancel = button(ui, "cancel").clicked();
            if ok {
                commands::create_door(state, key, fast);
                return true;
            }
            cancel
        })
        .inner;

    if !close {
        state.dialog = Some(Dialog::Door { key, fast });
    }
    close
}

fn stairs_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    steps: String,
    rise: String,
    depth: String,
    width: String,
    direction: super::state::StairsDirection,
    top_texture: String,
    side_texture: String,
) -> bool {
    let mut steps = steps;
    let mut rise = rise;
    let mut depth = depth;
    let mut width = width;
    let mut direction = direction;
    let mut top_texture = top_texture;
    let mut side_texture = side_texture;

    ui.colored_label(theme::MENU_FG, "First step starts at cursor.");
    ui.add_space(4.0);

    macro_rules! row {
        ($label:expr, $buf:ident, $w:expr) => {
            ui.horizontal(|ui| {
                ui.colored_label(theme::MENU_FG, $label);
                ui.add(text_box(&mut $buf, $w));
            });
        };
    }
    row!("Number of steps:", steps, 4);
    row!("Step rise:      ", rise, 4);
    row!("Step depth:     ", depth, 4);
    row!("Step width:     ", width, 4);
    row!("Top texture:    ", top_texture, 8);
    row!("Side texture:   ", side_texture, 8);

    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Direction:");
        for d in [
            super::state::StairsDirection::North,
            super::state::StairsDirection::East,
            super::state::StairsDirection::South,
            super::state::StairsDirection::West,
        ] {
            let active = direction == d;
            let label = if active {
                RichText::new(d.label()).color(theme::VGA_YELLOW).underline()
            } else {
                RichText::new(d.label()).color(theme::VGA_BRIGHT_CYAN)
            };
            if ui.add(egui::Label::new(label).sense(egui::Sense::click())).clicked() {
                direction = d;
            }
        }
    });
    ui.add_space(6.0);

    let close = ui
        .horizontal(|ui| {
            let ok = button(ui, "OK").clicked();
            let cancel = button(ui, "cancel").clicked();
            if ok {
                let s: usize = steps.trim().parse().unwrap_or(8);
                let r: i32 = rise.trim().parse().unwrap_or(16);
                let d: i32 = depth.trim().parse().unwrap_or(32);
                let w: i32 = width.trim().parse().unwrap_or(128);
                commands::create_stairs(state, s, r, d, w, direction, &top_texture, &side_texture);
                return true;
            }
            cancel
        })
        .inner;

    if !close {
        state.dialog = Some(Dialog::Stairs {
            steps,
            rise,
            depth,
            width,
            direction,
            top_texture,
            side_texture,
        });
    }
    close
}

fn error_list_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    results: Vec<super::checks::CheckResult>,
    cursor: usize,
) -> bool {
    use super::checks::Severity;
    if results.is_empty() {
        ui.colored_label(theme::VGA_BRIGHT_GREEN, "No errors detected.");
        ui.add_space(6.0);
        return ok_button(ui, "OK");
    }
    let cursor = cursor.min(results.len() - 1);
    let cur = &results[cursor];

    ui.colored_label(theme::MENU_FG, format!("{} of {}", cursor + 1, results.len()));
    let label_color = match cur.severity {
        Severity::Error => theme::VGA_BRIGHT_RED,
        Severity::Warning => theme::VGA_YELLOW,
    };
    ui.colored_label(label_color, &cur.label);
    ui.add_space(2.0);
    ui.colored_label(theme::VGA_WHITE, &cur.message);
    if let Some((mode, idx)) = cur.at {
        ui.add_space(2.0);
        ui.colored_label(
            theme::VGA_DARK_GRAY,
            format!("at {}: {idx}", mode_label(mode)),
        );
    }
    ui.add_space(6.0);

    let mut new_cursor = cursor;
    let close = ui
        .horizontal(|ui| {
            let prev = button(ui, "Previous").clicked();
            let next = button(ui, "Next").clicked();
            let goto = button(ui, "Goto").clicked();
            let cls = button(ui, "Close").clicked();
            if prev && cursor > 0 {
                new_cursor = cursor - 1;
            }
            if next && cursor + 1 < results.len() {
                new_cursor = cursor + 1;
            }
            if goto {
                if let Some((mode, idx)) = cur.at {
                    state.mode = mode;
                    state.selection.clear();
                    state.selection.push(idx);
                    super::commands::focus_on_selection(state);
                }
            }
            cls
        })
        .inner;

    if !close {
        state.dialog = Some(Dialog::ErrorList { results, cursor: new_cursor });
    }
    close
}

fn mode_label(mode: super::SelectionMode) -> &'static str {
    match mode {
        super::SelectionMode::Vertex => "Vertex",
        super::SelectionMode::LineDef => "LineDef",
        super::SelectionMode::Sector => "Sector",
        super::SelectionMode::Thing => "Thing",
    }
}

fn save_warning_body(ui: &mut egui::Ui, state: &mut EditorState, pending: PendingAction) -> bool {
    ui.colored_label(theme::VGA_BRIGHT_RED, "This map has been modified.");
    ui.colored_label(theme::MENU_FG, "Do you want to save?");
    ui.add_space(6.0);
    let close = ui
        .horizontal(|ui| {
            let yes = button(ui, "Yes").clicked();
            let no = button(ui, "No").clicked();
            let cancel = button(ui, "cancel").clicked();
            if yes {
                commands::save_map(state);
                // If save itself opened a Notice (e.g. error), state.dialog is
                // now that Notice — don't run the pending action.
                if state.dialog.is_none() && !state.is_dirty {
                    commands::run_pending(state, &pending);
                }
                return true;
            }
            if no {
                commands::run_pending(state, &pending);
                return true;
            }
            cancel
        })
        .inner;

    if !close {
        state.dialog = Some(Dialog::SaveWarning { pending });
    }
    close
}

fn about_body(ui: &mut egui::Ui) -> bool {
    ui.colored_label(theme::VGA_WHITE, "EdMap v2.0.0");
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

    const COLS: usize = 3;
    const CELL_W: f32 = 88.0;
    const CELL_H: f32 = 22.0;
    const CELL_GAP: f32 = 4.0;
    const PAD: f32 = 10.0;

    let dialog_width = COLS as f32 * CELL_W + (COLS as f32 - 1.0) * CELL_GAP + PAD * 2.0;

    let mut selected = selected.min(maps.len() - 1);
    let mut activated: Option<usize> = None;
    let mut hovered: Option<usize> = None;
    let wad_path_label = state
        .wad_path
        .as_ref()
        .map(|p| {
            let s = p.to_string_lossy();
            // Mimic original: keep last two path components for context.
            let parts: Vec<&str> = s.rsplit(['/', '\\']).take(2).collect();
            if parts.len() == 2 {
                format!("..\\{}\\{}", parts[1], parts[0])
            } else {
                s.into_owned()
            }
        })
        .unwrap_or_default();

    egui::Frame::none()
        .fill(theme::SIDEBAR_BG)
        .inner_margin(egui::Margin::same(PAD))
        .show(ui, |ui| {
            ui.set_min_width(dialog_width - 4.0);
            ui.spacing_mut().item_spacing = egui::vec2(CELL_GAP, CELL_GAP);

            ui.label(
                RichText::new("Select a map:")
                    .color(theme::VGA_WHITE)
                    .monospace()
                    .size(14.0),
            );
            ui.add_space(4.0);

            for chunk_start in (0..maps.len()).step_by(COLS) {
                ui.horizontal(|ui| {
                    for col in 0..COLS {
                        let i = chunk_start + col;
                        if i >= maps.len() {
                            ui.allocate_space(egui::vec2(CELL_W, CELL_H));
                            continue;
                        }
                        let is_active = i == selected;
                        let (clicked, is_hovered) =
                            map_cell(ui, &maps[i], CELL_W, CELL_H, is_active);
                        if is_hovered {
                            hovered = Some(i);
                        }
                        if clicked {
                            selected = i;
                            activated = Some(i);
                        }
                    }
                });
            }

            ui.add_space(8.0);
            let display_idx = hovered.unwrap_or(selected);
            let lump = &maps[display_idx];
            let title = super::map_titles::title_for(lump).unwrap_or(lump);
            ui.label(
                RichText::new(title)
                    .color(theme::VGA_WHITE)
                    .monospace()
                    .size(14.0)
                    .strong(),
            );
            if !wad_path_label.is_empty() {
                ui.label(
                    RichText::new(&wad_path_label)
                        .color(theme::VGA_WHITE)
                        .monospace(),
                );
            }
        });

    if let Some(i) = activated {
        load_selected_map(state, maps[i].clone());
        return true;
    }

    ui.add_space(6.0);

    let close = ui
        .horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                button(ui, "cancel").clicked()
            })
            .inner
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
            state.is_dirty = false;
            state.undo_baseline = state.map.clone();
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

/// Beveled gray cell with blue text - mimics the original EdMap map grid buttons.
/// Returns (clicked, hovered).
fn map_cell(ui: &mut egui::Ui, label: &str, w: f32, h: f32, selected: bool) -> (bool, bool) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    let painter = ui.painter_at(rect);
    let hovered = resp.hovered();
    let pressed = selected || resp.is_pointer_button_down_on() || hovered;
    painter.rect_filled(rect, 0.0, theme::MENU_BG);
    theme::draw_bevel(&painter, rect, pressed);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        egui::FontId::new(13.0, egui::FontFamily::Monospace),
        theme::VGA_BLUE,
    );
    (resp.clicked(), hovered)
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
        egui::FontId::new(13.0, egui::FontFamily::Monospace),
        fg,
    );
    resp
}

fn text_box<'a>(buf: &'a mut String, char_width: usize) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(buf)
        .desired_width(char_width as f32 * 9.0)
        .text_color(theme::VGA_BLACK)
        .font(egui::FontId::new(13.0, egui::FontFamily::Monospace))
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

// ---------------- Per-object property editor ----------------

fn parse_i16(s: &str) -> Option<i16> { s.trim().parse().ok() }
fn parse_u16(s: &str) -> Option<u16> { s.trim().parse().ok() }

fn edit_vertex_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    idx: usize,
    x: String,
    y: String,
) -> bool {
    let mut x = x;
    let mut y = y;
    ui.colored_label(theme::MENU_FG, format!("Vertex #{idx}"));
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "X:");
        ui.add(text_box(&mut x, 8));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Y:");
        ui.add(text_box(&mut y, 8));
    });
    ui.add_space(6.0);

    let close = ui.horizontal(|ui| {
        let ok = button(ui, "OK").clicked();
        let cancel = button(ui, "cancel").clicked();
        if ok {
            if let Some(map) = state.map.as_mut() {
                if let Some(v) = map.vertices.get_mut(idx) {
                    if let Some(nx) = parse_i16(&x) { v.x = nx; }
                    if let Some(ny) = parse_i16(&y) { v.y = ny; }
                    state.is_dirty = true;
                }
            }
            return true;
        }
        cancel
    }).inner;
    if !close {
        state.dialog = Some(Dialog::EditVertex { idx, x, y });
    }
    close
}

fn edit_linedef_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    idx: usize,
    flags: String,
    special: String,
    tag: String,
    front_sidedef: String,
    back_sidedef: String,
) -> bool {
    let mut flags = flags;
    let mut special = special;
    let mut tag = tag;
    let mut front_sidedef = front_sidedef;
    let mut back_sidedef = back_sidedef;

    ui.colored_label(theme::MENU_FG, format!("LineDef #{idx}"));
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Flags (bitmask):");
        ui.add(text_box(&mut flags, 6));
    });
    let mut pick_special = false;
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Special type:  ");
        ui.add(text_box(&mut special, 6));
        if button(ui, "Pick…").clicked() {
            pick_special = true;
        }
    });
    if let Ok(code) = special.trim().parse::<u16>() {
        ui.colored_label(
            theme::VGA_DARK_GRAY,
            super::picker_data::label_for(super::picker_data::LINEDEF_ACTIONS, code),
        );
    }
    if pick_special {
        let stashed = Dialog::EditLineDef {
            idx, flags, special, tag, front_sidedef, back_sidedef,
        };
        state.dialog_pending = Some(stashed);
        state.dialog = Some(Dialog::Picker {
            kind: super::state::PickerKind::LineDefAction,
            expanded: 1, // default open: Doors
        });
        return true;
    }
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Sector tag:    ");
        ui.add(text_box(&mut tag, 6));
        if button(ui, "Next Unused").clicked() {
            tag = commands::next_unused_tag_pub(state).to_string();
        }
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Front sidedef: ");
        ui.add(text_box(&mut front_sidedef, 8));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Back sidedef:  ");
        ui.add(text_box(&mut back_sidedef, 8));
    });
    ui.colored_label(theme::VGA_DARK_GRAY, "(use 65535 for no back sidedef)");
    ui.add_space(6.0);

    let close = ui.horizontal(|ui| {
        let ok = button(ui, "OK").clicked();
        let cancel = button(ui, "cancel").clicked();
        if ok {
            if let Some(map) = state.map.as_mut() {
                if let Some(ld) = map.linedefs.get_mut(idx) {
                    if let Some(v) = parse_u16(&flags) { ld.flags = v; }
                    if let Some(v) = parse_u16(&special) { ld.special_type = v; }
                    if let Some(v) = parse_u16(&tag) { ld.sector_tag = v; }
                    if let Some(v) = parse_u16(&front_sidedef) { ld.front_sidedef = v; }
                    if let Some(v) = parse_u16(&back_sidedef) { ld.back_sidedef = v; }
                    state.is_dirty = true;
                }
            }
            return true;
        }
        cancel
    }).inner;
    if !close {
        state.dialog = Some(Dialog::EditLineDef {
            idx, flags, special, tag, front_sidedef, back_sidedef,
        });
    }
    close
}

#[allow(clippy::too_many_arguments)]
fn edit_sector_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    idx: usize,
    floor_height: String,
    ceiling_height: String,
    light: String,
    sector_type: String,
    tag: String,
    floor_texture: String,
    ceiling_texture: String,
) -> bool {
    let mut floor_height = floor_height;
    let mut ceiling_height = ceiling_height;
    let mut light = light;
    let mut sector_type = sector_type;
    let mut tag = tag;
    let mut floor_texture = floor_texture;
    let mut ceiling_texture = ceiling_texture;

    ui.colored_label(theme::MENU_FG, format!("Sector #{idx}"));
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Floor height:  ");
        ui.add(text_box(&mut floor_height, 8));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Ceiling height:");
        ui.add(text_box(&mut ceiling_height, 8));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Light (0-255): ");
        ui.add(text_box(&mut light, 6));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Sector type:   ");
        ui.add(text_box(&mut sector_type, 6));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Tag:           ");
        ui.add(text_box(&mut tag, 6));
        if button(ui, "Next Unused").clicked() {
            tag = commands::next_unused_tag_pub(state).to_string();
        }
    });
    let mut pick_floor = false;
    let mut pick_ceiling = false;
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Floor texture:  ");
        ui.add(text_box(&mut floor_texture, 9));
        if button(ui, "Pick").clicked() {
            pick_floor = true;
        }
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Ceiling texture:");
        ui.add(text_box(&mut ceiling_texture, 9));
        if button(ui, "Pick").clicked() {
            pick_ceiling = true;
        }
    });
    ui.add_space(6.0);

    if pick_floor || pick_ceiling {
        // Stash this dialog with current edits, open viewer in pick mode.
        let stashed = Dialog::EditSector {
            idx, floor_height, ceiling_height, light, sector_type, tag,
            floor_texture, ceiling_texture,
        };
        let target = if pick_floor {
            super::state::PickTarget::SectorFloor
        } else {
            super::state::PickTarget::SectorCeiling
        };
        state.dialog_pending = Some(stashed);
        state.viewer_pick = Some(target);
        state.viewer_category = target.default_category();
        state.viewer_open = true;
        return true; // close current dialog rendering this frame
    }

    let close = ui.horizontal(|ui| {
        let ok = button(ui, "OK").clicked();
        let cancel = button(ui, "cancel").clicked();
        if ok {
            if let Some(map) = state.map.as_mut() {
                if let Some(s) = map.sectors.get_mut(idx) {
                    if let Some(v) = parse_i16(&floor_height) { s.floor_height = v; }
                    if let Some(v) = parse_i16(&ceiling_height) { s.ceiling_height = v; }
                    if let Some(v) = parse_i16(&light) { s.light_level = v.clamp(0, 255); }
                    if let Some(v) = parse_u16(&sector_type) { s.sector_type = v; }
                    if let Some(v) = parse_u16(&tag) { s.tag = v; }
                    s.floor_texture = clamp_tex_name(&floor_texture);
                    s.ceiling_texture = clamp_tex_name(&ceiling_texture);
                    state.is_dirty = true;
                }
            }
            return true;
        }
        cancel
    }).inner;
    if !close {
        state.dialog = Some(Dialog::EditSector {
            idx, floor_height, ceiling_height, light, sector_type, tag,
            floor_texture, ceiling_texture,
        });
    }
    close
}

fn edit_thing_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    idx: usize,
    x: String,
    y: String,
    angle: String,
    thing_type: String,
    flags: String,
) -> bool {
    let mut x = x;
    let mut y = y;
    let mut angle = angle;
    let mut thing_type = thing_type;
    let mut flags = flags;

    ui.colored_label(theme::MENU_FG, format!("Thing #{idx}"));
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "X:    ");
        ui.add(text_box(&mut x, 8));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Y:    ");
        ui.add(text_box(&mut y, 8));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Angle:");
        ui.add(text_box(&mut angle, 6));
    });
    let mut pick_type = false;
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Type: ");
        ui.add(text_box(&mut thing_type, 6));
        if button(ui, "Pick…").clicked() {
            pick_type = true;
        }
    });
    if let Ok(code) = thing_type.trim().parse::<u16>() {
        ui.colored_label(
            theme::VGA_DARK_GRAY,
            super::picker_data::label_for(super::picker_data::THING_TYPES, code),
        );
    }
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Flags:");
        ui.add(text_box(&mut flags, 6));
    });
    ui.add_space(6.0);

    if pick_type {
        let stashed = Dialog::EditThing { idx, x, y, angle, thing_type, flags };
        state.dialog_pending = Some(stashed);
        state.dialog = Some(Dialog::Picker {
            kind: super::state::PickerKind::ThingType,
            expanded: 2, // default open category: Monsters
        });
        return true;
    }

    let close = ui.horizontal(|ui| {
        let ok = button(ui, "OK").clicked();
        let cancel = button(ui, "cancel").clicked();
        if ok {
            if let Some(map) = state.map.as_mut() {
                if let Some(t) = map.things.get_mut(idx) {
                    if let Some(v) = parse_i16(&x) { t.x = v; }
                    if let Some(v) = parse_i16(&y) { t.y = v; }
                    if let Some(v) = parse_i16(&angle) { t.angle = v; }
                    if let Some(v) = parse_u16(&thing_type) { t.thing_type = v; }
                    if let Some(v) = parse_u16(&flags) { t.flags = v; }
                    state.is_dirty = true;
                }
            }
            return true;
        }
        cancel
    }).inner;
    if !close {
        state.dialog = Some(Dialog::EditThing { idx, x, y, angle, thing_type, flags });
    }
    close
}

/// Clip texture name to 8 ASCII bytes, uppercase. Matches DOOM lump-name limits.
fn clamp_tex_name(s: &str) -> String {
    let bytes = s.as_bytes();
    let n = bytes.len().min(8);
    let mut out = String::with_capacity(n);
    for &b in &bytes[..n] {
        if b.is_ascii() {
            out.push(b.to_ascii_uppercase() as char);
        }
    }
    if out.is_empty() { "-".into() } else { out }
}

// ---------------- Map utilities (Shift / Expand / Light) ----------------

fn shift_map_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    dx: String,
    dy: String,
    dz: String,
) -> bool {
    let mut dx = dx;
    let mut dy = dy;
    let mut dz = dz;
    ui.colored_label(theme::MENU_FG, "Translate every vertex + thing by (dx, dy)");
    ui.colored_label(theme::MENU_FG, "and every sector floor + ceiling by dz.");
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "X (east/west):  ");
        ui.add(text_box(&mut dx, 8));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Y (north/south):");
        ui.add(text_box(&mut dy, 8));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Z (up/down):    ");
        ui.add(text_box(&mut dz, 8));
    });
    ui.add_space(6.0);
    let close = ui
        .horizontal(|ui| {
            let ok = button(ui, "OK").clicked();
            let cancel = button(ui, "cancel").clicked();
            if ok {
                let dx_v: i32 = dx.trim().parse().unwrap_or(0);
                let dy_v: i32 = dy.trim().parse().unwrap_or(0);
                let dz_v: i32 = dz.trim().parse().unwrap_or(0);
                commands::shift_map(state, dx_v, dy_v, dz_v);
                return true;
            }
            cancel
        })
        .inner;
    if !close {
        state.dialog = Some(Dialog::ShiftMap { dx, dy, dz });
    }
    close
}

fn expand_map_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    sx: String,
    sy: String,
    sz: String,
) -> bool {
    let mut sx = sx;
    let mut sy = sy;
    let mut sz = sz;
    ui.colored_label(theme::MENU_FG, "Scale every vertex around the map's center.");
    ui.colored_label(theme::MENU_FG, "Heights scale around 0. Factors must be > 0.");
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "X factor:");
        ui.add(text_box(&mut sx, 6));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Y factor:");
        ui.add(text_box(&mut sy, 6));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Z factor:");
        ui.add(text_box(&mut sz, 6));
    });
    ui.add_space(6.0);
    let close = ui
        .horizontal(|ui| {
            let ok = button(ui, "OK").clicked();
            let cancel = button(ui, "cancel").clicked();
            if ok {
                let sx_v: f32 = sx.trim().parse().unwrap_or(1.0);
                let sy_v: f32 = sy.trim().parse().unwrap_or(1.0);
                let sz_v: f32 = sz.trim().parse().unwrap_or(1.0);
                commands::expand_map(state, sx_v, sy_v, sz_v);
                return true;
            }
            cancel
        })
        .inner;
    if !close {
        state.dialog = Some(Dialog::ExpandMap { sx, sy, sz });
    }
    close
}

fn light_adjust_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    a: String,
    b: String,
) -> bool {
    let mut a = a;
    let mut b = b;
    ui.colored_label(theme::MENU_FG, "new_light = old_light × A/100 + B");
    ui.colored_label(theme::MENU_FG, "Result clamped to [0, 255].");
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "A (amplify, %): ");
        ui.add(text_box(&mut a, 5));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "B (brighten):   ");
        ui.add(text_box(&mut b, 5));
    });
    ui.add_space(6.0);
    let close = ui
        .horizontal(|ui| {
            let ok = button(ui, "OK").clicked();
            let cancel = button(ui, "cancel").clicked();
            if ok {
                let a_v: i32 = a.trim().parse().unwrap_or(100);
                let b_v: i32 = b.trim().parse().unwrap_or(0);
                commands::light_adjust(state, a_v, b_v);
                return true;
            }
            cancel
        })
        .inner;
    if !close {
        state.dialog = Some(Dialog::LightAdjust { a, b });
    }
    close
}

// ---------------- Lift / Teleporter dialog bodies ----------------

fn lift_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    repeatable: bool,
    fast: bool,
) -> bool {
    let mut repeatable = repeatable;
    let mut fast = fast;

    ui.colored_label(theme::MENU_FG, "Turns the selected sector into a lift.");
    ui.colored_label(theme::VGA_DARK_GRAY, "(must be in Sector mode with one selected)");
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Trigger:");
        for (label, want) in [("Once", false), ("Repeatable", true)] {
            let active = repeatable == want;
            let label = if active {
                RichText::new(label).color(theme::VGA_YELLOW).underline()
            } else {
                RichText::new(label).color(theme::VGA_BRIGHT_CYAN)
            };
            if ui.add(egui::Label::new(label).sense(egui::Sense::click())).clicked() {
                repeatable = want;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Speed:  ");
        for (label, want) in [("Normal", false), ("Fast", true)] {
            let active = fast == want;
            let label = if active {
                RichText::new(label).color(theme::VGA_YELLOW).underline()
            } else {
                RichText::new(label).color(theme::VGA_BRIGHT_CYAN)
            };
            if ui.add(egui::Label::new(label).sense(egui::Sense::click())).clicked() {
                fast = want;
            }
        }
    });
    ui.add_space(6.0);

    let close = ui
        .horizontal(|ui| {
            let ok = button(ui, "OK").clicked();
            let cancel = button(ui, "cancel").clicked();
            if ok {
                commands::create_lift(state, repeatable, fast);
                return true;
            }
            cancel
        })
        .inner;

    if !close {
        state.dialog = Some(Dialog::Lift { repeatable, fast });
    }
    close
}

fn teleporter_body(ui: &mut egui::Ui, state: &mut EditorState) -> bool {
    ui.colored_label(theme::MENU_FG, "Pairs the two selected sectors as a");
    ui.colored_label(theme::MENU_FG, "two-way teleporter.");
    ui.add_space(2.0);
    ui.colored_label(theme::VGA_DARK_GRAY, "(Sector mode, exactly two sectors selected)");
    ui.add_space(2.0);
    ui.colored_label(theme::VGA_DARK_GRAY, "Inserts a Thing-14 destination in each pad");
    ui.colored_label(theme::VGA_DARK_GRAY, "and assigns matching tags + WR teleport actions.");
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        let ok = button(ui, "OK").clicked();
        let cancel = button(ui, "cancel").clicked();
        if ok {
            commands::create_teleporter(state);
            return true;
        }
        cancel
    })
    .inner
}

// ---------------- Curve LineDef + Things Filter dialog bodies ----------------

fn curve_linedef_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    vertices_per_line: String,
    curve_distance: String,
    delta_angle: String,
) -> bool {
    let mut vertices_per_line = vertices_per_line;
    let mut curve_distance = curve_distance;
    let mut delta_angle = delta_angle;
    ui.colored_label(theme::MENU_FG, "Replace selected LineDef with a smooth arc.");
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Vertices per line:");
        ui.add(text_box(&mut vertices_per_line, 4));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Curve distance:   ");
        ui.add(text_box(&mut curve_distance, 6));
    });
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Delta angle (deg):");
        ui.add(text_box(&mut delta_angle, 5));
    });
    ui.colored_label(theme::VGA_DARK_GRAY, "(negative curve_distance flips direction)");
    ui.add_space(6.0);
    let close = ui.horizontal(|ui| {
        let ok = button(ui, "OK").clicked();
        let cancel = button(ui, "cancel").clicked();
        if ok {
            let n: usize = vertices_per_line.trim().parse().unwrap_or(8);
            let d: f32 = curve_distance.trim().parse().unwrap_or(64.0);
            commands::curve_linedef(state, n, d);
            return true;
        }
        cancel
    }).inner;
    if !close {
        state.dialog = Some(Dialog::CurveLineDef { vertices_per_line, curve_distance, delta_angle });
    }
    close
}

const THING_CATEGORY_LABELS: [&str; 11] = [
    "Player Starts",
    "Teleports",
    "Monsters",
    "Weapons",
    "Ammunition",
    "Health & Armor",
    "Powerups",
    "Keys",
    "Obstacles",
    "Light Sources",
    "Decoration",
];

fn things_filter_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    categories: [bool; 11],
) -> bool {
    let mut categories = categories;
    ui.colored_label(theme::MENU_FG, "Show only Things in checked categories:");
    ui.add_space(4.0);
    for (i, label) in THING_CATEGORY_LABELS.iter().enumerate() {
        ui.checkbox(&mut categories[i], *label);
    }
    ui.add_space(6.0);
    let close = ui.horizontal(|ui| {
        let ok = button(ui, "OK").clicked();
        let all = button(ui, "All").clicked();
        let none = button(ui, "None").clicked();
        let cancel = button(ui, "cancel").clicked();
        if all {
            categories = [true; 11];
        }
        if none {
            categories = [false; 11];
        }
        if ok {
            state.thing_filter = categories;
            return true;
        }
        cancel
    }).inner;
    if !close {
        state.dialog = Some(Dialog::ThingsFilter { categories });
    }
    close
}

// ---------------- Categorized picker (Thing types + LineDef actions) ----------------

fn picker_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    kind: super::state::PickerKind,
    expanded: usize,
) -> bool {
    use super::picker_data::{LINEDEF_ACTIONS, THING_TYPES};
    let table = match kind {
        super::state::PickerKind::ThingType => THING_TYPES,
        super::state::PickerKind::LineDefAction => LINEDEF_ACTIONS,
    };

    ui.colored_label(theme::MENU_FG, "Click a category to expand, then click an entry.");
    ui.add_space(4.0);

    let mut new_expanded = expanded;
    let mut picked: Option<u16> = None;

    egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
        for (i, cat) in table.iter().enumerate() {
            let is_expanded = i == expanded;
            let prefix = if is_expanded { "▼" } else { "▶" };
            let cat_label = format!("{prefix} {}", cat.label);
            if ui
                .add(
                    egui::Label::new(
                        RichText::new(&cat_label).color(theme::VGA_BRIGHT_CYAN).strong(),
                    )
                    .sense(egui::Sense::click()),
                )
                .clicked()
            {
                new_expanded = if is_expanded { usize::MAX } else { i };
            }
            if is_expanded {
                for entry in cat.entries {
                    let label = format!("    {} — {}", entry.code, entry.label);
                    if ui
                        .add(
                            egui::Label::new(
                                RichText::new(&label).color(theme::VGA_WHITE),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .clicked()
                    {
                        picked = Some(entry.code);
                    }
                }
            }
        }
    });
    ui.add_space(6.0);

    let close = ui.horizontal(|ui| {
        let cancel = button(ui, "cancel").clicked();
        cancel
    }).inner;

    if let Some(code) = picked {
        apply_picker_choice(state, kind, code);
        return true;
    }
    if !close {
        state.dialog = Some(Dialog::Picker { kind, expanded: new_expanded });
    } else {
        // cancel — restore stashed dialog if any
        if let Some(stashed) = state.dialog_pending.take() {
            state.dialog = Some(stashed);
            return false; // dialog reopens this frame; don't double-close
        }
    }
    close
}

fn apply_picker_choice(
    state: &mut EditorState,
    kind: super::state::PickerKind,
    code: u16,
) {
    let Some(mut stashed) = state.dialog_pending.take() else { return };
    match (kind, &mut stashed) {
        (super::state::PickerKind::ThingType, Dialog::EditThing { thing_type, .. }) => {
            *thing_type = code.to_string();
        }
        (super::state::PickerKind::LineDefAction, Dialog::EditLineDef { special, .. }) => {
            *special = code.to_string();
        }
        _ => {}
    }
    state.dialog = Some(stashed);
    state.status_message = Some(format!("Picked code {code}"));
}

fn rotate_selection_body(ui: &mut egui::Ui, state: &mut EditorState, degrees: String) -> bool {
    let mut degrees = degrees;
    ui.colored_label(theme::MENU_FG, "Rotate selected objects around their");
    ui.colored_label(theme::MENU_FG, "bounding-box center.");
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Degrees:");
        ui.add(text_box(&mut degrees, 6));
    });
    ui.add_space(6.0);
    let close = ui.horizontal(|ui| {
        let ok = button(ui, "OK").clicked();
        let cancel = button(ui, "cancel").clicked();
        if ok {
            let d: f32 = degrees.trim().parse().unwrap_or(0.0);
            commands::rotate_selection(state, d);
            return true;
        }
        cancel
    }).inner;
    if !close {
        state.dialog = Some(Dialog::RotateSelection { degrees });
    }
    close
}

fn scale_selection_body(ui: &mut egui::Ui, state: &mut EditorState, percent: String) -> bool {
    let mut percent = percent;
    ui.colored_label(theme::MENU_FG, "Scale selected objects by percentage,");
    ui.colored_label(theme::MENU_FG, "around their bounding-box center.");
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Percent (100=unchanged):");
        ui.add(text_box(&mut percent, 6));
    });
    ui.add_space(6.0);
    let close = ui.horizontal(|ui| {
        let ok = button(ui, "OK").clicked();
        let cancel = button(ui, "cancel").clicked();
        if ok {
            let p: f32 = percent.trim().parse().unwrap_or(100.0);
            commands::scale_selection(state, p / 100.0);
            return true;
        }
        cancel
    }).inner;
    if !close {
        state.dialog = Some(Dialog::ScaleSelection { percent });
    }
    close
}

fn find_replace_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    kind: super::state::FindKind,
    find: String,
    replace: String,
    replace_mode: bool,
) -> bool {
    use super::state::FindKind;
    let mut kind = kind;
    let mut find = find;
    let mut replace = replace;
    let mut replace_mode = replace_mode;

    ui.colored_label(theme::MENU_FG, "Search type:");
    egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
        for &k in FindKind::all() {
            let active = k == kind;
            let label = if active {
                RichText::new(format!("• {}", k.label())).color(theme::VGA_YELLOW)
            } else {
                RichText::new(format!("  {}", k.label())).color(theme::VGA_BRIGHT_CYAN)
            };
            if ui.add(egui::Label::new(label).sense(egui::Sense::click())).clicked() {
                kind = k;
            }
        }
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Find:   ");
        ui.add(text_box(&mut find, 16));
    });

    if kind.supports_replace() {
        ui.checkbox(&mut replace_mode, "Replace mode");
        if replace_mode {
            ui.horizontal(|ui| {
                ui.colored_label(theme::MENU_FG, "Replace:");
                ui.add(text_box(&mut replace, 16));
            });
        }
    } else {
        replace_mode = false;
    }
    ui.add_space(6.0);

    let close = ui.horizontal(|ui| {
        let do_find = button(ui, "Find").clicked();
        let do_replace = if replace_mode { button(ui, "Replace All").clicked() } else { false };
        let cancel = button(ui, "Close").clicked();
        if do_find {
            commands::find_objects(state, kind, &find);
            return true;
        }
        if do_replace {
            commands::replace_objects(state, kind, &find, &replace);
            return true;
        }
        cancel
    }).inner;

    if !close {
        state.dialog = Some(Dialog::FindReplace { kind, find, replace, replace_mode });
    }
    close
}

fn preferences_body(ui: &mut egui::Ui, state: &mut EditorState) -> bool {
    use crate::theme;
    ui.colored_label(theme::MENU_FG, "Override editor colors. Changes apply immediately.");
    ui.add_space(4.0);

    let overrides = &mut state.theme_overrides;

    fn color_row(
        ui: &mut egui::Ui,
        label: &str,
        slot: &mut Option<egui::Color32>,
        default: egui::Color32,
    ) {
        ui.horizontal(|ui| {
            ui.colored_label(theme::MENU_FG, label);
            let mut current = slot.unwrap_or(default);
            if ui.color_edit_button_srgba(&mut current).changed() {
                *slot = Some(current);
            }
            if ui.button("Reset").clicked() {
                *slot = None;
            }
        });
    }

    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
        color_row(ui, "Viewport background:", &mut overrides.viewport_bg, theme::VIEWPORT_BG);
        color_row(ui, "LineDef normal:     ", &mut overrides.linedef_normal, theme::LINEDEF_NORMAL);
        color_row(ui, "LineDef two-sided:  ", &mut overrides.linedef_two_sided, theme::LINEDEF_TWO_SIDED);
        color_row(ui, "LineDef selected:   ", &mut overrides.linedef_selected, theme::LINEDEF_SELECTED);
        color_row(ui, "Vertex dot:         ", &mut overrides.vertex_dot, theme::VERTEX_DOT);
        color_row(ui, "Vertex hover:       ", &mut overrides.vertex_hover, theme::VERTEX_HOVER);
        color_row(ui, "Thing marker:       ", &mut overrides.thing_mark, theme::THING_MARK);
        color_row(ui, "Grid dot:           ", &mut overrides.grid_dot, theme::GRID_DOT);

        ui.add_space(8.0);
        ui.colored_label(theme::VGA_WHITE, "3D view (Q to enter)");
        let v3d = &mut state.config.view3d;
        ui.checkbox(&mut v3d.invert_mouse_x, "Invert mouse X (horizontal)");
        ui.checkbox(&mut v3d.invert_mouse_y, "Invert mouse Y (vertical)");
        ui.horizontal(|ui| {
            ui.colored_label(theme::MENU_FG, "Mouse sensitivity:");
            ui.add(egui::Slider::new(&mut v3d.mouse_sensitivity, 0.1..=4.0).fixed_decimals(2));
        });
        ui.horizontal(|ui| {
            ui.colored_label(theme::MENU_FG, "Move speed:       ");
            ui.add(egui::Slider::new(&mut v3d.move_speed, 0.25..=4.0).fixed_decimals(2));
        });
        ui.horizontal(|ui| {
            ui.colored_label(theme::MENU_FG, "Sprint multiplier:");
            ui.add(egui::Slider::new(&mut v3d.sprint_multiplier, 1.0..=8.0).fixed_decimals(1));
        });
        ui.horizontal(|ui| {
            ui.colored_label(theme::MENU_FG, "Field of view:    ");
            ui.add(egui::Slider::new(&mut v3d.fov_degrees, 30.0..=130.0).suffix("°").fixed_decimals(0));
        });
        if button(ui, "Reset 3D defaults").clicked() {
            *v3d = super::config::View3DConfig::default();
        }
    });
    ui.add_space(6.0);
    ui.colored_label(theme::VGA_DARK_GRAY, "Hotkey customization not yet implemented.");
    ui.add_space(6.0);

    let close = ui.horizontal(|ui| {
        let ok = button(ui, "Close").clicked();
        let reset_all = button(ui, "Reset all").clicked();
        if reset_all {
            *overrides = super::state::ThemeOverrides::default();
        }
        ok
    }).inner;
    if close {
        if let Err(e) = state.config.save() {
            state.status_message = Some(format!("Save preferences: {e}"));
        }
    }
    close
}

fn test_map_settings_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    exe: String,
    args: String,
) -> bool {
    let mut exe = exe;
    let mut args = args;

    ui.colored_label(
        theme::MENU_FG,
        "Source-port to launch when you press Ctrl-F9 (Play map).",
    );
    ui.add_space(4.0);

    ui.colored_label(theme::MENU_FG, "Executable (gzdoom, dsda-doom, ...):");
    ui.horizontal(|ui| {
        ui.add(text_box(&mut exe, 36));
        #[cfg(not(target_arch = "wasm32"))]
        {
            if button(ui, "Browse").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    exe = path.to_string_lossy().to_string();
                }
            }
        }
    });
    ui.add_space(4.0);

    ui.colored_label(theme::MENU_FG, "Arguments (placeholders: %F %L %E %M):");
    ui.add(text_box(&mut args, 48));
    ui.add_space(2.0);
    ui.colored_label(theme::VGA_DARK_GRAY, "%F = temp PWAD path   %L = map name");
    ui.colored_label(theme::VGA_DARK_GRAY, "%E = episode #         %M = map #");
    ui.add_space(8.0);

    let close = ui.horizontal(|ui| {
        let ok = button(ui, "OK").clicked();
        let cancel = button(ui, "Cancel").clicked();
        if ok {
            state.config.test_map.exe = exe.trim().to_string();
            state.config.test_map.args = args.clone();
            if let Err(e) = state.config.save() {
                state.status_message = Some(format!("Could not save config: {e}"));
            } else {
                state.status_message = Some("Test map settings saved".into());
            }
            return true;
        }
        cancel
    }).inner;

    if !close {
        state.dialog = Some(Dialog::TestMapSettings { exe, args });
    }
    close
}

#[allow(clippy::too_many_arguments)]
fn export_picture_body(
    ui: &mut egui::Ui,
    state: &mut EditorState,
    width: String,
    height: String,
    with_grid: bool,
    with_vertices: bool,
    with_things: bool,
    with_thing_bboxes: bool,
) -> bool {
    let mut width = width;
    let mut height = height;
    let mut with_grid = with_grid;
    let mut with_vertices = with_vertices;
    let mut with_things = with_things;
    let mut with_thing_bboxes = with_thing_bboxes;

    ui.colored_label(theme::MENU_FG, "Render the map to a PNG (auto-fit, 5% padding).");
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.colored_label(theme::MENU_FG, "Width: ");
        ui.add(text_box(&mut width, 6));
        ui.add_space(8.0);
        ui.colored_label(theme::MENU_FG, "Height:");
        ui.add(text_box(&mut height, 6));
    });
    ui.add_space(4.0);

    let cb_text = |s: &str| RichText::new(s).color(theme::MENU_FG);
    ui.checkbox(&mut with_grid, cb_text("Grid dots"));
    ui.checkbox(&mut with_vertices, cb_text("Vertex dots"));
    ui.checkbox(&mut with_things, cb_text("Things (X markers)"));
    ui.add_enabled(
        with_things,
        egui::Checkbox::new(&mut with_thing_bboxes, cb_text("Thing bounding boxes")),
    );
    ui.add_space(8.0);

    let close = ui.horizontal(|ui| {
        let ok = button(ui, "Export...").clicked();
        let cancel = button(ui, "Cancel").clicked();
        if ok {
            let w: u32 = width.trim().parse().unwrap_or(1024).clamp(64, 16384);
            let h: u32 = height.trim().parse().unwrap_or(1024).clamp(64, 16384);
            let opts = super::export_picture::ExportOptions {
                width: w,
                height: h,
                with_grid,
                grid_size: state.grid_size.max(1),
                with_vertices,
                with_things,
                with_thing_bboxes,
            };
            commands::export_picture(state, opts);
            return true;
        }
        cancel
    }).inner;

    if !close {
        state.dialog = Some(Dialog::ExportPicture {
            width,
            height,
            with_grid,
            with_vertices,
            with_things,
            with_thing_bboxes,
        });
    }
    close
}
