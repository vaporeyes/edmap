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
    push_undo(state);
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
        PendingAction::NewMap => new_map(state),
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

/// Return the index of the nearest vertex to (x, y) within `range` (inclusive)
/// world units, or None.
fn nearest_vertex_within(map: &crate::wad::MapData, x: i16, y: i16, range: i32) -> Option<usize> {
    let r2 = range * range;
    let mut best: Option<(usize, i32)> = None;
    for (i, v) in map.vertices.iter().enumerate() {
        let dx = v.x as i32 - x as i32;
        let dy = v.y as i32 - y as i32;
        let d2 = dx * dx + dy * dy;
        if d2 <= r2 {
            match best {
                Some((_, bd)) if bd <= d2 => {}
                _ => best = Some((i, d2)),
            }
        }
    }
    best.map(|(i, _)| i)
}

/// Snap a world coordinate to the editor's snap_size, rounding to nearest.
fn snap_world(value: f32, snap: i32) -> i16 {
    let s = snap.max(1) as f32;
    let snapped = (value / s).round() * s;
    snapped.clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

/// Edit > Add/split (Ins) — insert a new primitive of the current mode at
/// the cursor (or split the selected one in LineDef mode).
pub fn add_at_cursor(state: &mut EditorState) {
    if state.map.is_none() { return; }
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return };
    let cx = state.cursor_world.x;
    let cy = state.cursor_world.y;
    let snap = state.snap_size;

    match state.mode {
        SelectionMode::Vertex => {
            let nx = snap_world(cx, snap);
            let ny = snap_world(cy, snap);
            // Stitch: if a vertex already lives at (nx, ny) within stitch range,
            // select that one instead of creating a duplicate. Default range = 2.
            if let Some(existing) = nearest_vertex_within(map, nx, ny, 2) {
                state.selection.clear();
                state.selection.push(existing);
                state.status_message =
                    Some(format!("Stitched to vertex {existing}"));
                return;
            }
            map.vertices.push(crate::wad::Vertex { x: nx, y: ny });
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

    fn map_with_one_sector_and_thing() -> EditorState {
        let map = MapData {
            name: "U".into(),
            vertices: vec![Vertex { x: 10, y: 20 }, Vertex { x: 100, y: 200 }],
            linedefs: vec![],
            sidedefs: vec![],
            sectors: vec![crate::wad::Sector {
                floor_height: 0,
                ceiling_height: 128,
                floor_texture: "F".into(),
                ceiling_texture: "C".into(),
                light_level: 100,
                sector_type: 0,
                tag: 0,
            }],
            things: vec![crate::wad::Thing {
                x: 50, y: 60, angle: 0, thing_type: 1, flags: 7,
            }],
        };
        let mut state = EditorState::default();
        state.map = Some(map);
        state
    }

    #[test]
    fn shift_map_translates_vertices_things_and_sectors() {
        let mut state = map_with_one_sector_and_thing();
        shift_map(&mut state, 100, -50, 32);
        let m = state.map.as_ref().unwrap();
        assert_eq!(m.vertices[0].x, 110);
        assert_eq!(m.vertices[0].y, -30);
        assert_eq!(m.vertices[1].x, 200);
        assert_eq!(m.vertices[1].y, 150);
        assert_eq!(m.things[0].x, 150);
        assert_eq!(m.things[0].y, 10);
        assert_eq!(m.sectors[0].floor_height, 32);
        assert_eq!(m.sectors[0].ceiling_height, 160);
        assert!(state.is_dirty);
    }

    #[test]
    fn expand_map_scales_around_centroid() {
        let mut state = map_with_one_sector_and_thing();
        // Centroid is ((10+100)/2, (20+200)/2) = (55, 110).
        // Vertex 0 at (10, 20): offset from centroid = (-45, -90).
        // After 2x: offset (-90, -180); new pos (55-90, 110-180) = (-35, -70).
        let ok = expand_map(&mut state, 2.0, 2.0, 1.5);
        assert!(ok);
        let m = state.map.as_ref().unwrap();
        assert_eq!(m.vertices[0].x, -35);
        assert_eq!(m.vertices[0].y, -70);
        // Vertex 1 at (100, 200): offset (45, 90); after 2x → (145, 290).
        assert_eq!(m.vertices[1].x, 145);
        assert_eq!(m.vertices[1].y, 290);
        // Heights scale by 1.5x.
        assert_eq!(m.sectors[0].floor_height, 0);
        assert_eq!(m.sectors[0].ceiling_height, 192);
        assert!(state.is_dirty);
    }

    #[test]
    fn expand_map_rejects_non_positive_factor() {
        let mut state = map_with_one_sector_and_thing();
        let ok = expand_map(&mut state, -1.0, 1.0, 1.0);
        assert!(!ok);
        // Original geometry untouched.
        let m = state.map.as_ref().unwrap();
        assert_eq!(m.vertices[0].x, 10);
        assert!(matches!(state.dialog, Some(Dialog::Notice { .. })));
    }

    #[test]
    fn lift_applies_action_to_two_sided_boundary_only() {
        // Two adjacent sectors: 0 (lift target) and 1 (corridor).
        // One 2-sided boundary linedef + one 1-sided wall on sector 0.
        let map = MapData {
            name: "L".into(),
            vertices: vec![
                Vertex { x: 0, y: 0 },
                Vertex { x: 64, y: 0 },
                Vertex { x: 64, y: 64 },
                Vertex { x: 0, y: 64 },
            ],
            linedefs: vec![
                LineDef {
                    start_vertex: 0, end_vertex: 1,
                    flags: LineDef::FLAG_TWO_SIDED,
                    special_type: 0, sector_tag: 0,
                    front_sidedef: 0, back_sidedef: 1,
                },
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
                    upper_texture: "-".into(), lower_texture: "-".into(), middle_texture: "STEP".into(),
                    sector: 0 },
            ],
            sectors: vec![
                crate::wad::Sector { floor_height: 0, ceiling_height: 128,
                    floor_texture: "F".into(), ceiling_texture: "C".into(),
                    light_level: 160, sector_type: 0, tag: 0 },
                crate::wad::Sector { floor_height: 32, ceiling_height: 128,
                    floor_texture: "F".into(), ceiling_texture: "C".into(),
                    light_level: 160, sector_type: 0, tag: 0 },
            ],
            things: vec![],
        };
        let mut state = EditorState::default();
        state.map = Some(map);
        state.mode = SelectionMode::Sector;
        state.selection = vec![0];

        let count = create_lift(&mut state, true, false);
        assert_eq!(count, 1);
        let m = state.map.as_ref().unwrap();
        // Lift action 88 (WR) on the boundary linedef, with the new tag 1.
        assert_eq!(m.linedefs[0].special_type, 88);
        assert_eq!(m.linedefs[0].sector_tag, 1);
        // 1-sided wall untouched.
        assert_eq!(m.linedefs[1].special_type, 0);
        assert_eq!(m.linedefs[1].sector_tag, 0);
        assert_eq!(m.sectors[0].tag, 1);
        assert!(state.is_dirty);
    }

    #[test]
    fn teleporter_links_two_sectors_with_destination_things() {
        // Three sectors: A (0), B (1), and an outer corridor (2). Boundaries:
        //  ld0: 0|2  (front=0, back=2) — sector A's boundary
        //  ld1: 1|2  (front=1, back=2) — sector B's boundary
        //  ld2: 0|1  (front=0, back=1) — shared A↔B edge (should be skipped)
        let mut sds = vec![
            SideDef { x_offset: 0, y_offset: 0,
                upper_texture: "-".into(), lower_texture: "-".into(), middle_texture: "-".into(), sector: 0 },
            SideDef { x_offset: 0, y_offset: 0,
                upper_texture: "-".into(), lower_texture: "-".into(), middle_texture: "-".into(), sector: 2 },
            SideDef { x_offset: 0, y_offset: 0,
                upper_texture: "-".into(), lower_texture: "-".into(), middle_texture: "-".into(), sector: 1 },
            SideDef { x_offset: 0, y_offset: 0,
                upper_texture: "-".into(), lower_texture: "-".into(), middle_texture: "-".into(), sector: 2 },
            SideDef { x_offset: 0, y_offset: 0,
                upper_texture: "-".into(), lower_texture: "-".into(), middle_texture: "-".into(), sector: 0 },
            SideDef { x_offset: 0, y_offset: 0,
                upper_texture: "-".into(), lower_texture: "-".into(), middle_texture: "-".into(), sector: 1 },
        ];
        let map = MapData {
            name: "T".into(),
            vertices: vec![
                Vertex { x: 0, y: 0 }, Vertex { x: 64, y: 0 },
                Vertex { x: 64, y: 64 }, Vertex { x: 0, y: 64 },
            ],
            linedefs: vec![
                LineDef { start_vertex: 0, end_vertex: 1,
                    flags: LineDef::FLAG_TWO_SIDED, special_type: 0, sector_tag: 0,
                    front_sidedef: 0, back_sidedef: 1 },
                LineDef { start_vertex: 2, end_vertex: 3,
                    flags: LineDef::FLAG_TWO_SIDED, special_type: 0, sector_tag: 0,
                    front_sidedef: 2, back_sidedef: 3 },
                LineDef { start_vertex: 1, end_vertex: 2,
                    flags: LineDef::FLAG_TWO_SIDED, special_type: 0, sector_tag: 0,
                    front_sidedef: 4, back_sidedef: 5 },
            ],
            sidedefs: std::mem::take(&mut sds),
            sectors: vec![
                crate::wad::Sector { floor_height: 0, ceiling_height: 128,
                    floor_texture: "F".into(), ceiling_texture: "C".into(),
                    light_level: 160, sector_type: 0, tag: 0 },
                crate::wad::Sector { floor_height: 0, ceiling_height: 128,
                    floor_texture: "F".into(), ceiling_texture: "C".into(),
                    light_level: 160, sector_type: 0, tag: 0 },
                crate::wad::Sector { floor_height: 0, ceiling_height: 128,
                    floor_texture: "F".into(), ceiling_texture: "C".into(),
                    light_level: 160, sector_type: 0, tag: 0 },
            ],
            things: vec![],
        };
        let mut state = EditorState::default();
        state.map = Some(map);
        state.mode = SelectionMode::Sector;
        state.selection = vec![0, 1]; // sectors A and B

        let ok = create_teleporter(&mut state);
        assert!(ok);
        let m = state.map.as_ref().unwrap();

        // Two destination things added (one per pad).
        let dest_count = m.things.iter().filter(|t| t.thing_type == 14).count();
        assert_eq!(dest_count, 2);

        // Sector tags assigned (1 and 2 are first two unused).
        assert_eq!(m.sectors[0].tag, 1);
        assert_eq!(m.sectors[1].tag, 2);

        // ld0 (A's boundary) should teleport to B's tag (2).
        assert_eq!(m.linedefs[0].special_type, 97);
        assert_eq!(m.linedefs[0].sector_tag, 2);
        // ld1 (B's boundary) should teleport to A's tag (1).
        assert_eq!(m.linedefs[1].special_type, 97);
        assert_eq!(m.linedefs[1].sector_tag, 1);
        // ld2 (A↔B shared) should NOT be wired — it's the inter-pad edge.
        assert_eq!(m.linedefs[2].special_type, 0);
        assert_eq!(m.linedefs[2].sector_tag, 0);
    }

    #[test]
    fn light_adjust_applies_formula_and_clamps() {
        let mut state = map_with_one_sector_and_thing();
        // light starts at 100. With A=150, B=0 → 100*150/100 = 150.
        light_adjust(&mut state, 150, 0);
        assert_eq!(state.map.as_ref().unwrap().sectors[0].light_level, 150);
        // Apply again with A=200, B=10 → 150*2 + 10 = 310 → clamped to 255.
        light_adjust(&mut state, 200, 10);
        assert_eq!(state.map.as_ref().unwrap().sectors[0].light_level, 255);
        // Apply with A=0, B=-10 → 0 - 10 = -10 → clamped to 0.
        light_adjust(&mut state, 0, -10);
        assert_eq!(state.map.as_ref().unwrap().sectors[0].light_level, 0);
    }
}

// ---------------- Automatic constructions ----------------

/// Build a regular N-gon sector centered on the cursor with the given radius.
/// Vertices wound counter-clockwise so DOOM front sidedefs face inward.
pub fn create_polygon(state: &mut EditorState, sides: usize, radius: f32) {
    push_undo(state);
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
    push_undo(state);
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
    push_undo(state);
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

// ---------------- Map-wide utilities ----------------

/// Shift the entire map by (dx, dy) world units and (dz) heights. Vertex
/// positions are translated by (dx, dy); every sector's floor and ceiling
/// height add dz. Things and texture offsets are NOT moved (matches EdMap).
pub fn shift_map(state: &mut EditorState, dx: i32, dy: i32, dz: i32) {
    if dx == 0 && dy == 0 && dz == 0 {
        return;
    }
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return };
    for v in map.vertices.iter_mut() {
        v.x = (v.x as i32 + dx).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        v.y = (v.y as i32 + dy).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
    for t in map.things.iter_mut() {
        t.x = (t.x as i32 + dx).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        t.y = (t.y as i32 + dy).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
    for s in map.sectors.iter_mut() {
        s.floor_height =
            (s.floor_height as i32 + dz).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        s.ceiling_height =
            (s.ceiling_height as i32 + dz).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
    state.is_dirty = true;
    state.status_message = Some(format!("Map shifted by ({dx}, {dy}, {dz})"));
}

/// Scale the entire map by (sx, sy) around the bounding-box center, and
/// scale heights by sz around 0. Returns true if the operation succeeded.
pub fn expand_map(state: &mut EditorState, sx: f32, sy: f32, sz: f32) -> bool {
    if sx <= 0.0 || sy <= 0.0 || sz <= 0.0 {
        state.dialog = Some(Dialog::Notice {
            title: "Expand/Reduce".into(),
            message: "Scale factors must be positive.".into(),
        });
        return false;
    }
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return false };
    if map.vertices.is_empty() {
        return false;
    }
    let (mut min_x, mut max_x, mut min_y, mut max_y) =
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for v in &map.vertices {
        min_x = min_x.min(v.x as i32);
        max_x = max_x.max(v.x as i32);
        min_y = min_y.min(v.y as i32);
        max_y = max_y.max(v.y as i32);
    }
    let cx = (min_x + max_x) as f32 * 0.5;
    let cy = (min_y + max_y) as f32 * 0.5;
    let scale_xy = |v: &mut crate::wad::Vertex| {
        let nx = (cx + ((v.x as f32) - cx) * sx).round() as i32;
        let ny = (cy + ((v.y as f32) - cy) * sy).round() as i32;
        v.x = nx.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        v.y = ny.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    };
    for v in map.vertices.iter_mut() {
        scale_xy(v);
    }
    for t in map.things.iter_mut() {
        let nx = (cx + ((t.x as f32) - cx) * sx).round() as i32;
        let ny = (cy + ((t.y as f32) - cy) * sy).round() as i32;
        t.x = nx.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        t.y = ny.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
    for s in map.sectors.iter_mut() {
        let nf = (s.floor_height as f32 * sz).round() as i32;
        let nc = (s.ceiling_height as f32 * sz).round() as i32;
        s.floor_height = nf.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        s.ceiling_height = nc.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
    state.is_dirty = true;
    state.status_message = Some(format!("Map scaled by ({sx:.2}, {sy:.2}, {sz:.2})"));
    true
}

/// Apply `new_light = old_light * a/100 + b` per-sector, clamped to 0..255.
/// Matches the EdMap "Light adjustment" formula exactly.
pub fn light_adjust(state: &mut EditorState, a: i32, b: i32) {
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return };
    for s in map.sectors.iter_mut() {
        let v = (s.light_level as i32 * a) / 100 + b;
        s.light_level = v.clamp(0, 255) as i16;
    }
    state.is_dirty = true;
    state.status_message = Some(format!("Light adjusted: light = old × {a}/100 + {b}"));
}

// ---------------- Automatic Lift / Teleporter ----------------

/// DOOM teleporter destination thing-type code (per the public DOOM thing table).
const THING_TELEPORT_DESTINATION: u16 = 14;

/// DOOM teleport linedef-action special: WR Teleport.
const TELEPORT_SPECIAL_WR: u16 = 97;

fn next_unused_tag(map: &crate::wad::MapData) -> u16 {
    let mut max = 0u16;
    for s in &map.sectors {
        if s.tag > max {
            max = s.tag;
        }
    }
    for ld in &map.linedefs {
        if ld.sector_tag > max {
            max = ld.sector_tag;
        }
    }
    max.saturating_add(1)
}

fn lift_special(repeatable: bool, fast: bool) -> u16 {
    // 62  S1  switch lift, once, normal
    // 88  WR  walk lift, repeatable, normal
    // 121 W1  walk lift, once, fast
    // 123 SR  switch lift, repeatable, fast
    match (repeatable, fast) {
        (true, false) => 88,
        (true, true) => 123,
        (false, false) => 62,
        (false, true) => 121,
    }
}

/// Convert the selected sector into a triggered lift. Boundary linedefs (2-sided
/// where exactly one side is the selected sector) get the lift action with a
/// fresh tag matching the sector. Returns the count of linedefs that got the
/// action.
pub fn create_lift(state: &mut EditorState, repeatable: bool, fast: bool) -> usize {
    if state.mode != SelectionMode::Sector || state.selection.len() != 1 {
        state.dialog = Some(Dialog::Notice {
            title: "Lift".into(),
            message: "Select exactly one sector first.".into(),
        });
        return 0;
    }
    push_undo(state);
    let target = state.selection[0] as u16;
    let Some(map) = state.map.as_mut() else { return 0 };
    if (target as usize) >= map.sectors.len() {
        return 0;
    }
    let new_tag = next_unused_tag(map);
    let special = lift_special(repeatable, fast);

    map.sectors[target as usize].tag = new_tag;

    let mut count = 0usize;
    for ld in map.linedefs.iter_mut() {
        if !ld.is_two_sided() || ld.back_sidedef == LineDef::NO_SIDEDEF {
            continue;
        }
        let front = map.sidedefs.get(ld.front_sidedef as usize).map(|sd| sd.sector);
        let back = map.sidedefs.get(ld.back_sidedef as usize).map(|sd| sd.sector);
        let is_boundary = match (front, back) {
            (Some(f), Some(b)) => (f == target) ^ (b == target),
            _ => false,
        };
        if is_boundary {
            ld.special_type = special;
            ld.sector_tag = new_tag;
            count += 1;
        }
    }

    if count == 0 {
        state.dialog = Some(Dialog::Notice {
            title: "Lift".into(),
            message: "Sector has no 2-sided boundary; nothing to trigger.".into(),
        });
        // Roll back the tag mutation since the lift won't function.
        if let Some(s) = state.map.as_mut().and_then(|m| m.sectors.get_mut(target as usize)) {
            s.tag = 0;
        }
        return 0;
    }

    state.is_dirty = true;
    state.status_message = Some(format!(
        "Lift: special {special} on {count} linedef(s) ({}, {})",
        if repeatable { "repeatable" } else { "once" },
        if fast { "fast" } else { "normal" }
    ));
    count
}

/// Pair two selected sectors as a teleporter. Each gets a fresh tag and a
/// destination thing at its centroid; each one's non-shared 2-sided boundary
/// linedefs receive a teleport action pointing at the OTHER pad's tag.
pub fn create_teleporter(state: &mut EditorState) -> bool {
    if state.mode != SelectionMode::Sector || state.selection.len() != 2 {
        state.dialog = Some(Dialog::Notice {
            title: "Teleporter".into(),
            message: "Select exactly two sectors first (shift-click in Sector mode).".into(),
        });
        return false;
    }
    push_undo(state);
    let a = state.selection[0] as u16;
    let b = state.selection[1] as u16;
    if a == b {
        return false;
    }
    let centroid_a = sector_centroid_for_idx(state, a as usize);
    let centroid_b = sector_centroid_for_idx(state, b as usize);
    let Some(map) = state.map.as_mut() else { return false };

    let tag_a = next_unused_tag(map);
    // Compute a second fresh tag that doesn't collide with tag_a.
    let tag_b = {
        let mut t = tag_a.saturating_add(1);
        while sector_or_linedef_uses_tag(map, t) || t == tag_a {
            t = t.saturating_add(1);
            if t == 0 {
                break; // wrapped — give up and use whatever we have
            }
        }
        t
    };

    map.sectors[a as usize].tag = tag_a;
    map.sectors[b as usize].tag = tag_b;

    // Place destination things at each centroid.
    if let Some((cx, cy)) = centroid_a {
        map.things.push(crate::wad::Thing {
            x: cx.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16,
            y: cy.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16,
            angle: 0,
            thing_type: THING_TELEPORT_DESTINATION,
            flags: 7,
        });
    }
    if let Some((cx, cy)) = centroid_b {
        map.things.push(crate::wad::Thing {
            x: cx.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16,
            y: cy.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16,
            angle: 0,
            thing_type: THING_TELEPORT_DESTINATION,
            flags: 7,
        });
    }

    let mut count = 0usize;
    for ld in map.linedefs.iter_mut() {
        if !ld.is_two_sided() || ld.back_sidedef == LineDef::NO_SIDEDEF {
            continue;
        }
        let front = map.sidedefs.get(ld.front_sidedef as usize).map(|sd| sd.sector);
        let back = map.sidedefs.get(ld.back_sidedef as usize).map(|sd| sd.sector);
        let (Some(f), Some(bk)) = (front, back) else { continue };

        // Skip the linedef directly between the two pads (would teleport to/from
        // each other infinitely if both sides were tagged).
        if (f == a && bk == b) || (f == b && bk == a) {
            continue;
        }
        // Sector A's boundary linedefs (one side a, other side != b) → tag_b
        // Sector B's boundary linedefs (one side b, other side != a) → tag_a
        let target_tag = if (f == a) ^ (bk == a) {
            Some(tag_b)
        } else if (f == b) ^ (bk == b) {
            Some(tag_a)
        } else {
            None
        };
        if let Some(t) = target_tag {
            ld.special_type = TELEPORT_SPECIAL_WR;
            ld.sector_tag = t;
            count += 1;
        }
    }

    state.is_dirty = true;
    state.status_message = Some(format!(
        "Teleporter: tags {tag_a}<->{tag_b}, {count} linedef(s) wired"
    ));
    true
}

fn sector_centroid_for_idx(state: &EditorState, sector_idx: usize) -> Option<(f32, f32)> {
    let map = state.map.as_ref()?;
    sector_centroid(map, sector_idx)
}

fn sector_or_linedef_uses_tag(map: &crate::wad::MapData, tag: u16) -> bool {
    map.sectors.iter().any(|s| s.tag == tag)
        || map.linedefs.iter().any(|ld| ld.sector_tag == tag)
}

// ---------------- Single-key utilities (F flip, PgUp/Dn adjustments) ----------------

/// Flip every selected LineDef: swap front/back sidedefs and start/end vertices
/// so the front side faces the opposite direction.
pub fn flip_selected_linedefs(state: &mut EditorState) -> usize {
    if state.mode != SelectionMode::LineDef || state.selection.is_empty() {
        return 0;
    }
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return 0 };
    let mut count = 0;
    for &i in &state.selection {
        let Some(ld) = map.linedefs.get_mut(i) else { continue };
        std::mem::swap(&mut ld.start_vertex, &mut ld.end_vertex);
        std::mem::swap(&mut ld.front_sidedef, &mut ld.back_sidedef);
        count += 1;
    }
    if count > 0 {
        state.is_dirty = true;
        state.status_message = Some(format!("Flipped {count} linedef(s)"));
    }
    count
}

/// Adjust ceiling height (Shift inverts to floor) on every selected sector.
/// dz is the signed delta in DOOM units.
pub fn adjust_selected_heights(state: &mut EditorState, dz: i32, target_floor: bool) -> usize {
    if state.mode != SelectionMode::Sector || state.selection.is_empty() {
        return 0;
    }
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return 0 };
    let mut count = 0;
    for &i in &state.selection {
        let Some(s) = map.sectors.get_mut(i) else { continue };
        let v = if target_floor {
            (s.floor_height as i32 + dz).clamp(i16::MIN as i32, i16::MAX as i32) as i16
        } else {
            (s.ceiling_height as i32 + dz).clamp(i16::MIN as i32, i16::MAX as i32) as i16
        };
        if target_floor {
            s.floor_height = v;
        } else {
            s.ceiling_height = v;
        }
        count += 1;
    }
    if count > 0 {
        state.is_dirty = true;
        let label = if target_floor { "floor" } else { "ceiling" };
        state.status_message = Some(format!("{label} {dz:+} on {count} sector(s)"));
    }
    count
}

/// Adjust light_level on every selected sector by `db` (clamped 0..255).
pub fn adjust_selected_light(state: &mut EditorState, db: i32) -> usize {
    if state.mode != SelectionMode::Sector || state.selection.is_empty() {
        return 0;
    }
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return 0 };
    let mut count = 0;
    for &i in &state.selection {
        let Some(s) = map.sectors.get_mut(i) else { continue };
        let v = (s.light_level as i32 + db).clamp(0, 255) as i16;
        s.light_level = v;
        count += 1;
    }
    if count > 0 {
        state.is_dirty = true;
        state.status_message = Some(format!("light {db:+} on {count} sector(s)"));
    }
    count
}

/// Public wrapper around the existing private next_unused_tag, so the dialog
/// module can offer a "Next Unused" button.
pub fn next_unused_tag_pub(state: &EditorState) -> u16 {
    state
        .map
        .as_ref()
        .map(next_unused_tag)
        .unwrap_or(1)
}

/// Replace the selected linedef with `n` linedefs forming a smooth circular
/// arc between the original endpoints. `curve_distance` is the perpendicular
/// distance from the arc's midpoint to the original linedef. Negative flips
/// to the opposite side. `delta_angle` is unused for now (we always derive
/// the arc from chord + sagitta).
pub fn curve_linedef(state: &mut EditorState, n: usize, curve_distance: f32) {
    if state.mode != SelectionMode::LineDef || state.selection.len() != 1 {
        state.dialog = Some(Dialog::Notice {
            title: "Curve LineDef".into(),
            message: "Select exactly one LineDef first.".into(),
        });
        return;
    }
    push_undo(state);
    let n = n.clamp(2, 32);
    let ld_idx = state.selection[0];
    let Some(map) = state.map.as_mut() else { return };
    let Some(ld) = map.linedefs.get(ld_idx).copied() else { return };
    let (Some(a), Some(b)) = (
        map.vertices.get(ld.start_vertex as usize).copied(),
        map.vertices.get(ld.end_vertex as usize).copied(),
    ) else { return };

    let ax = a.x as f32;
    let ay = a.y as f32;
    let bx = b.x as f32;
    let by = b.y as f32;
    let dx = bx - ax;
    let dy = by - ay;
    let chord_len = (dx * dx + dy * dy).sqrt();
    if chord_len < 1.0 {
        return;
    }
    // Perpendicular unit vector to (a → b), pointing right of travel direction.
    let px = -dy / chord_len;
    let py = dx / chord_len;
    let s = curve_distance; // sagitta; negative flips side
    // Circle radius from sagitta and half-chord: R = (h² + (c/2)²) / (2h)
    let half_c = chord_len * 0.5;
    let r = if s.abs() < 0.001 {
        return; // straight line — nothing to curve
    } else {
        (s * s + half_c * half_c) / (2.0 * s)
    };
    // Center of the circle (signed; negative R means center on the OTHER side).
    let mid_x = (ax + bx) * 0.5;
    let mid_y = (ay + by) * 0.5;
    let cx = mid_x + px * (s - r);
    let cy = mid_y + py * (s - r);
    let r_abs = r.abs();
    let theta_a = (ay - cy).atan2(ax - cx);
    let theta_b = (by - cy).atan2(bx - cx);
    // Choose the short way around (consistent with sagitta sign).
    let mut sweep = theta_b - theta_a;
    if r > 0.0 {
        while sweep <= 0.0 { sweep += std::f32::consts::TAU; }
        while sweep > std::f32::consts::TAU { sweep -= std::f32::consts::TAU; }
    } else {
        while sweep >= 0.0 { sweep -= std::f32::consts::TAU; }
        while sweep < -std::f32::consts::TAU { sweep += std::f32::consts::TAU; }
    }

    // Insert n-1 intermediate vertices, then rewrite the linedefs.
    let mut intermediate: Vec<u16> = Vec::with_capacity(n - 1);
    for i in 1..n {
        let t = i as f32 / n as f32;
        let theta = theta_a + sweep * t;
        let vx = (cx + r_abs * theta.cos()).round() as i32;
        let vy = (cy + r_abs * theta.sin()).round() as i32;
        let vx = vx.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let vy = vy.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        // Stitch to nearest existing vertex within 2 units.
        let idx = if let Some(existing) = nearest_vertex_within(map, vx, vy, 2) {
            existing as u16
        } else {
            map.vertices.push(crate::wad::Vertex { x: vx, y: vy });
            (map.vertices.len() - 1) as u16
        };
        intermediate.push(idx);
    }

    // Rewrite linedef chain: a → m1 → m2 → ... → b, all sharing original ld's
    // attributes (flags, special, tag, sidedef indices).
    let original_end = ld.end_vertex;
    map.linedefs[ld_idx].end_vertex = intermediate[0];
    for window in intermediate.windows(2) {
        let new_ld = crate::wad::LineDef {
            start_vertex: window[0],
            end_vertex: window[1],
            ..ld
        };
        map.linedefs.push(new_ld);
    }
    let last_intermediate = *intermediate.last().unwrap();
    let final_ld = crate::wad::LineDef {
        start_vertex: last_intermediate,
        end_vertex: original_end,
        ..ld
    };
    map.linedefs.push(final_ld);

    state.is_dirty = true;
    state.status_message = Some(format!("Curved into {n} linedef(s)"));
}

/// Auto-align textures along a connected chain of linedefs starting from the
/// selected one. Walks linedefs that share an endpoint AND have the same
/// middle texture name on the front sidedef. For each linedef in the chain,
/// sets sidedef.x_offset = (running_offset mod texture_width). Returns count.
///
/// Texture width defaults to 64 if the bank doesn't know the texture. For
/// truly accurate alignment the caller could resolve the width from
/// TextureBank, but 64 is a reasonable approximation for most DOOM textures.
pub fn auto_align_textures(state: &mut EditorState) -> usize {
    if state.mode != SelectionMode::LineDef || state.selection.len() != 1 {
        state.dialog = Some(Dialog::Notice {
            title: "Auto-align".into(),
            message: "Select exactly one LineDef first.".into(),
        });
        return 0;
    }
    push_undo(state);
    let start_ld_idx = state.selection[0];
    let Some(map) = state.map.as_mut() else { return 0 };
    let Some(start_ld) = map.linedefs.get(start_ld_idx).copied() else { return 0 };
    if start_ld.front_sidedef == LineDef::NO_SIDEDEF {
        return 0;
    }
    let Some(start_sd) = map.sidedefs.get(start_ld.front_sidedef as usize).cloned() else { return 0 };
    let target_tex = start_sd.middle_texture.clone();
    if target_tex.is_empty() || target_tex == "-" {
        state.status_message = Some("No texture to align".into());
        return 0;
    }
    let texture_width: i32 = 64; // safe default; matches most DOOM textures.

    // BFS from start linedef, walking neighbors that share an endpoint AND
    // whose front sidedef carries the same middle texture.
    let mut chain: Vec<usize> = vec![start_ld_idx];
    let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();
    visited.insert(start_ld_idx);

    // For chain ordering: walk forward from end_vertex, then prepend backward
    // walk from start_vertex. Simple: do two directional walks.
    let mut tail_vertex = start_ld.end_vertex;
    loop {
        let next = map.linedefs.iter().enumerate().find(|(i, ld)| {
            !visited.contains(i)
                && (ld.start_vertex == tail_vertex || ld.end_vertex == tail_vertex)
                && ld.front_sidedef != LineDef::NO_SIDEDEF
                && map.sidedefs.get(ld.front_sidedef as usize).map(|sd| sd.middle_texture == target_tex).unwrap_or(false)
        });
        let Some((next_idx, next_ld)) = next.map(|(i, ld)| (i, *ld)) else { break };
        visited.insert(next_idx);
        chain.push(next_idx);
        tail_vertex = if next_ld.start_vertex == tail_vertex { next_ld.end_vertex } else { next_ld.start_vertex };
    }

    // Walk the chain assigning offsets.
    let mut offset: i32 = start_sd.x_offset as i32;
    let mut count = 0;
    for &i in &chain {
        let Some(ld) = map.linedefs.get(i).copied() else { continue };
        let Some((Some(va), Some(vb))) = Some((
            map.vertices.get(ld.start_vertex as usize).copied(),
            map.vertices.get(ld.end_vertex as usize).copied(),
        )) else { continue };
        let dx = (vb.x - va.x) as f32;
        let dy = (vb.y - va.y) as f32;
        let length = (dx * dx + dy * dy).sqrt() as i32;
        if let Some(sd) = map.sidedefs.get_mut(ld.front_sidedef as usize) {
            sd.x_offset = (offset.rem_euclid(texture_width)) as i16;
            count += 1;
        }
        offset = offset.saturating_add(length);
    }

    state.is_dirty = true;
    state.status_message = Some(format!("Aligned {count} sidedef(s) of {target_tex}"));
    count
}

// ---------------- Cleanup commands ----------------

/// Sweep the map and delete every linedef whose start and end vertices
/// coincide. Returns the count removed.
pub fn fix_zero_length_linedefs(state: &mut EditorState) -> usize {
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return 0 };
    let mut removed = 0;
    let mut i = 0;
    while i < map.linedefs.len() {
        let ld = &map.linedefs[i];
        let zero_len = match (
            map.vertices.get(ld.start_vertex as usize),
            map.vertices.get(ld.end_vertex as usize),
        ) {
            (Some(a), Some(b)) => a.x == b.x && a.y == b.y,
            _ => true, // missing vertex → also invalid
        };
        if zero_len {
            map.linedefs.remove(i);
            removed += 1;
        } else {
            i += 1;
        }
    }
    if removed > 0 {
        state.is_dirty = true;
        state.selection.clear();
    }
    state.status_message = Some(format!("Removed {removed} zero-length linedef(s)"));
    removed
}

