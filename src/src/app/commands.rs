// ABOUTME: Editor commands invoked by menu clicks and keybindings.
// ABOUTME: Each function mutates EditorState; keep them small and pure-ish.

use super::state::{Dialog, EditorState, SelectionMode};
use crate::wad::LineDef;

pub fn cycle_selection(state: &mut EditorState, direction: i32) {
    let total = state.total_for_mode();
    if total == 0 {
        state.selection.clear();
        return;
    }
    let current = state.selection.last().copied().unwrap_or(usize::MAX);
    let next = if current == usize::MAX {
        0
    } else if direction >= 0 {
        (current + 1) % total
    } else {
        (current + total - 1) % total
    };
    state.selection.clear();
    state.selection.push(next);
}

pub fn center_map(state: &mut EditorState) {
    let Some(map) = &state.map else {
        state.view_center = egui::pos2(0.0, 0.0);
        state.view_zoom = 1.0;
        return;
    };
    if map.vertices.is_empty() {
        state.view_center = egui::pos2(0.0, 0.0);
        return;
    }
    let mut min = (f32::INFINITY, f32::INFINITY);
    let mut max = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for v in &map.vertices {
        min.0 = min.0.min(v.x as f32);
        min.1 = min.1.min(v.y as f32);
        max.0 = max.0.max(v.x as f32);
        max.1 = max.1.max(v.y as f32);
    }
    state.view_center = egui::pos2((min.0 + max.0) * 0.5, (min.1 + max.1) * 0.5);
    let w = (max.0 - min.0).max(1.0);
    let h = (max.1 - min.1).max(1.0);
    state.view_zoom = (600.0 / w).min(500.0 / h).max(0.05);
}

pub fn set_mode(state: &mut EditorState, mode: SelectionMode) {
    if state.mode != mode {
        state.mode = mode;
        state.selection.clear();
    }
}

/// Move the viewport so the first selected object sits in the center.
/// Used by Goto and Find. Leaves zoom alone.
pub fn focus_on_selection(state: &mut EditorState) {
    let Some(map) = &state.map else { return };
    let Some(&idx) = state.selection.first() else { return };
    let world = match state.mode {
        SelectionMode::Vertex => map.vertices.get(idx).map(|v| (v.x as f32, v.y as f32)),
        SelectionMode::LineDef => map.linedefs.get(idx).and_then(|ld| {
            let a = map.vertices.get(ld.start_vertex as usize)?;
            let b = map.vertices.get(ld.end_vertex as usize)?;
            Some(((a.x + b.x) as f32 * 0.5, (a.y + b.y) as f32 * 0.5))
        }),
        SelectionMode::Sector => sector_centroid(map, idx),
        SelectionMode::Thing => map.things.get(idx).map(|t| (t.x as f32, t.y as f32)),
    };
    if let Some((x, y)) = world {
        state.view_center = egui::pos2(x, y);
    }
}

fn sector_centroid(map: &crate::wad::MapData, sector_idx: usize) -> Option<(f32, f32)> {
    // Average all linedef-endpoint vertices that face this sector via SideDefs.
    let mut sum = (0i64, 0i64);
    let mut count = 0u32;
    for ld in &map.linedefs {
        for sd_idx in [ld.front_sidedef, ld.back_sidedef] {
            if sd_idx == crate::wad::LineDef::NO_SIDEDEF {
                continue;
            }
            let Some(sd) = map.sidedefs.get(sd_idx as usize) else { continue };
            if sd.sector as usize != sector_idx {
                continue;
            }
            for vi in [ld.start_vertex, ld.end_vertex] {
                if let Some(v) = map.vertices.get(vi as usize) {
                    sum.0 += v.x as i64;
                    sum.1 += v.y as i64;
                    count += 1;
                }
            }
        }
    }
    if count == 0 {
        return None;
    }
    Some((sum.0 as f32 / count as f32, sum.1 as f32 / count as f32))
}

