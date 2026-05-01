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