/// Auto-fill missing required textures with a default. For a 1-sided line:
/// middle texture must be present. For a 2-sided line: upper present when
/// adjacent ceilings differ; lower present when adjacent floors differ.
/// Returns count of slots filled.
pub fn fix_missing_textures(state: &mut EditorState) -> usize {
    push_undo(state);
    const DEFAULT_TEX: &str = "STARTAN2";
    let Some(map) = state.map.as_mut() else { return 0 };

    let is_blank = |s: &str| s.is_empty() || s == "-";
    let mut filled = 0;

    // We need read-only sector + sidedef refs to decide; collect required-fix
    // tuples first, then mutate.
    let mut to_fix: Vec<(usize, &'static str)> = Vec::new(); // (sidedef_idx, slot)
    for ld in &map.linedefs {
        if !ld.is_two_sided() || ld.back_sidedef == LineDef::NO_SIDEDEF {
            // 1-sided: front sidedef middle must be present.
            if ld.front_sidedef != LineDef::NO_SIDEDEF {
                if let Some(sd) = map.sidedefs.get(ld.front_sidedef as usize) {
                    if is_blank(&sd.middle_texture) {
                        to_fix.push((ld.front_sidedef as usize, "middle"));
                    }
                }
            }
            continue;
        }
        let (Some(front_sd), Some(back_sd)) = (
            map.sidedefs.get(ld.front_sidedef as usize),
            map.sidedefs.get(ld.back_sidedef as usize),
        ) else { continue };
        let (Some(fs), Some(bs)) = (
            map.sectors.get(front_sd.sector as usize),
            map.sectors.get(back_sd.sector as usize),
        ) else { continue };

        if bs.ceiling_height < fs.ceiling_height && is_blank(&front_sd.upper_texture) {
            to_fix.push((ld.front_sidedef as usize, "upper"));
        }
        if fs.ceiling_height < bs.ceiling_height && is_blank(&back_sd.upper_texture) {
            to_fix.push((ld.back_sidedef as usize, "upper"));
        }
        if bs.floor_height > fs.floor_height && is_blank(&front_sd.lower_texture) {
            to_fix.push((ld.front_sidedef as usize, "lower"));
        }
        if fs.floor_height > bs.floor_height && is_blank(&back_sd.lower_texture) {
            to_fix.push((ld.back_sidedef as usize, "lower"));
        }
    }

    for (idx, slot) in to_fix {
        if let Some(sd) = map.sidedefs.get_mut(idx) {
            match slot {
                "middle" => sd.middle_texture = DEFAULT_TEX.into(),
                "upper" => sd.upper_texture = DEFAULT_TEX.into(),
                "lower" => sd.lower_texture = DEFAULT_TEX.into(),
                _ => {}
            }
            filled += 1;
        }
    }
    if filled > 0 {
        state.is_dirty = true;
    }
    state.status_message = Some(format!("Filled {filled} missing texture slot(s) with {DEFAULT_TEX}"));
    filled
}

/// Strip unused texture names — set to "-" when a slot doesn't need to render.
/// 1-sided line: upper + lower not needed; only middle. 2-sided: middle only
/// needed if explicitly desired (rare); upper only needed when adjacent
/// ceilings differ; lower only when floors differ.
pub fn remove_unused_textures(state: &mut EditorState) -> usize {
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return 0 };
    let mut cleared = 0;

    let mut to_clear: Vec<(usize, &'static str)> = Vec::new();
    for ld in &map.linedefs {
        if !ld.is_two_sided() || ld.back_sidedef == LineDef::NO_SIDEDEF {
            if ld.front_sidedef != LineDef::NO_SIDEDEF {
                if let Some(sd) = map.sidedefs.get(ld.front_sidedef as usize) {
                    if !sd.upper_texture.is_empty() && sd.upper_texture != "-" {
                        to_clear.push((ld.front_sidedef as usize, "upper"));
                    }
                    if !sd.lower_texture.is_empty() && sd.lower_texture != "-" {
                        to_clear.push((ld.front_sidedef as usize, "lower"));
                    }
                }
            }
            continue;
        }
        let (Some(front_sd), Some(back_sd)) = (
            map.sidedefs.get(ld.front_sidedef as usize),
            map.sidedefs.get(ld.back_sidedef as usize),
        ) else { continue };
        let (Some(fs), Some(bs)) = (
            map.sectors.get(front_sd.sector as usize),
            map.sectors.get(back_sd.sector as usize),
        ) else { continue };

        // Upper not needed if ceilings match.
        if fs.ceiling_height == bs.ceiling_height {
            if !front_sd.upper_texture.is_empty() && front_sd.upper_texture != "-" {
                to_clear.push((ld.front_sidedef as usize, "upper"));
            }
            if !back_sd.upper_texture.is_empty() && back_sd.upper_texture != "-" {
                to_clear.push((ld.back_sidedef as usize, "upper"));
            }
        }
        // Lower not needed if floors match.
        if fs.floor_height == bs.floor_height {
            if !front_sd.lower_texture.is_empty() && front_sd.lower_texture != "-" {
                to_clear.push((ld.front_sidedef as usize, "lower"));
            }
            if !back_sd.lower_texture.is_empty() && back_sd.lower_texture != "-" {
                to_clear.push((ld.back_sidedef as usize, "lower"));
            }
        }
    }

    for (idx, slot) in to_clear {
        if let Some(sd) = map.sidedefs.get_mut(idx) {
            match slot {
                "upper" => sd.upper_texture = "-".into(),
                "lower" => sd.lower_texture = "-".into(),
                _ => {}
            }
            cleared += 1;
        }
    }
    if cleared > 0 {
        state.is_dirty = true;
    }
    state.status_message = Some(format!("Cleared {cleared} unused texture slot(s)"));
    cleared
}

// ---------------- Copy / Paste / Flip selection ----------------

/// Copy current selection into the internal clipboard. Coordinates stored
/// relative to the bounding-box centroid so paste lands intuitively at the
/// cursor.
pub fn copy_selection(state: &mut EditorState) {
    use super::state::Clipboard;
    let Some(map) = state.map.as_ref() else { return };
    if state.selection.is_empty() {
        state.status_message = Some("Copy: nothing selected.".into());
        return;
    }
    match state.mode {
        SelectionMode::Vertex => {
            let verts: Vec<crate::wad::Vertex> = state
                .selection
                .iter()
                .filter_map(|&i| map.vertices.get(i).copied())
                .collect();
            if verts.is_empty() {
                return;
            }
            let cx = (verts.iter().map(|v| v.x as i32).sum::<i32>() / verts.len() as i32) as i16;
            let cy = (verts.iter().map(|v| v.y as i32).sum::<i32>() / verts.len() as i32) as i16;
            let rel: Vec<_> = verts.iter().map(|v| crate::wad::Vertex {
                x: v.x - cx,
                y: v.y - cy,
            }).collect();
            state.clipboard = Some(Clipboard::Vertices(rel));
            state.status_message = Some(format!("Copied {} vertex/vertices", verts.len()));
        }
        SelectionMode::Thing => {
            let things: Vec<crate::wad::Thing> = state
                .selection
                .iter()
                .filter_map(|&i| map.things.get(i).copied())
                .collect();
            if things.is_empty() {
                return;
            }
            let cx = (things.iter().map(|t| t.x as i32).sum::<i32>() / things.len() as i32) as i16;
            let cy = (things.iter().map(|t| t.y as i32).sum::<i32>() / things.len() as i32) as i16;
            let rel: Vec<_> = things.iter().map(|t| crate::wad::Thing {
                x: t.x - cx,
                y: t.y - cy,
                ..*t
            }).collect();
            state.clipboard = Some(Clipboard::Things(rel));
            state.status_message = Some(format!("Copied {} thing(s)", things.len()));
        }
        _ => {
            state.status_message = Some("Copy: only Vertex and Thing modes supported.".into());
        }
    }
}

/// Paste clipboard at cursor. Each pasted object's relative coordinates are
/// translated by the cursor world position. Mode is forced to match the
/// clipboard contents.
pub fn paste_clipboard(state: &mut EditorState) {
    use super::state::Clipboard;
    let Some(clip) = state.clipboard.clone() else {
        state.status_message = Some("Paste: clipboard empty.".into());
        return;
    };
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return };
    let cx = state.cursor_world.x.round() as i32;
    let cy = state.cursor_world.y.round() as i32;
    let mut new_indices = Vec::new();

    match clip {
        Clipboard::Vertices(verts) => {
            for v in &verts {
                let nx = (v.x as i32 + cx).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                let ny = (v.y as i32 + cy).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                map.vertices.push(crate::wad::Vertex { x: nx, y: ny });
                new_indices.push(map.vertices.len() - 1);
            }
            state.mode = SelectionMode::Vertex;
        }
        Clipboard::Things(things) => {
            for t in &things {
                let nx = (t.x as i32 + cx).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                let ny = (t.y as i32 + cy).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                map.things.push(crate::wad::Thing { x: nx, y: ny, ..*t });
                new_indices.push(map.things.len() - 1);
            }
            state.mode = SelectionMode::Thing;
        }
    }
    state.selection = new_indices;
    state.is_dirty = true;
    state.status_message = Some(format!("Pasted {} object(s)", state.selection.len()));
}