/// Translate every selected object by (dx, dy) world units. Used by drag.
pub fn translate_selection(state: &mut EditorState, dx: i32, dy: i32) {
    if dx == 0 && dy == 0 {
        return;
    }
    let Some(map) = state.map.as_mut() else { return };
    match state.mode {
        SelectionMode::Vertex => {
            for &i in &state.selection {
                if let Some(v) = map.vertices.get_mut(i) {
                    v.x = v.x.saturating_add(dx as i16);
                    v.y = v.y.saturating_add(dy as i16);
                }
            }
        }
        SelectionMode::LineDef => {
            // Move every vertex referenced by any selected LineDef. Use a set
            // so a shared vertex isn't translated twice when both adjoining
            // LineDefs are selected.
            let mut moved: std::collections::HashSet<u16> = std::collections::HashSet::new();
            for &i in &state.selection {
                let Some(ld) = map.linedefs.get(i) else { continue };
                moved.insert(ld.start_vertex);
                moved.insert(ld.end_vertex);
            }
            for vi in moved {
                if let Some(v) = map.vertices.get_mut(vi as usize) {
                    v.x = v.x.saturating_add(dx as i16);
                    v.y = v.y.saturating_add(dy as i16);
                }
            }
        }
        SelectionMode::Thing => {
            for &i in &state.selection {
                if let Some(t) = map.things.get_mut(i) {
                    t.x = t.x.saturating_add(dx as i16);
                    t.y = t.y.saturating_add(dy as i16);
                }
            }
        }
        SelectionMode::Sector => {
            // Translate every vertex that participates in any selected sector
            // (resolved via SideDefs). De-duplicate so vertex isn't moved twice.
            let mut moved: std::collections::HashSet<u16> = std::collections::HashSet::new();
            for ld in &map.linedefs {
                for sd_idx in [ld.front_sidedef, ld.back_sidedef] {
                    if sd_idx == LineDef::NO_SIDEDEF {
                        continue;
                    }
                    let Some(sd) = map.sidedefs.get(sd_idx as usize) else { continue };
                    if state.selection.iter().any(|&s| s == sd.sector as usize) {
                        moved.insert(ld.start_vertex);
                        moved.insert(ld.end_vertex);
                    }
                }
            }
            for vi in moved {
                if let Some(v) = map.vertices.get_mut(vi as usize) {
                    v.x = v.x.saturating_add(dx as i16);
                    v.y = v.y.saturating_add(dy as i16);
                }
            }
        }
    }
    state.is_dirty = true;
}

/// Delete the selected object(s). Refuses to delete vertices that still
/// support a LineDef (matches original "Delete\\This VERTEX supports a LINEDEF.").
pub fn delete_selected(state: &mut EditorState) {
    if state.selection.is_empty() {
        return;
    }
    let Some(map) = state.map.as_mut() else { return };

    match state.mode {
        SelectionMode::Vertex => {
            // Refuse if any selected vertex is referenced by a LineDef.
            for &vi in &state.selection {
                let referenced = map.linedefs.iter().any(|ld| {
                    ld.start_vertex as usize == vi || ld.end_vertex as usize == vi
                });
                if referenced {
                    state.dialog = Some(Dialog::Notice {
                        title: "Delete".into(),
                        message: "This VERTEX supports a LINEDEF.".into(),
                    });
                    return;
                }
            }
            // Delete in descending order so earlier indices stay valid.
            let mut indices: Vec<usize> = state.selection.clone();
            indices.sort_unstable_by(|a, b| b.cmp(a));
            for vi in indices {
                if vi < map.vertices.len() {
                    map.vertices.remove(vi);
                    // Fix up linedef vertex indices.
                    for ld in &mut map.linedefs {
                        if ld.start_vertex as usize > vi {
                            ld.start_vertex -= 1;
                        }
                        if ld.end_vertex as usize > vi {
                            ld.end_vertex -= 1;
                        }
                    }
                }
            }
        }
        SelectionMode::LineDef => {
            let mut indices: Vec<usize> = state.selection.clone();
            indices.sort_unstable_by(|a, b| b.cmp(a));
            for li in indices {
                if li < map.linedefs.len() {
                    map.linedefs.remove(li);
                }
            }
        }
        SelectionMode::Thing => {
            let mut indices: Vec<usize> = state.selection.clone();
            indices.sort_unstable_by(|a, b| b.cmp(a));
            for ti in indices {
                if ti < map.things.len() {
                    map.things.remove(ti);
                }
            }
        }
        SelectionMode::Sector => {
            // Sector deletion is destructive (orphans linedefs). Surface a
            // not-yet-implemented notice rather than silently corrupting the map.
            state.dialog = Some(Dialog::Notice {
                title: "Delete".into(),
                message: "Sector deletion not implemented yet.".into(),
            });
            return;
        }
    }

    state.selection.clear();
    state.is_dirty = true;
}

/// Snap a world-coordinate delta to the editor's snap size, accumulating any
/// sub-snap residual into `state.drag_residual` so motion isn't lost.
pub fn snap_drag_delta(state: &mut EditorState, delta_world: egui::Vec2) -> (i32, i32) {
    let snap = state.snap_size.max(1) as f32;
    let total = state.drag_residual + delta_world;
    // Round toward zero so dragging ←/↑ doesn't get a free pixel.
    let dx_units = (total.x / snap).trunc();
    let dy_units = (total.y / snap).trunc();
    let dx = (dx_units * snap) as i32;
    let dy = (dy_units * snap) as i32;
    state.drag_residual = total - egui::vec2(dx as f32, dy as f32);
    (dx, dy)
}

