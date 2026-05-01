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