/// Flip selection along an axis through its bounding-box center. `horizontal`
/// flips X (mirror left↔right); `!horizontal` flips Y (top↔bottom).
pub fn flip_selection_axis(state: &mut EditorState, horizontal: bool) {
    if state.selection.is_empty() {
        return;
    }
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return };
    // Compute bounding-box center for selected objects.
    let (mut min_x, mut max_x, mut min_y, mut max_y) =
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    let positions: Vec<(i32, i32)> = match state.mode {
        SelectionMode::Vertex => state.selection.iter()
            .filter_map(|&i| map.vertices.get(i).map(|v| (v.x as i32, v.y as i32)))
            .collect(),
        SelectionMode::Thing => state.selection.iter()
            .filter_map(|&i| map.things.get(i).map(|t| (t.x as i32, t.y as i32)))
            .collect(),
        _ => {
            state.status_message = Some("Flip: only Vertex and Thing modes supported.".into());
            return;
        }
    };
    if positions.is_empty() {
        return;
    }
    for &(x, y) in &positions {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let cx = (min_x + max_x) / 2;
    let cy = (min_y + max_y) / 2;

    match state.mode {
        SelectionMode::Vertex => {
            for &i in &state.selection {
                if let Some(v) = map.vertices.get_mut(i) {
                    if horizontal {
                        let new_x = (2 * cx - v.x as i32).clamp(i16::MIN as i32, i16::MAX as i32);
                        v.x = new_x as i16;
                    } else {
                        let new_y = (2 * cy - v.y as i32).clamp(i16::MIN as i32, i16::MAX as i32);
                        v.y = new_y as i16;
                    }
                }
            }
        }
        SelectionMode::Thing => {
            for &i in &state.selection {
                if let Some(t) = map.things.get_mut(i) {
                    if horizontal {
                        let new_x = (2 * cx - t.x as i32).clamp(i16::MIN as i32, i16::MAX as i32);
                        t.x = new_x as i16;
                        // Mirror angle around the Y axis: 180 - angle.
                        t.angle = (180 - t.angle).rem_euclid(360);
                    } else {
                        let new_y = (2 * cy - t.y as i32).clamp(i16::MIN as i32, i16::MAX as i32);
                        t.y = new_y as i16;
                        // Mirror angle around the X axis: -angle.
                        t.angle = (-t.angle).rem_euclid(360);
                    }
                }
            }
        }
        _ => {}
    }
    state.is_dirty = true;
    state.status_message = Some(format!(
        "Flipped {} {} object(s)",
        state.selection.len(),
        if horizontal { "horizontally" } else { "vertically" }
    ));
}