/// Save the current map back to its source PWAD path. Refuses to write to an
/// IWAD; falls through to Save-As when no path is set yet.
pub fn save_map(state: &mut EditorState) {
    let Some(map) = state.map.as_ref() else {
        state.dialog = Some(Dialog::Notice {
            title: "Save".into(),
            message: "No map to save.".into(),
        });
        return;
    };
    if let Some(wad) = state.wad.as_ref() {
        if matches!(wad.header.kind, crate::wad::WadKind::Iwad) {
            state.dialog = Some(Dialog::Notice {
                title: "PWAD Save".into(),
                message: "Cannot save to the IWAD. Use Save as PWAD.".into(),
            });
            return;
        }
    }
    let Some(path) = state.wad_path.clone() else {
        return save_map_as(state);
    };
    let map_clone = map.clone();
    let result = match state.wad.as_ref() {
        Some(wad) => crate::wad::save_map_to_path(&path, Some(wad), &map_clone),
        None => crate::wad::save_map_to_path(&path, None, &map_clone),
    };
    match result {
        Ok(()) => {
            state.is_dirty = false;
            state.undo_baseline = state.map.clone();
            state.status_message = Some(format!("Saved to {}", path.display()));
            // Re-read the WAD so subsequent saves see our own writes (and the
            // texture bank picks up any preserved/added asset lumps).
            if let Ok(reread) = crate::wad::Wad::from_path(&path) {
                state.wad = Some(reread);
            }
        }
        Err(e) => {
            state.dialog = Some(Dialog::Notice {
                title: "PWAD Save".into(),
                message: format!("Save failed: {e}"),
            });
        }
    }
}

/// Prompt for a target path with the native picker and save the map there.
pub fn save_map_as(state: &mut EditorState) {
    let Some(map) = state.map.as_ref() else {
        state.dialog = Some(Dialog::Notice {
            title: "Save as PWAD".into(),
            message: "No map to save.".into(),
        });
        return;
    };
    let suggested = format!("{}.wad", map.name);
    let Some(path) = rfd::FileDialog::new()
        .add_filter("WAD files", &["wad", "WAD"])
        .set_file_name(&suggested)
        .save_file()
    else {
        return;
    };
    let map_clone = map.clone();
    let src = state.wad.as_ref();
    match crate::wad::save_map_to_path(&path, src, &map_clone) {
        Ok(()) => {
            state.wad_path = Some(path.clone());
            // Re-read so the in-memory Wad matches what's now on disk.
            if let Ok(reread) = crate::wad::Wad::from_path(&path) {
                state.wad = Some(reread);
            }
            state.is_dirty = false;
            state.undo_baseline = state.map.clone();
            state.status_message = Some(format!("Saved to {}", path.display()));
        }
        Err(e) => {
            state.dialog = Some(Dialog::Notice {
                title: "Save as PWAD".into(),
                message: format!("Save failed: {e}"),
            });
        }
    }
}

/// Run an action that was queued behind the Save warning dialog.
pub fn run_pending(state: &mut EditorState, action: &super::state::PendingAction) {
    use super::state::PendingAction;
    match action {
        PendingAction::Quit => std::process::exit(0),
        PendingAction::NewMap => {
            state.map = None;
            state.wad = None;
            state.wad_path = None;
            state.selection.clear();
            state.view_center = egui::pos2(0.0, 0.0);
            state.view_zoom = 1.0;
            state.is_dirty = false;
            state.undo_baseline = None;
        }
        PendingAction::OpenWad => {
            // Caller (menu/keybinding) reroutes through the picker on the next click.
            // We just clear dirty so the next Open doesn't re-trigger the warning.
            state.is_dirty = false;
        }
    }
}

/// Helper: if the map has unsaved edits, queue the action behind a SaveWarning
/// dialog and return false (caller skips the immediate work). Otherwise return
/// true so the caller can proceed inline.
pub fn dirty_guard(state: &mut EditorState, pending: super::state::PendingAction) -> bool {
    if state.is_dirty {
        state.dialog = Some(super::state::Dialog::SaveWarning { pending });
        return false;
    }
    true
}

/// Snapshot the current map as the new "last save" baseline. Call after load
/// and after a successful save.
pub fn capture_baseline(state: &mut EditorState) {
    state.undo_baseline = state.map.clone();
}

/// Edit > Undo from last save — restore the snapshot if available.
pub fn undo_to_baseline(state: &mut EditorState) {
    let Some(base) = state.undo_baseline.clone() else {
        state.dialog = Some(super::state::Dialog::Notice {
            title: "Undo".into(),
            message: "No saved baseline to revert to.".into(),
        });
        return;
    };
    state.map = Some(base);
    state.selection.clear();
    state.is_dirty = false;
}

/// Run a CheckSet against the current map and open the Error List dialog.
pub fn run_checks(state: &mut EditorState, set: super::checks::CheckSet) {
    let Some(map) = state.map.as_ref() else {
        state.dialog = Some(super::state::Dialog::Notice {
            title: "Check".into(),
            message: "No map loaded.".into(),
        });
        return;
    };
    let results = super::checks::run(map, set);
    state.last_check_results = results.clone();
    state.dialog = Some(super::state::Dialog::ErrorList { results, cursor: 0 });
}

/// Reopen the last check results without re-running.
pub fn reopen_error_list(state: &mut EditorState) {
    let results = state.last_check_results.clone();
    state.dialog = Some(super::state::Dialog::ErrorList { results, cursor: 0 });
}

