// ABOUTME: Editor commands invoked by menu clicks and keybindings.
// ABOUTME: Each function mutates EditorState; keep them small and pure-ish.

use super::state::{EditorState, SelectionMode};

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