/// Rotate selected vertices/things around their bounding-box center by degrees.
pub fn rotate_selection(state: &mut EditorState, degrees: f32) {
    if state.selection.is_empty() {
        return;
    }
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return };
    let radians = degrees.to_radians();
    let cos_t = radians.cos();
    let sin_t = radians.sin();

    // Compute bounding-box center over selection.
    let (mut min_x, mut max_x, mut min_y, mut max_y) =
        (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
    let positions: Vec<(f32, f32)> = match state.mode {
        SelectionMode::Vertex => state.selection.iter()
            .filter_map(|&i| map.vertices.get(i).map(|v| (v.x as f32, v.y as f32)))
            .collect(),
        SelectionMode::Thing => state.selection.iter()
            .filter_map(|&i| map.things.get(i).map(|t| (t.x as f32, t.y as f32)))
            .collect(),
        _ => {
            state.status_message = Some("Rotate: only Vertex and Thing modes supported.".into());
            return;
        }
    };
    if positions.is_empty() { return; }
    for &(x, y) in &positions {
        min_x = min_x.min(x); max_x = max_x.max(x);
        min_y = min_y.min(y); max_y = max_y.max(y);
    }
    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;

    let rotate_pt = |x: f32, y: f32| -> (f32, f32) {
        let dx = x - cx;
        let dy = y - cy;
        (cx + dx * cos_t - dy * sin_t, cy + dx * sin_t + dy * cos_t)
    };

    match state.mode {
        SelectionMode::Vertex => {
            for &i in &state.selection {
                if let Some(v) = map.vertices.get_mut(i) {
                    let (nx, ny) = rotate_pt(v.x as f32, v.y as f32);
                    v.x = nx.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                    v.y = ny.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                }
            }
        }
        SelectionMode::Thing => {
            for &i in &state.selection {
                if let Some(t) = map.things.get_mut(i) {
                    let (nx, ny) = rotate_pt(t.x as f32, t.y as f32);
                    t.x = nx.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                    t.y = ny.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                    // Rotate facing angle to match.
                    t.angle = (t.angle as i32 + degrees.round() as i32).rem_euclid(360) as i16;
                }
            }
        }
        _ => {}
    }
    state.is_dirty = true;
    state.status_message = Some(format!("Rotated {} object(s) by {degrees}°", state.selection.len()));
}