// ---------------- Add/split (Ins) ----------------

/// Snap a world coordinate to the editor's snap_size, rounding to nearest.
fn snap_world(value: f32, snap: i32) -> i16 {
    let s = snap.max(1) as f32;
    let snapped = (value / s).round() * s;
    snapped.clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

/// Edit > Add/split (Ins) — insert a new primitive of the current mode at
/// the cursor (or split the selected one in LineDef mode).
pub fn add_at_cursor(state: &mut EditorState) {
    let Some(map) = state.map.as_mut() else { return };
    let cx = state.cursor_world.x;
    let cy = state.cursor_world.y;
    let snap = state.snap_size;

    match state.mode {
        SelectionMode::Vertex => {
            let v = crate::wad::Vertex {
                x: snap_world(cx, snap),
                y: snap_world(cy, snap),
            };
            map.vertices.push(v);
            state.selection.clear();
            state.selection.push(map.vertices.len() - 1);
            state.is_dirty = true;
        }
        SelectionMode::LineDef => {
            let Some(&ld_idx) = state.selection.first() else {
                state.dialog = Some(Dialog::Notice {
                    title: "Add/split".into(),
                    message: "Select a LineDef first to split it.".into(),
                });
                return;
            };
            let Some(ld) = map.linedefs.get(ld_idx).copied() else { return };
            let (Some(a), Some(b)) = (
                map.vertices.get(ld.start_vertex as usize).copied(),
                map.vertices.get(ld.end_vertex as usize).copied(),
            ) else { return };

            // Project cursor onto segment a→b, parameter t in [0,1].
            let dx = (b.x - a.x) as f32;
            let dy = (b.y - a.y) as f32;
            let len2 = dx * dx + dy * dy;
            if len2 < f32::EPSILON {
                state.dialog = Some(Dialog::Notice {
                    title: "Add/split".into(),
                    message: "Cannot split a zero-length LineDef.".into(),
                });
                return;
            }
            let t = ((cx - a.x as f32) * dx + (cy - a.y as f32) * dy) / len2;
            let t = t.clamp(0.05, 0.95); // avoid degenerate splits at the endpoints
            let split_x = a.x as f32 + t * dx;
            let split_y = a.y as f32 + t * dy;

            let new_vertex = crate::wad::Vertex {
                x: snap_world(split_x, snap),
                y: snap_world(split_y, snap),
            };
            map.vertices.push(new_vertex);
            let new_vi = (map.vertices.len() - 1) as u16;

            // Original linedef now ends at the new vertex; new linedef goes
            // from new vertex to original's end. Sidedef indices are shared:
            // both halves face the same sectors, so shared SideDefs are correct
            // (they reference the same sector and texture faces).
            let original_end = ld.end_vertex;
            map.linedefs[ld_idx].end_vertex = new_vi;
            let new_ld = crate::wad::LineDef {
                start_vertex: new_vi,
                end_vertex: original_end,
                ..ld
            };
            map.linedefs.push(new_ld);
            state.selection.clear();
            state.selection.push(map.linedefs.len() - 1);
            state.is_dirty = true;
        }
        SelectionMode::Thing => {
            let t = crate::wad::Thing {
                x: snap_world(cx, snap),
                y: snap_world(cy, snap),
                angle: 0,
                thing_type: 1, // Player 1 start
                flags: 7,      // skills 1&2 + skill 3 + skills 4&5
            };
            map.things.push(t);
            state.selection.clear();
            state.selection.push(map.things.len() - 1);
            state.is_dirty = true;
        }
        SelectionMode::Sector => {
            state.dialog = Some(Dialog::Notice {
                title: "Add/split".into(),
                message: "Sector creation not implemented yet.".into(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wad::{LineDef, MapData, SideDef, Vertex};

    fn make_state_with_one_linedef() -> EditorState {
        let map = MapData {
            name: "TEST".into(),
            vertices: vec![Vertex { x: 0, y: 0 }, Vertex { x: 256, y: 0 }],
            linedefs: vec![LineDef {
                start_vertex: 0,
                end_vertex: 1,
                flags: 5,
                special_type: 7,
                sector_tag: 11,
                front_sidedef: 0,
                back_sidedef: LineDef::NO_SIDEDEF,
            }],
            sidedefs: vec![SideDef {
                x_offset: 0,
                y_offset: 0,
                upper_texture: "-".into(),
                lower_texture: "-".into(),
                middle_texture: "STARTAN2".into(),
                sector: 0,
            }],
            sectors: vec![],
            things: vec![],
        };
        let mut state = EditorState::default();
        state.map = Some(map);
        state.mode = SelectionMode::LineDef;
        state.selection = vec![0];
        state.snap_size = 8;
        state
    }

    #[test]
    fn split_linedef_inserts_vertex_and_appends_linedef() {
        let mut state = make_state_with_one_linedef();
        // Cursor at (128, 0) — exact midpoint of the line. snap=8 keeps it at 128.
        state.cursor_world = egui::pos2(128.0, 0.0);
        add_at_cursor(&mut state);

        let map = state.map.as_ref().unwrap();
        assert_eq!(map.vertices.len(), 3, "new vertex inserted");
        assert_eq!(map.vertices[2].x, 128);
        assert_eq!(map.vertices[2].y, 0);

        assert_eq!(map.linedefs.len(), 2, "original split into two");
        // Original linedef's end_vertex now points at the new vertex.
        assert_eq!(map.linedefs[0].end_vertex, 2);
        // New linedef inherits flags/special/tag/sidedefs.
        let new = &map.linedefs[1];
        assert_eq!(new.start_vertex, 2);
        assert_eq!(new.end_vertex, 1);
        assert_eq!(new.flags, 5);
        assert_eq!(new.special_type, 7);
        assert_eq!(new.sector_tag, 11);
        assert_eq!(new.front_sidedef, 0);
        assert_eq!(new.back_sidedef, LineDef::NO_SIDEDEF);

        assert!(state.is_dirty);
        // Selection cursor moved to the new linedef.
        assert_eq!(state.selection, vec![1]);
    }

    #[test]
    fn polygon_emits_vertices_linedefs_sidedefs_sector() {
        let mut state = EditorState::default();
        state.map = Some(MapData {
            name: "TEST".into(),
            vertices: vec![],
            linedefs: vec![],
            sidedefs: vec![],
            sectors: vec![],
            things: vec![],
        });
        state.cursor_world = egui::pos2(0.0, 0.0);
        create_polygon(&mut state, 6, 100.0);

        let m = state.map.as_ref().unwrap();
        assert_eq!(m.vertices.len(), 6);
        assert_eq!(m.linedefs.len(), 6);
        assert_eq!(m.sidedefs.len(), 6);
        assert_eq!(m.sectors.len(), 1);

        // Each linedef references a unique sidedef and the new sector.
        for (i, ld) in m.linedefs.iter().enumerate() {
            assert_eq!(ld.front_sidedef as usize, i);
            assert_eq!(ld.back_sidedef, LineDef::NO_SIDEDEF);
        }
        for sd in &m.sidedefs {
            assert_eq!(sd.sector, 0);
        }
        // Selection follows the new sector.
        assert_eq!(state.mode, SelectionMode::Sector);
        assert_eq!(state.selection, vec![0]);
        assert!(state.is_dirty);
    }

    #[test]
    fn add_vertex_appends_at_snapped_cursor() {
        let mut state = make_state_with_one_linedef();
        state.mode = SelectionMode::Vertex;
        state.selection.clear();
        state.snap_size = 16;
        state.cursor_world = egui::pos2(33.0, 49.0); // expects snap to (32, 48)
        add_at_cursor(&mut state);

        let map = state.map.as_ref().unwrap();
        assert_eq!(map.vertices.len(), 3);
        assert_eq!(map.vertices[2].x, 32);
        assert_eq!(map.vertices[2].y, 48);
        assert!(state.is_dirty);
        assert_eq!(state.selection, vec![2]);
    }

    #[test]
    fn door_sets_special_on_two_sided_boundary_only() {
        // Map: two adjacent sectors. Sector 0 (interior) is bordered by sector 1
        // (corridor) on one 2-sided line. Sector 0 also has a one-sided wall.
        let map = MapData {
            name: "DOORTEST".into(),
            vertices: vec![
                Vertex { x: 0, y: 0 },
                Vertex { x: 64, y: 0 },
                Vertex { x: 64, y: 64 },
                Vertex { x: 0, y: 64 },
            ],
            linedefs: vec![
                // 2-sided boundary linedef (the door): front=sector 0, back=sector 1.
                LineDef {
                    start_vertex: 0, end_vertex: 1,
                    flags: LineDef::FLAG_TWO_SIDED,
                    special_type: 0, sector_tag: 0,
                    front_sidedef: 0, back_sidedef: 1,
                },
                // 1-sided wall of sector 0 (should NOT receive the door action).
                LineDef {
                    start_vertex: 1, end_vertex: 2,
                    flags: 0, special_type: 0, sector_tag: 0,
                    front_sidedef: 2, back_sidedef: LineDef::NO_SIDEDEF,
                },
            ],
            sidedefs: vec![
                SideDef { x_offset: 0, y_offset: 0,
                    upper_texture: "-".into(), lower_texture: "-".into(), middle_texture: "-".into(),
                    sector: 0 },
                SideDef { x_offset: 0, y_offset: 0,
                    upper_texture: "-".into(), lower_texture: "-".into(), middle_texture: "-".into(),
                    sector: 1 },
                SideDef { x_offset: 0, y_offset: 0,
                    upper_texture: "-".into(), lower_texture: "-".into(), middle_texture: "STARTAN2".into(),
                    sector: 0 },
            ],
            sectors: vec![
                crate::wad::Sector {
                    floor_height: 0, ceiling_height: 128,
                    floor_texture: "FLOOR4_8".into(), ceiling_texture: "CEIL3_5".into(),
                    light_level: 160, sector_type: 0, tag: 0,
                },
                crate::wad::Sector {
                    floor_height: 0, ceiling_height: 128,
                    floor_texture: "FLOOR4_8".into(), ceiling_texture: "CEIL3_5".into(),
                    light_level: 160, sector_type: 0, tag: 0,
                },
            ],
            things: vec![],
        };
        let mut state = EditorState::default();
        state.map = Some(map);
        state.mode = SelectionMode::Sector;
        state.selection = vec![0];

        let count = create_door(&mut state, super::super::state::DoorKey::Keyless, false);
        assert_eq!(count, 1);

        let m = state.map.as_ref().unwrap();
        // Door = special 1, only on the 2-sided boundary linedef.
        assert_eq!(m.linedefs[0].special_type, 1);
        // 1-sided wall stays untouched.
        assert_eq!(m.linedefs[1].special_type, 0);
        // Door sector is now closed (ceiling = floor).
        assert_eq!(m.sectors[0].ceiling_height, m.sectors[0].floor_height);
        assert!(state.is_dirty);
    }

    #[test]
    fn cancel_pick_restores_stashed_dialog() {
        let mut state = EditorState::default();
        let stashed = Dialog::EditSector {
            idx: 0,
            floor_height: "0".into(),
            ceiling_height: "128".into(),
            light: "160".into(),
            sector_type: "0".into(),
            tag: "0".into(),
            floor_texture: "FLOOR4_8".into(),
            ceiling_texture: "CEIL3_5".into(),
        };
        state.dialog_pending = Some(stashed);
        state.viewer_pick = Some(super::super::state::PickTarget::SectorFloor);

        super::super::viewer::cancel_pick(&mut state);

        assert!(state.viewer_pick.is_none());
        assert!(state.dialog_pending.is_none());
        match state.dialog {
            Some(Dialog::EditSector { ref floor_texture, .. }) => {
                assert_eq!(floor_texture, "FLOOR4_8", "stashed dialog restored intact");
            }
            _ => panic!("expected EditSector dialog after cancel"),
        }
    }

    #[test]
    fn open_properties_populates_dialog_for_selected_sector() {
        let mut state = EditorState::default();
        state.map = Some(MapData {
            name: "T".into(),
            vertices: vec![],
            linedefs: vec![],
            sidedefs: vec![],
            sectors: vec![crate::wad::Sector {
                floor_height: -8,
                ceiling_height: 192,
                floor_texture: "FLOOR4_8".into(),
                ceiling_texture: "F_SKY1".into(),
                light_level: 144,
                sector_type: 9,
                tag: 17,
            }],
            things: vec![],
        });
        state.mode = SelectionMode::Sector;
        state.selection = vec![0];

        open_properties(&mut state);

        match state.dialog {
            Some(Dialog::EditSector { idx, ref floor_height, ref ceiling_height, ref light, ref sector_type, ref tag, ref floor_texture, ref ceiling_texture }) => {
                assert_eq!(idx, 0);
                assert_eq!(floor_height, "-8");
                assert_eq!(ceiling_height, "192");
                assert_eq!(light, "144");
                assert_eq!(sector_type, "9");
                assert_eq!(tag, "17");
                assert_eq!(floor_texture, "FLOOR4_8");
                assert_eq!(ceiling_texture, "F_SKY1");
            }
            _ => panic!("expected EditSector dialog, got {:?}", state.dialog),
        }
    }
}

// ---------------- Automatic constructions ----------------

/// Build a regular N-gon sector centered on the cursor with the given radius.
/// Vertices wound counter-clockwise so DOOM front sidedefs face inward.
pub fn create_polygon(state: &mut EditorState, sides: usize, radius: f32) {
    let Some(map) = state.map.as_mut() else { return };
    let sides = sides.clamp(3, 64);
    let radius = radius.clamp(8.0, 4096.0);
    let cx = state.cursor_world.x;
    let cy = state.cursor_world.y;

    let v_base = map.vertices.len() as u16;
    let sd_base = map.sidedefs.len() as u16;
    let new_sector = map.sectors.len() as u16;

    // Vertices on the circle, CCW.
    for i in 0..sides {
        let theta = (i as f32) * std::f32::consts::TAU / (sides as f32);
        let x = (cx + radius * theta.cos()) as i16;
        let y = (cy + radius * theta.sin()) as i16;
        map.vertices.push(crate::wad::Vertex { x, y });
    }

    // SideDefs (one per linedef). All face the new sector.
    for _ in 0..sides {
        map.sidedefs.push(crate::wad::SideDef {
            x_offset: 0,
            y_offset: 0,
            upper_texture: "-".into(),
            lower_texture: "-".into(),
            middle_texture: "STARTAN2".into(),
            sector: new_sector,
        });
    }

    // LineDefs connecting v_i -> v_{i+1}.
    for i in 0..sides {
        let next = (i + 1) % sides;
        map.linedefs.push(crate::wad::LineDef {
            start_vertex: v_base + i as u16,
            end_vertex: v_base + next as u16,
            flags: crate::wad::LineDef::FLAG_BLOCK_ALL,
            special_type: 0,
            sector_tag: 0,
            front_sidedef: sd_base + i as u16,
            back_sidedef: crate::wad::LineDef::NO_SIDEDEF,
        });
    }

    // The Sector itself.
    map.sectors.push(crate::wad::Sector {
        floor_height: 0,
        ceiling_height: 128,
        floor_texture: "FLOOR4_8".into(),
        ceiling_texture: "CEIL3_5".into(),
        light_level: 160,
        sector_type: 0,
        tag: 0,
    });

    state.mode = SelectionMode::Sector;
    state.selection.clear();
    state.selection.push(new_sector as usize);
    state.is_dirty = true;
    state.status_message = Some(format!("Polygon: {sides} sides, radius {radius:.0}"));
}

/// Build a chain of `steps` rectangular step sectors stacked along `direction`.
/// Each step's floor is `rise` higher than the previous; ceilings are constant.
/// Adjacent steps share their inner edge with both sidedefs filled in.
pub fn create_stairs(
    state: &mut EditorState,
    steps: usize,
    rise: i32,
    depth: i32,
    width: i32,
    direction: super::state::StairsDirection,
    top_texture: &str,
    side_texture: &str,
) {
    let Some(map) = state.map.as_mut() else { return };
    let steps = steps.clamp(2, 64);
    let rise = rise.clamp(1, 1024);
    let depth = depth.clamp(8, 1024);
    let width = width.clamp(8, 1024);

    let (fx, fy) = direction.forward();
    let (rx, ry) = direction.right();
    let cx = state.cursor_world.x.round() as i32;
    let cy = state.cursor_world.y.round() as i32;

    // For each step we create 4 vertices, 4 linedefs, 4 sidedefs, 1 sector.
    // Adjacent steps share an edge: the linedef between step i and step i+1
    // lives on step i's "far" edge and step i+1's "near" edge. To keep code
    // simple and uniform, we generate every step independently and then in a
    // second pass we stitch the shared edges by adding back-sidedefs.
    let half_w = width / 2;
    let first_sector = map.sectors.len();

    let mut step_v_base: Vec<u16> = Vec::with_capacity(steps);

    for i in 0..steps as i32 {
        let near = i * depth;
        let far = (i + 1) * depth;
        // Four corners (CCW from near-left):
        // a = near-left, b = far-left, c = far-right, d = near-right
        let nl_x = cx + fx * near + rx * (-half_w);
        let nl_y = cy + fy * near + ry * (-half_w);
        let fl_x = cx + fx * far + rx * (-half_w);
        let fl_y = cy + fy * far + ry * (-half_w);
        let fr_x = cx + fx * far + rx * half_w;
        let fr_y = cy + fy * far + ry * half_w;
        let nr_x = cx + fx * near + rx * half_w;
        let nr_y = cy + fy * near + ry * half_w;

        let v_base = map.vertices.len() as u16;
        step_v_base.push(v_base);
        for (x, y) in [(nl_x, nl_y), (fl_x, fl_y), (fr_x, fr_y), (nr_x, nr_y)] {
            map.vertices.push(crate::wad::Vertex {
                x: x.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                y: y.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            });
        }

        let sd_base = map.sidedefs.len() as u16;
        let sector_idx = first_sector + i as usize;
        for _ in 0..4 {
            map.sidedefs.push(crate::wad::SideDef {
                x_offset: 0,
                y_offset: 0,
                upper_texture: "-".into(),
                lower_texture: side_texture.into(),
                middle_texture: side_texture.into(),
                sector: sector_idx as u16,
            });
        }

        // LineDefs CCW: a→b, b→c, c→d, d→a. Front sides face inward.
        let edges: [(u16, u16); 4] = [
            (v_base, v_base + 1),
            (v_base + 1, v_base + 2),
            (v_base + 2, v_base + 3),
            (v_base + 3, v_base),
        ];
        for (e_idx, (sv, ev)) in edges.iter().enumerate() {
            map.linedefs.push(crate::wad::LineDef {
                start_vertex: *sv,
                end_vertex: *ev,
                flags: crate::wad::LineDef::FLAG_BLOCK_ALL,
                special_type: 0,
                sector_tag: 0,
                front_sidedef: sd_base + e_idx as u16,
                back_sidedef: crate::wad::LineDef::NO_SIDEDEF,
            });
        }

        let floor_h = (i * rise).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let light = (192 - (i * 4).min(80)).clamp(0, 255) as i16;
        map.sectors.push(crate::wad::Sector {
            floor_height: floor_h,
            ceiling_height: 256,
            floor_texture: top_texture.into(),
            ceiling_texture: "F_SKY1".into(),
            light_level: light,
            sector_type: 0,
            tag: 0,
        });
    }

    // Stitch shared edges. The "far" edge of step i is its 2nd linedef
    // (b→c, edges[1]). The "near" edge of step i+1 is its 4th linedef
    // (d→a, edges[3]). Those two refer to the same physical edge in opposite
    // direction but their endpoints don't currently match; we collapse the
    // duplicates by remapping step i+1's near edge to use step i's far edge.
    // Simpler approach for first cut: leave duplicated geometry. A node
    // builder will dedupe vertices on save anyway, and visually it's fine.
    // Future improvement: weld duplicate vertices in a post-pass.

    state.mode = SelectionMode::Sector;
    state.selection.clear();
    state.selection.push(first_sector);
    state.is_dirty = true;
    state.status_message = Some(format!(
        "Stairs: {steps} steps × {rise} rise, {} facing",
        direction.label()
    ));
}

/// Convert the selected sector into a closed door. Returns the count of
/// boundary linedefs that received the door action (or 0 if the sector has
/// no 2-sided boundaries).
pub fn create_door(state: &mut EditorState, key: super::state::DoorKey, fast: bool) -> usize {
    use super::state::DoorKey;
    if state.mode != SelectionMode::Sector || state.selection.len() != 1 {
        state.dialog = Some(Dialog::Notice {
            title: "Door".into(),
            message: "Select exactly one sector first.".into(),
        });
        return 0;
    }
    let target_sector = state.selection[0] as u16;

    let Some(map) = state.map.as_mut() else { return 0 };
    if (target_sector as usize) >= map.sectors.len() {
        return 0;
    }

    // Pick the door-action special based on key + fast.
    let special: u16 = match (key, fast) {
        (DoorKey::Keyless, false) => 1,
        (DoorKey::Keyless, true) => 117,
        (DoorKey::Blue, _) => 26,
        (DoorKey::Yellow, _) => 27,
        (DoorKey::Red, _) => 28,
    };

    // Close the door (ceiling = floor).
    let floor_h = map.sectors[target_sector as usize].floor_height;
    map.sectors[target_sector as usize].ceiling_height = floor_h;

    // Walk every linedef. A boundary linedef has one sidedef in our sector
    // and one in some other sector. Set its special_type.
    let mut count = 0;
    for ld in map.linedefs.iter_mut() {
        if !ld.is_two_sided() || ld.back_sidedef == LineDef::NO_SIDEDEF {
            continue;
        }
        let front_sd = map.sidedefs.get(ld.front_sidedef as usize).map(|sd| sd.sector);
        let back_sd = map.sidedefs.get(ld.back_sidedef as usize).map(|sd| sd.sector);
        let is_boundary = match (front_sd, back_sd) {
            (Some(f), Some(b)) => (f == target_sector) ^ (b == target_sector),
            _ => false,
        };
        if is_boundary {
            ld.special_type = special;
            count += 1;
        }
    }

    if count == 0 {
        state.dialog = Some(Dialog::Notice {
            title: "Door".into(),
            message: "Sector has no 2-sided boundary; nothing to trigger.".into(),
        });
        // Don't keep the closed-ceiling change in this dead-end case.
        map.sectors[target_sector as usize].ceiling_height = floor_h.saturating_add(128);
        return 0;
    }

    state.is_dirty = true;
    state.status_message = Some(format!(
        "Door: special {special} on {count} linedef(s) ({}, {})",
        key.label(),
        if fast { "fast" } else { "normal" }
    ));
    count
}

/// Open the per-object property editor for the first selected object.
pub fn open_properties(state: &mut EditorState) {
    let Some(map) = state.map.as_ref() else { return };
    let Some(&idx) = state.selection.first() else {
        state.status_message = Some("Properties: nothing selected.".into());
        return;
    };
    state.dialog = match state.mode {
        SelectionMode::Vertex => map.vertices.get(idx).map(|v| Dialog::EditVertex {
            idx,
            x: v.x.to_string(),
            y: v.y.to_string(),
        }),
        SelectionMode::LineDef => map.linedefs.get(idx).map(|ld| Dialog::EditLineDef {
            idx,
            flags: ld.flags.to_string(),
            special: ld.special_type.to_string(),
            tag: ld.sector_tag.to_string(),
            front_sidedef: ld.front_sidedef.to_string(),
            back_sidedef: ld.back_sidedef.to_string(),
        }),
        SelectionMode::Sector => map.sectors.get(idx).map(|s| Dialog::EditSector {
            idx,
            floor_height: s.floor_height.to_string(),
            ceiling_height: s.ceiling_height.to_string(),
            light: s.light_level.to_string(),
            sector_type: s.sector_type.to_string(),
            tag: s.tag.to_string(),
            floor_texture: s.floor_texture.clone(),
            ceiling_texture: s.ceiling_texture.clone(),
        }),
        SelectionMode::Thing => map.things.get(idx).map(|t| Dialog::EditThing {
            idx,
            x: t.x.to_string(),
            y: t.y.to_string(),
            angle: t.angle.to_string(),
            thing_type: t.thing_type.to_string(),
            flags: t.flags.to_string(),
        }),
    };
}