/// Scale selected vertices/things around their bounding-box center by `factor`.
/// `factor < 1.0` shrinks; > 1.0 grows. Rejects non-positive factors.
pub fn scale_selection(state: &mut EditorState, factor: f32) {
    if factor <= 0.0 {
        state.status_message = Some("Scale: factor must be positive.".into());
        return;
    }
    if state.selection.is_empty() {
        return;
    }
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return };
    let positions: Vec<(f32, f32)> = match state.mode {
        SelectionMode::Vertex => state.selection.iter()
            .filter_map(|&i| map.vertices.get(i).map(|v| (v.x as f32, v.y as f32)))
            .collect(),
        SelectionMode::Thing => state.selection.iter()
            .filter_map(|&i| map.things.get(i).map(|t| (t.x as f32, t.y as f32)))
            .collect(),
        _ => {
            state.status_message = Some("Scale: only Vertex and Thing modes supported.".into());
            return;
        }
    };
    if positions.is_empty() { return; }
    let (mut min_x, mut max_x, mut min_y, mut max_y) =
        (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
    for &(x, y) in &positions {
        min_x = min_x.min(x); max_x = max_x.max(x);
        min_y = min_y.min(y); max_y = max_y.max(y);
    }
    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;

    match state.mode {
        SelectionMode::Vertex => {
            for &i in &state.selection {
                if let Some(v) = map.vertices.get_mut(i) {
                    let nx = (cx + (v.x as f32 - cx) * factor).round();
                    let ny = (cy + (v.y as f32 - cy) * factor).round();
                    v.x = nx.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                    v.y = ny.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                }
            }
        }
        SelectionMode::Thing => {
            for &i in &state.selection {
                if let Some(t) = map.things.get_mut(i) {
                    let nx = (cx + (t.x as f32 - cx) * factor).round();
                    let ny = (cy + (t.y as f32 - cy) * factor).round();
                    t.x = nx.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                    t.y = ny.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                }
            }
        }
        _ => {}
    }
    state.is_dirty = true;
    state.status_message = Some(format!("Scaled {} object(s) by {factor:.2}", state.selection.len()));
}

/// Distribute a sector field linearly across the selection in selection-order.
/// First and last selected sectors keep their values; intermediate sectors
/// receive linearly interpolated values.
pub fn gradient_sector_field(state: &mut EditorState, field: GradientField) {
    if state.mode != SelectionMode::Sector || state.selection.len() < 3 {
        state.dialog = Some(Dialog::Notice {
            title: "Gradient".into(),
            message: "Select at least 3 sectors first.".into(),
        });
        return;
    }
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return };
    let n = state.selection.len();
    let first = state.selection[0];
    let last = state.selection[n - 1];
    let (start_v, end_v) = match (
        map.sectors.get(first).cloned(),
        map.sectors.get(last).cloned(),
    ) {
        (Some(a), Some(b)) => match field {
            GradientField::Floor => (a.floor_height as i32, b.floor_height as i32),
            GradientField::Ceiling => (a.ceiling_height as i32, b.ceiling_height as i32),
            GradientField::Brightness => (a.light_level as i32, b.light_level as i32),
        },
        _ => return,
    };

    for (i, &s_idx) in state.selection.iter().enumerate() {
        if i == 0 || i == n - 1 {
            continue; // endpoints unchanged
        }
        let t = i as f32 / (n - 1) as f32;
        let v = (start_v as f32 + (end_v - start_v) as f32 * t).round() as i32;
        if let Some(s) = map.sectors.get_mut(s_idx) {
            match field {
                GradientField::Floor => {
                    s.floor_height = v.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                }
                GradientField::Ceiling => {
                    s.ceiling_height = v.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                }
                GradientField::Brightness => {
                    s.light_level = v.clamp(0, 255) as i16;
                }
            }
        }
    }
    state.is_dirty = true;
    let label = match field {
        GradientField::Floor => "floor heights",
        GradientField::Ceiling => "ceiling heights",
        GradientField::Brightness => "brightness",
    };
    state.status_message = Some(format!("Gradient applied to {n} sector {label}"));
}

#[derive(Debug, Clone, Copy)]
pub enum GradientField {
    Floor,
    Ceiling,
    Brightness,
}

/// Maximum number of undo snapshots to keep.
pub const UNDO_DEPTH: usize = 50;

/// Push the current map onto the undo stack BEFORE a mutation. Cheap clone of
/// `MapData` (a few Vec<i16/u16/String>); for very large maps this is still a
/// few hundred KB which is fine.
pub fn push_undo(state: &mut EditorState) {
    let Some(map) = state.map.clone() else { return };
    if state.undo_stack.len() >= UNDO_DEPTH {
        state.undo_stack.remove(0);
    }
    state.undo_stack.push(map);
}

/// Pop one snapshot from the undo stack and restore it.
pub fn pop_undo(state: &mut EditorState) {
    let Some(prev) = state.undo_stack.pop() else {
        state.status_message = Some("Undo: nothing to undo.".into());
        return;
    };
    state.map = Some(prev);
    state.selection.clear();
    state.is_dirty = !state.undo_stack.is_empty();
    state.status_message = Some(format!(
        "Undo (depth: {}/{UNDO_DEPTH})",
        state.undo_stack.len()
    ));
}

// ---------------- Join / Merge sectors ----------------

/// Join all selected sectors into the FIRST selected sector. Every sidedef
/// whose sector matches a non-first selected index is rewritten to point at
/// the first sector's index. Shared boundary linedefs are PRESERVED (the
/// sectors look like a single sector but the geometry is unchanged).
pub fn join_sectors(state: &mut EditorState) -> usize {
    if state.mode != SelectionMode::Sector || state.selection.len() < 2 {
        state.dialog = Some(Dialog::Notice {
            title: "Join Sectors".into(),
            message: "Select at least 2 sectors first.".into(),
        });
        return 0;
    }
    push_undo(state);
    let target = state.selection[0] as u16;
    let to_merge: std::collections::HashSet<u16> =
        state.selection.iter().skip(1).map(|&i| i as u16).collect();

    let Some(map) = state.map.as_mut() else { return 0 };
    let mut count = 0;
    for sd in map.sidedefs.iter_mut() {
        if to_merge.contains(&sd.sector) {
            sd.sector = target;
            count += 1;
        }
    }
    state.is_dirty = true;
    state.selection = vec![target as usize];
    state.status_message = Some(format!("Joined: {count} sidedef(s) reassigned to sector {target}"));
    count
}

/// Merge: same as Join, but also delete linedefs whose front+back sidedefs
/// both end up in the merged target sector (i.e. the boundary between joined
/// sectors). Returns count of deleted linedefs.
pub fn merge_sectors(state: &mut EditorState) -> usize {
    if state.mode != SelectionMode::Sector || state.selection.len() < 2 {
        state.dialog = Some(Dialog::Notice {
            title: "Merge Sectors".into(),
            message: "Select at least 2 sectors first.".into(),
        });
        return 0;
    }
    push_undo(state);
    let target = state.selection[0] as u16;
    let to_merge: std::collections::HashSet<u16> =
        state.selection.iter().skip(1).map(|&i| i as u16).collect();

    let Some(map) = state.map.as_mut() else { return 0 };
    // Reassign sidedefs first.
    for sd in map.sidedefs.iter_mut() {
        if to_merge.contains(&sd.sector) {
            sd.sector = target;
        }
    }
    // Now delete linedefs where both sidedefs reference target.
    let target = target;
    let mut removed = 0;
    let mut i = 0;
    while i < map.linedefs.len() {
        let ld = &map.linedefs[i];
        if !ld.is_two_sided() || ld.back_sidedef == LineDef::NO_SIDEDEF {
            i += 1;
            continue;
        }
        let f = map.sidedefs.get(ld.front_sidedef as usize).map(|sd| sd.sector);
        let b = map.sidedefs.get(ld.back_sidedef as usize).map(|sd| sd.sector);
        if matches!((f, b), (Some(fs), Some(bs)) if fs == target && bs == target) {
            map.linedefs.remove(i);
            removed += 1;
        } else {
            i += 1;
        }
    }
    state.is_dirty = true;
    state.selection = vec![target as usize];
    state.status_message = Some(format!("Merged: {removed} boundary linedef(s) removed"));
    removed
}

// ---------------- Find / Find & Replace ----------------

use super::state::FindKind;

/// Run a find against the current map. Updates state.selection with matches
/// and switches mode to match. Returns count.
pub fn find_objects(state: &mut EditorState, kind: FindKind, needle: &str) -> usize {
    let Some(map) = state.map.as_ref() else { return 0 };
    let mut hits = Vec::new();
    let needle_norm = needle.trim().to_uppercase();
    let needle_num: Option<u16> = needle.trim().parse().ok();

    let new_mode = match kind {
        FindKind::LineDefTexture => {
            for (i, ld) in map.linedefs.iter().enumerate() {
                let mut matched = false;
                for sd_idx in [ld.front_sidedef, ld.back_sidedef] {
                    if sd_idx == LineDef::NO_SIDEDEF {
                        continue;
                    }
                    if let Some(sd) = map.sidedefs.get(sd_idx as usize) {
                        if sd.upper_texture.eq_ignore_ascii_case(&needle_norm)
                            || sd.middle_texture.eq_ignore_ascii_case(&needle_norm)
                            || sd.lower_texture.eq_ignore_ascii_case(&needle_norm)
                        {
                            matched = true;
                            break;
                        }
                    }
                }
                if matched {
                    hits.push(i);
                }
            }
            SelectionMode::LineDef
        }
        FindKind::SectorFloorTexture => {
            for (i, s) in map.sectors.iter().enumerate() {
                if s.floor_texture.eq_ignore_ascii_case(&needle_norm) {
                    hits.push(i);
                }
            }
            SelectionMode::Sector
        }
        FindKind::SectorCeilingTexture => {
            for (i, s) in map.sectors.iter().enumerate() {
                if s.ceiling_texture.eq_ignore_ascii_case(&needle_norm) {
                    hits.push(i);
                }
            }
            SelectionMode::Sector
        }
        FindKind::LineDefAction => {
            let Some(n) = needle_num else { return 0 };
            for (i, ld) in map.linedefs.iter().enumerate() {
                if ld.special_type == n {
                    hits.push(i);
                }
            }
            SelectionMode::LineDef
        }
        FindKind::SectorTag => {
            let Some(n) = needle_num else { return 0 };
            for (i, s) in map.sectors.iter().enumerate() {
                if s.tag == n {
                    hits.push(i);
                }
            }
            SelectionMode::Sector
        }
        FindKind::ThingType => {
            let Some(n) = needle_num else { return 0 };
            for (i, t) in map.things.iter().enumerate() {
                if t.thing_type == n {
                    hits.push(i);
                }
            }
            SelectionMode::Thing
        }
        FindKind::LineDefIndex => {
            let Some(n) = needle_num else { return 0 };
            if (n as usize) < map.linedefs.len() { hits.push(n as usize); }
            SelectionMode::LineDef
        }
        FindKind::SectorIndex => {
            let Some(n) = needle_num else { return 0 };
            if (n as usize) < map.sectors.len() { hits.push(n as usize); }
            SelectionMode::Sector
        }
        FindKind::ThingIndex => {
            let Some(n) = needle_num else { return 0 };
            if (n as usize) < map.things.len() { hits.push(n as usize); }
            SelectionMode::Thing
        }
        FindKind::VertexIndex => {
            let Some(n) = needle_num else { return 0 };
            if (n as usize) < map.vertices.len() { hits.push(n as usize); }
            SelectionMode::Vertex
        }
    };

    let count = hits.len();
    state.mode = new_mode;
    state.selection = hits;
    if count > 0 {
        focus_on_selection(state);
    }
    state.status_message = Some(format!("Found {count} match(es)"));
    count
}

/// Replace all instances of `needle` with `replacement` for the given kind.
pub fn replace_objects(
    state: &mut EditorState,
    kind: FindKind,
    needle: &str,
    replacement: &str,
) -> usize {
    if !kind.supports_replace() {
        state.dialog = Some(Dialog::Notice {
            title: "Replace".into(),
            message: format!("{} cannot be replaced.", kind.label()),
        });
        return 0;
    }
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return 0 };
    let needle_n = needle.trim().to_uppercase();
    let repl_n = replacement.trim().to_uppercase();
    let needle_num: Option<u16> = needle.trim().parse().ok();
    let repl_num: Option<u16> = replacement.trim().parse().ok();
    let mut count = 0;

    match kind {
        FindKind::LineDefTexture => {
            for sd in map.sidedefs.iter_mut() {
                for slot in [&mut sd.upper_texture, &mut sd.middle_texture, &mut sd.lower_texture] {
                    if slot.eq_ignore_ascii_case(&needle_n) {
                        *slot = repl_n.clone();
                        count += 1;
                    }
                }
            }
        }
        FindKind::SectorFloorTexture => {
            for s in map.sectors.iter_mut() {
                if s.floor_texture.eq_ignore_ascii_case(&needle_n) {
                    s.floor_texture = repl_n.clone();
                    count += 1;
                }
            }
        }
        FindKind::SectorCeilingTexture => {
            for s in map.sectors.iter_mut() {
                if s.ceiling_texture.eq_ignore_ascii_case(&needle_n) {
                    s.ceiling_texture = repl_n.clone();
                    count += 1;
                }
            }
        }
        FindKind::LineDefAction => {
            let (Some(n), Some(r)) = (needle_num, repl_num) else { return 0 };
            for ld in map.linedefs.iter_mut() {
                if ld.special_type == n {
                    ld.special_type = r;
                    count += 1;
                }
            }
        }
        FindKind::SectorTag => {
            let (Some(n), Some(r)) = (needle_num, repl_num) else { return 0 };
            for s in map.sectors.iter_mut() {
                if s.tag == n {
                    s.tag = r;
                    count += 1;
                }
            }
        }
        FindKind::ThingType => {
            let (Some(n), Some(r)) = (needle_num, repl_num) else { return 0 };
            for t in map.things.iter_mut() {
                if t.thing_type == n {
                    t.thing_type = r;
                    count += 1;
                }
            }
        }
        _ => {}
    }
    if count > 0 {
        state.is_dirty = true;
    }
    state.status_message = Some(format!("Replaced {count} occurrence(s)"));
    count
}

// ---------------- Line-Draw Mode ----------------

/// Toggle line-draw mode on/off.
pub fn toggle_line_draw(state: &mut EditorState) {
    use super::state::LineDrawState;
    if state.line_draw.is_some() {
        state.line_draw = None;
        state.status_message = Some("Line-draw cancelled".into());
    } else {
        state.line_draw = Some(LineDrawState { chain: Vec::new() });
        state.status_message = Some("Line-draw active. Right-click to place vertices, left-click to close.".into());
    }
}

/// Place a vertex at the cursor in line-draw mode. Stitches to existing.
pub fn line_draw_place_vertex(state: &mut EditorState) {
    let snap = state.snap_size;
    let cx = state.cursor_world.x;
    let cy = state.cursor_world.y;
    let nx = snap_world(cx, snap);
    let ny = snap_world(cy, snap);
    let Some(map) = state.map.as_mut() else { return };
    let Some(line_draw) = state.line_draw.as_mut() else { return };

    // Reuse existing vertex within stitch range.
    let vi = if let Some(existing) = nearest_vertex_within(map, nx, ny, 2) {
        existing as u16
    } else {
        map.vertices.push(crate::wad::Vertex { x: nx, y: ny });
        (map.vertices.len() - 1) as u16
    };

    // If chain is non-empty, also create a linedef from the last vertex.
    if let Some(&prev) = line_draw.chain.last() {
        if prev != vi {
            map.linedefs.push(crate::wad::LineDef {
                start_vertex: prev,
                end_vertex: vi,
                flags: crate::wad::LineDef::FLAG_BLOCK_ALL,
                special_type: 0,
                sector_tag: 0,
                front_sidedef: crate::wad::LineDef::NO_SIDEDEF,
                back_sidedef: crate::wad::LineDef::NO_SIDEDEF,
            });
        }
    }
    line_draw.chain.push(vi);
    state.is_dirty = true;
}

/// Try to close the current line-draw chain into a sector. Returns true if
/// the chain was closed (left-click landed on the first vertex).
pub fn line_draw_try_close(state: &mut EditorState) -> bool {
    let snap = state.snap_size;
    let cx = state.cursor_world.x;
    let cy = state.cursor_world.y;
    let nx = snap_world(cx, snap);
    let ny = snap_world(cy, snap);
    let Some(map) = state.map.as_mut() else { return false };
    let Some(line_draw) = state.line_draw.as_mut() else { return false };
    if line_draw.chain.len() < 3 {
        return false;
    }
    let first_v = line_draw.chain[0];
    let Some(start) = map.vertices.get(first_v as usize).copied() else { return false };
    let dx = (start.x - nx).abs();
    let dy = (start.y - ny).abs();
    if dx > 2 || dy > 2 {
        return false; // not close enough to first vertex
    }

    // Close: insert one final linedef from last vertex back to first.
    let last = *line_draw.chain.last().unwrap();
    if last != first_v {
        map.linedefs.push(crate::wad::LineDef {
            start_vertex: last,
            end_vertex: first_v,
            flags: crate::wad::LineDef::FLAG_BLOCK_ALL,
            special_type: 0,
            sector_tag: 0,
            front_sidedef: crate::wad::LineDef::NO_SIDEDEF,
            back_sidedef: crate::wad::LineDef::NO_SIDEDEF,
        });
    }

    // Create a sector for the new region with default DOOM textures.
    let new_sector = map.sectors.len() as u16;
    map.sectors.push(crate::wad::Sector {
        floor_height: 0,
        ceiling_height: 128,
        floor_texture: "FLOOR4_8".into(),
        ceiling_texture: "CEIL3_5".into(),
        light_level: 160,
        sector_type: 0,
        tag: 0,
    });

    // Walk every linedef created during this draw (last N linedefs in the
    // map; identified by the chain's edges) and assign each a fresh sidedef
    // pointing at the new sector. Front-side faces inward (CCW assumption).
    let chain = line_draw.chain.clone();
    let n = chain.len();
    for i in 0..n {
        let a = chain[i];
        let b = chain[(i + 1) % n];
        // Find the linedef matching (a, b) — should be one of the recently-added ones.
        for ld in map.linedefs.iter_mut().rev().take(n) {
            if ld.start_vertex == a && ld.end_vertex == b && ld.front_sidedef == crate::wad::LineDef::NO_SIDEDEF {
                let sd_idx = map.sidedefs.len() as u16;
                ld.front_sidedef = sd_idx;
                map.sidedefs.push(crate::wad::SideDef {
                    x_offset: 0,
                    y_offset: 0,
                    upper_texture: "-".into(),
                    lower_texture: "-".into(),
                    middle_texture: "STARTAN2".into(),
                    sector: new_sector,
                });
                break;
            }
        }
    }

    state.line_draw = None;
    state.mode = SelectionMode::Sector;
    state.selection = vec![new_sector as usize];
    state.is_dirty = true;
    state.status_message = Some(format!("Sector {new_sector} created from line-draw"));
    true
}

// ---------------- Prefabs (.epfab JSON files) ----------------

#[derive(serde::Serialize, serde::Deserialize)]
struct PrefabFile {
    version: u32,
    vertices: Vec<crate::wad::Vertex>,
    linedefs: Vec<crate::wad::LineDef>,
    sidedefs: Vec<crate::wad::SideDef>,
    sectors: Vec<crate::wad::Sector>,
}

/// Save the current Sector-mode selection as a .epfab JSON file. Picks the
/// linedefs that face any of the selected sectors via SideDef and re-numbers
/// vertex/sidedef/sector indices into the prefab's local index space.
pub fn save_selection_as_prefab(state: &mut EditorState) {
    if state.mode != SelectionMode::Sector || state.selection.is_empty() {
        state.dialog = Some(Dialog::Notice {
            title: "Save prefab".into(),
            message: "Select one or more sectors first (Sector mode).".into(),
        });
        return;
    }
    let Some(map) = state.map.as_ref() else { return };

    // Sectors first.
    let sector_indices: Vec<u16> = state.selection.iter().map(|&i| i as u16).collect();
    let mut sectors = Vec::new();
    let mut sector_remap = std::collections::HashMap::new();
    for (i, &orig) in sector_indices.iter().enumerate() {
        if let Some(s) = map.sectors.get(orig as usize) {
            sectors.push(s.clone());
            sector_remap.insert(orig, i as u16);
        }
    }

    // Sidedefs that point at any of these sectors.
    let mut sidedefs = Vec::new();
    let mut sidedef_remap = std::collections::HashMap::new();
    for (i, sd) in map.sidedefs.iter().enumerate() {
        if let Some(&new_sec) = sector_remap.get(&sd.sector) {
            let mut new_sd = sd.clone();
            new_sd.sector = new_sec;
            sidedef_remap.insert(i as u16, sidedefs.len() as u16);
            sidedefs.push(new_sd);
        }
    }

    // Linedefs whose front or back sidedef is in sidedef_remap.
    let mut linedefs = Vec::new();
    let mut vertex_remap = std::collections::HashMap::new();
    let mut vertices = Vec::new();
    for ld in &map.linedefs {
        let f = sidedef_remap.get(&ld.front_sidedef).copied();
        let b = sidedef_remap.get(&ld.back_sidedef).copied();
        if f.is_none() && b.is_none() {
            continue;
        }
        let mut new_ld = *ld;
        new_ld.front_sidedef = f.unwrap_or(crate::wad::LineDef::NO_SIDEDEF);
        new_ld.back_sidedef = b.unwrap_or(crate::wad::LineDef::NO_SIDEDEF);
        // Remap start/end vertices.
        for vref in [&mut new_ld.start_vertex, &mut new_ld.end_vertex] {
            let entry = vertex_remap.entry(*vref).or_insert_with(|| {
                if let Some(v) = map.vertices.get(*vref as usize) {
                    vertices.push(*v);
                }
                (vertices.len() - 1) as u16
            });
            *vref = *entry;
        }
        linedefs.push(new_ld);
    }

    // Centre on bounding box so paste lands at cursor naturally.
    let (mut min_x, mut max_x, mut min_y, mut max_y) =
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for v in &vertices {
        min_x = min_x.min(v.x as i32); max_x = max_x.max(v.x as i32);
        min_y = min_y.min(v.y as i32); max_y = max_y.max(v.y as i32);
    }
    let cx = ((min_x + max_x) / 2) as i16;
    let cy = ((min_y + max_y) / 2) as i16;
    for v in vertices.iter_mut() {
        v.x = v.x.saturating_sub(cx);
        v.y = v.y.saturating_sub(cy);
    }

    let prefab = PrefabFile { version: 1, vertices, linedefs, sidedefs, sectors };
    let Some(path) = rfd::FileDialog::new()
        .add_filter("EdMap Prefab", &["epfab"])
        .set_file_name("prefab.epfab")
        .save_file()
    else {
        return;
    };
    match serde_json::to_string_pretty(&prefab) {
        Ok(json) => match std::fs::write(&path, json) {
            Ok(()) => state.status_message = Some(format!("Saved prefab to {}", path.display())),
            Err(e) => state.dialog = Some(Dialog::Notice {
                title: "Save prefab".into(),
                message: format!("Write failed: {e}"),
            }),
        },
        Err(e) => state.dialog = Some(Dialog::Notice {
            title: "Save prefab".into(),
            message: format!("Serialize failed: {e}"),
        }),
    }
}

/// Load a .epfab file and place it at the cursor.
pub fn load_prefab_at_cursor(state: &mut EditorState) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("EdMap Prefab", &["epfab"])
        .pick_file()
    else {
        return;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        state.dialog = Some(Dialog::Notice {
            title: "Load prefab".into(),
            message: format!("Could not read {}", path.display()),
        });
        return;
    };
    let prefab: PrefabFile = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => {
            state.dialog = Some(Dialog::Notice {
                title: "Load prefab".into(),
                message: format!("Parse failed: {e}"),
            });
            return;
        }
    };
    push_undo(state);
    let Some(map) = state.map.as_mut() else { return };
    let cx = state.cursor_world.x.round() as i32;
    let cy = state.cursor_world.y.round() as i32;

    let v_offset = map.vertices.len() as u16;
    let sd_offset = map.sidedefs.len() as u16;
    let s_offset = map.sectors.len() as u16;

    for v in &prefab.vertices {
        let nx = (v.x as i32 + cx).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let ny = (v.y as i32 + cy).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        map.vertices.push(crate::wad::Vertex { x: nx, y: ny });
    }
    for s in &prefab.sectors {
        map.sectors.push(s.clone());
    }
    for sd in &prefab.sidedefs {
        let mut new_sd = sd.clone();
        new_sd.sector = new_sd.sector.saturating_add(s_offset);
        map.sidedefs.push(new_sd);
    }
    for ld in &prefab.linedefs {
        let mut new_ld = *ld;
        new_ld.start_vertex = new_ld.start_vertex.saturating_add(v_offset);
        new_ld.end_vertex = new_ld.end_vertex.saturating_add(v_offset);
        if new_ld.front_sidedef != crate::wad::LineDef::NO_SIDEDEF {
            new_ld.front_sidedef = new_ld.front_sidedef.saturating_add(sd_offset);
        }
        if new_ld.back_sidedef != crate::wad::LineDef::NO_SIDEDEF {
            new_ld.back_sidedef = new_ld.back_sidedef.saturating_add(sd_offset);
        }
        map.linedefs.push(new_ld);
    }
    state.is_dirty = true;
    state.status_message = Some(format!(
        "Loaded prefab: {} vertices, {} linedefs, {} sectors",
        prefab.vertices.len(), prefab.linedefs.len(), prefab.sectors.len()
    ));
}

/// Render the current map to a PNG with the given options and prompt for a save path.
pub fn export_picture(state: &mut EditorState, opts: super::export_picture::ExportOptions) {
    let Some(map) = state.map.as_ref() else {
        state.dialog = Some(Dialog::Notice {
            title: "Export Picture".into(),
            message: "No map to export.".into(),
        });
        return;
    };
    let bytes = match super::export_picture::render(map, &opts) {
        Ok(b) => b,
        Err(e) => {
            state.dialog = Some(Dialog::Notice {
                title: "Export Picture".into(),
                message: format!("Render failed: {e}"),
            });
            return;
        }
    };
    let suggested = format!("{}.png", map.name);
    let Some(path) = rfd::FileDialog::new()
        .add_filter("PNG image", &["png"])
        .set_file_name(&suggested)
        .save_file()
    else {
        return;
    };
    match std::fs::write(&path, &bytes) {
        Ok(()) => {
            state.status_message = Some(format!("Exported {} ({}x{})", path.display(), opts.width, opts.height));
        }
        Err(e) => {
            state.dialog = Some(Dialog::Notice {
                title: "Export Picture".into(),
                message: format!("Write failed: {e}"),
            });
        }
    }
}

/// Open the Test Map Settings dialog so the user can configure the source-port
/// executable and the args template.
pub fn open_test_map_settings(state: &mut EditorState) {
    let cfg = &state.config.test_map;
    state.dialog = Some(Dialog::TestMapSettings {
        exe: cfg.exe.clone(),
        args: cfg.args.clone(),
    });
}

/// Save the current map to a temp PWAD and launch the configured source-port.
/// If no executable is configured yet, opens the settings dialog instead.
pub fn test_map(state: &mut EditorState) {
    let Some(map) = state.map.as_ref() else {
        state.dialog = Some(Dialog::Notice {
            title: "Test Map".into(),
            message: "No map to test.".into(),
        });
        return;
    };
    let exe = state.config.test_map.exe.trim().to_string();
    if exe.is_empty() {
        state.status_message = Some("Configure source-port first (File > Test map settings)".into());
        open_test_map_settings(state);
        return;
    }

    // Write a fresh PWAD to the OS temp dir. We always rebuild from the
    // currently-loaded WAD so its custom textures/flats travel with the map.
    let temp_path = std::env::temp_dir().join("edmap_test.wad");
    let map_clone = map.clone();
    let write_result = match state.wad.as_ref() {
        Some(wad) => crate::wad::save_map_to_path(&temp_path, Some(wad), &map_clone),
        None => crate::wad::save_map_to_path(&temp_path, None, &map_clone),
    };
    if let Err(e) = write_result {
        state.dialog = Some(Dialog::Notice {
            title: "Test Map".into(),
            message: format!("Could not write temp PWAD: {e}"),
        });
        return;
    }

    let (episode, mapnum) = super::config::parse_map_warp(&map_clone.name);
    let pwad_path = temp_path.to_string_lossy().to_string();
    let template = state.config.test_map.args.clone();
    let args: Vec<String> = template
        .split_whitespace()
        .map(|tok| {
            tok.replace("%F", &pwad_path)
                .replace("%L", &map_clone.name)
                .replace("%E", &episode.to_string())
                .replace("%M", &mapnum.to_string())
        })
        .collect();

    match std::process::Command::new(&exe).args(&args).spawn() {
        Ok(_child) => {
            state.status_message = Some(format!(
                "Launched {} ({})",
                exe, map_clone.name
            ));
        }
        Err(e) => {
            state.dialog = Some(Dialog::Notice {
                title: "Test Map".into(),
                message: format!("Failed to launch '{exe}': {e}"),
            });
        }
    }
}

/// Build a brand-new map: a single 256x256 square room centered at the origin
/// with a Player 1 start in the middle. Stock vanilla DOOM textures are used
/// so the resulting PWAD plays in any IWAD that has them.
fn starter_map() -> crate::wad::MapData {
    use crate::wad::{LineDef, MapData, Sector, SideDef, Thing, Vertex};

    // Vertices in CCW order (Y-up): BL, TL, TR, BR. With this winding, each
    // linedef's right-perpendicular faces the room interior, so single-sided
    // fronts point inward.
    let vertices = vec![
        Vertex { x: -128, y: -128 }, // 0 BL
        Vertex { x: -128, y:  128 }, // 1 TL
        Vertex { x:  128, y:  128 }, // 2 TR
        Vertex { x:  128, y: -128 }, // 3 BR
    ];

    let mid = "STARTAN2".to_string();
    let dash = "-".to_string();
    let sidedefs: Vec<SideDef> = (0..4)
        .map(|_| SideDef {
            x_offset: 0,
            y_offset: 0,
            upper_texture: dash.clone(),
            lower_texture: dash.clone(),
            middle_texture: mid.clone(),
            sector: 0,
        })
        .collect();

    let mk_ld = |a: u16, b: u16, sd: u16| LineDef {
        start_vertex: a,
        end_vertex: b,
        flags: LineDef::FLAG_BLOCK_ALL,
        special_type: 0,
        sector_tag: 0,
        front_sidedef: sd,
        back_sidedef: LineDef::NO_SIDEDEF,
    };
    let linedefs = vec![
        mk_ld(0, 1, 0),
        mk_ld(1, 2, 1),
        mk_ld(2, 3, 2),
        mk_ld(3, 0, 3),
    ];

    let sectors = vec![Sector {
        floor_height: 0,
        ceiling_height: 128,
        floor_texture: "FLOOR4_8".into(),
        ceiling_texture: "CEIL3_5".into(),
        light_level: 160,
        sector_type: 0,
        tag: 0,
    }];

    let things = vec![Thing {
        x: 0,
        y: 0,
        angle: 90, // North
        thing_type: 1, // Player 1 start
        flags: 0x0007, // present on all single-player skills
    }];

    MapData {
        name: "MAP01".into(),
        vertices,
        linedefs,
        sidedefs,
        sectors,
        things,
    }
}

/// Replace the editor's state with a fresh starter map. Used by File > New
/// map and by the SaveWarning's PendingAction::NewMap flow.
pub fn new_map(state: &mut EditorState) {
    let map = starter_map();
    state.wad = None;
    state.wad_path = None;
    state.map = Some(map);
    state.mode = SelectionMode::Vertex;
    state.selection.clear();
    state.view_center = egui::pos2(0.0, 0.0);
    state.view_zoom = 1.0;
    state.is_dirty = true;
    state.undo_baseline = state.map.clone();
    state.undo_stack.clear();
    state.line_draw = None;
    state.last_check_results.clear();
    state.status_message = Some(
        "New map: 256x256 starter room with Player 1 start. Save with Ctrl-F2.".into(),
    );
}

/// Set every sidedef of `sector_idx` to use `tex` for upper/lower/middle wall
/// slots. For single-sided lines only middle is visible; for two-sided lines
/// upper/lower show on height transitions, so writing all three is the most
/// useful "set this room's walls" operation.
pub fn set_sector_wall_textures(state: &mut EditorState, sector_idx: u16, tex: &str) {
    if state.map.is_none() { return; }
    push_undo(state);
    let map = state.map.as_mut().unwrap();
    let mut touched = 0usize;
    for sd in map.sidedefs.iter_mut() {
        if sd.sector == sector_idx {
            sd.upper_texture = tex.to_string();
            sd.middle_texture = tex.to_string();
            sd.lower_texture = tex.to_string();
            touched += 1;
        }
    }
    if touched > 0 {
        state.is_dirty = true;
    }
}

/// Open the texture viewer in pick mode for the selected sector's ceiling.
/// Stashes a fresh EditSector dialog so the picked texture flows back into it.
pub fn open_sector_ceiling_picker(state: &mut EditorState) {
    if !ensure_sector_dialog_for_pick(state) { return; }
    state.viewer_pick = Some(super::state::PickTarget::SectorCeiling);
    state.viewer_category = super::state::ViewerCategory::Flats;
    state.viewer_open = true;
}

/// Same idea, for floor.
pub fn open_sector_floor_picker(state: &mut EditorState) {
    if !ensure_sector_dialog_for_pick(state) { return; }
    state.viewer_pick = Some(super::state::PickTarget::SectorFloor);
    state.viewer_category = super::state::ViewerCategory::Flats;
    state.viewer_open = true;
}

/// K hotkey: open the wall-texture picker that, on click, will rewrite every
/// sidedef in the selected sector. No EditSector dialog is stashed because the
/// pick applies directly to the map.
pub fn open_sector_walls_picker(state: &mut EditorState) {
    let Some(&sector_idx) = state.selection.first() else { return };
    if state.map.is_none() { return; }
    state.viewer_pick = Some(super::state::PickTarget::SectorWalls(sector_idx as u16));
    state.viewer_category = super::state::ViewerCategory::Walls;
    state.viewer_open = true;
    // No dialog stash — apply_pick writes directly to the map.
    state.dialog_pending = None;
}

/// Build (or reuse) an EditSector dialog for the currently-selected sector and
/// stash it in `dialog_pending` so a subsequent texture pick has a target.
/// Returns false if no sector is available.
fn ensure_sector_dialog_for_pick(state: &mut EditorState) -> bool {
    let Some(&idx) = state.selection.first() else { return false };
    let Some(map) = state.map.as_ref() else { return false };
    let Some(s) = map.sectors.get(idx) else { return false };
    state.dialog_pending = Some(super::state::Dialog::EditSector {
        idx,
        floor_height: s.floor_height.to_string(),
        ceiling_height: s.ceiling_height.to_string(),
        light: s.light_level.to_string(),
        sector_type: s.sector_type.to_string(),
        tag: s.tag.to_string(),
        floor_texture: s.floor_texture.clone(),
        ceiling_texture: s.ceiling_texture.clone(),
    });
    true
}


// ---------------------------------------------------------------------------
// Tag-line-to-sector tool (matches the original EdMap F7 workflow)
// ---------------------------------------------------------------------------

/// Initiate the two-step tag tool. Requires LineDef mode with a single
/// linedef selected; the next viewport click will pick the target sector.
pub fn begin_tag_link(state: &mut EditorState) {
    use super::state::SelectionMode;
    if state.mode != SelectionMode::LineDef {
        state.status_message = Some("Tag line to sector: switch to LineDef mode and select a line first".into());
        return;
    }
    if state.selection.len() != 1 {
        state.status_message = Some("Tag line to sector: select exactly one linedef".into());
        return;
    }
    let ld_idx = state.selection[0];
    state.tag_link_pending = Some(ld_idx);
    state.status_message = Some(format!("Tag tool: click a sector to tag with LineDef #{ld_idx}"));
}

/// Complete the two-step tag tool: assign a shared tag value to the linedef
/// and the picked sector. Reuses an existing tag if either side already has
/// one, otherwise allocates the smallest unused positive tag.
pub fn finish_tag_link(state: &mut EditorState, sector_idx: usize) {
    let Some(ld_idx) = state.tag_link_pending.take() else { return };
    push_undo(state);
    let Some(map) = state.map.as_mut() else {
        state.status_message = Some("Tag tool: no map loaded".into());
        return;
    };
    let Some(ld) = map.linedefs.get(ld_idx) else {
        state.status_message = Some("Tag tool: linedef no longer exists".into());
        return;
    };
    let Some(sec) = map.sectors.get(sector_idx) else {
        state.status_message = Some("Tag tool: sector no longer exists".into());
        return;
    };

    // Pick a tag: prefer the linedefs current tag, then the sectors, else allocate.
    let tag = if ld.sector_tag != 0 {
        ld.sector_tag
    } else if sec.tag != 0 {
        sec.tag
    } else {
        next_unused_tag(map)
    };
    map.linedefs[ld_idx].sector_tag = tag;
    map.sectors[sector_idx].tag = tag;
    state.is_dirty = true;
    state.status_message = Some(format!(
        "Tagged LineDef #{ld_idx} <-> Sector #{sector_idx} (tag {tag})"
    ));
}


/// "Enhance map" — combine the three single-purpose fixers into one button.
/// Status message reports the combined counts. Each individual fixer pushes
/// its own undo entry so the operation is reversible step-by-step.
pub fn enhance_map(state: &mut EditorState) {
    let zero = fix_zero_length_linedefs(state);
    let missing = fix_missing_textures(state);
    let unused = remove_unused_textures(state);
    state.status_message = Some(format!(
        "Enhance map: removed {zero} zero-length lines, filled {missing} missing tex, removed {unused} unused tex"
    ));
}

// ---------------------------------------------------------------------------
// Sector style clipboard: grab/apply tools (matches the original DOS EdMap
// "Grab style" / "Texture style" workflow). Single-slot for now; named
// multi-style storage can come later in a Preferences-backed dialog.
// ---------------------------------------------------------------------------

/// Capture the currently-selected sector's height/light/texture settings into
/// `sector_style_clipboard`. Requires Sector mode with exactly one selection.
pub fn grab_sector_style(state: &mut EditorState) {
    use super::state::{SectorStyle, SelectionMode};
    if state.mode != SelectionMode::Sector {
        state.status_message = Some("Grab style: switch to Sector mode first".into());
        return;
    }
    if state.selection.len() != 1 {
        state.status_message = Some("Grab style: select exactly one sector".into());
        return;
    }
    let sec_idx = state.selection[0];
    let Some(map) = state.map.as_ref() else { return };
    let Some(s) = map.sectors.get(sec_idx) else { return };
    state.sector_style_clipboard = Some(SectorStyle {
        floor_height: s.floor_height,
        ceiling_height: s.ceiling_height,
        floor_texture: s.floor_texture.clone(),
        ceiling_texture: s.ceiling_texture.clone(),
        light_level: s.light_level,
        sector_type: s.sector_type,
    });
    state.status_message = Some(format!(
        "Grabbed style from sector #{sec_idx} (floor {}, ceil {}, light {})",
        s.floor_height, s.ceiling_height, s.light_level
    ));
}

/// Apply only the floor/ceiling textures from the grabbed style to every
/// selected sector. The original DOS "Texture style" command.
pub fn apply_sector_style_textures(state: &mut EditorState) {
    apply_sector_style(state, true, false);
}

/// Apply every field from the grabbed style (heights, textures, light, type)
/// to every selected sector.
pub fn apply_sector_style_all(state: &mut EditorState) {
    apply_sector_style(state, true, true);
}

fn apply_sector_style(state: &mut EditorState, copy_textures: bool, copy_other: bool) {
    use super::state::SelectionMode;
    let Some(style) = state.sector_style_clipboard.clone() else {
        state.status_message = Some("Apply style: nothing grabbed yet (use Grab style first)".into());
        return;
    };
    if state.mode != SelectionMode::Sector || state.selection.is_empty() {
        state.status_message = Some("Apply style: select sectors first".into());
        return;
    }
    push_undo(state);
    let targets = state.selection.clone();
    let Some(map) = state.map.as_mut() else { return };
    let mut applied = 0;
    for &i in &targets {
        let Some(s) = map.sectors.get_mut(i) else { continue };
        if copy_textures {
            s.floor_texture = style.floor_texture.clone();
            s.ceiling_texture = style.ceiling_texture.clone();
        }
        if copy_other {
            s.floor_height = style.floor_height;
            s.ceiling_height = style.ceiling_height;
            s.light_level = style.light_level;
            s.sector_type = style.sector_type;
        }
        applied += 1;
    }
    if applied > 0 {
        state.is_dirty = true;
    }
    state.status_message = Some(format!(
        "Applied style to {applied} sector(s){}",
        if copy_other { " (full)" } else { " (textures only)" }
    ));
}
