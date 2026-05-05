// ABOUTME: Global keybindings dispatch — maps every menu hotkey from the UX spec to handle_command.
// ABOUTME: Plus mode keys (V/L/S/T) and zoom keys (+ / -) that mirror the original feel.

use eframe::egui::{self, Key, Modifiers};

use super::commands;
use super::state::{EditorState, SelectionMode};

/// Map of menu-style hotkey strings (from items_for) to (Modifiers, Key).
/// Stays in lockstep with the labels rendered in menu rows so users can't
/// see a hotkey on screen that we don't actually handle.
fn parse_hotkey(s: &str) -> Option<(Modifiers, Key)> {
    let mut mods = Modifiers::NONE;
    let mut rest = s;
    loop {
        if let Some(r) = rest.strip_prefix("Ctrl-") {
            mods.ctrl = true;
            mods.command = true;
            rest = r;
        } else if let Some(r) = rest.strip_prefix("Shift-") {
            mods.shift = true;
            rest = r;
        } else if let Some(r) = rest.strip_prefix("Alt-") {
            mods.alt = true;
            rest = r;
        } else {
            break;
        }
    }
    let key = match rest {
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "Ins" => Key::Insert,
        "BkSp" => Key::Backspace,
        "Num Lock" => return None, // egui has no NumLock; skip rather than misbind.
        ">" => Key::Period,        // physical key without shift; we'll allow shift too
        "<" => Key::Comma,
        "A" => Key::A, "B" => Key::B, "C" => Key::C, "D" => Key::D, "E" => Key::E,
        "F" => Key::F, "G" => Key::G, "H" => Key::H, "I" => Key::I, "J" => Key::J,
        "K" => Key::K, "L" => Key::L, "M" => Key::M, "N" => Key::N, "O" => Key::O,
        "P" => Key::P, "Q" => Key::Q, "R" => Key::R, "S" => Key::S, "T" => Key::T,
        "U" => Key::U, "V" => Key::V, "W" => Key::W, "X" => Key::X, "Y" => Key::Y,
        "Z" => Key::Z,
        _ => return None,
    };
    Some((mods, key))
}

/// Walk every menu and every item; if the item has a hotkey and the hotkey is
/// pressed this frame, dispatch the command. Single source of truth: the menu spec.
pub fn dispatch(ctx: &egui::Context, state: &mut EditorState, tx: &std::sync::mpsc::Sender<crate::app::AsyncCommand>) {
    let menus = super::menu::MENU_ORDER;
    let mut to_run: Option<(&'static str, &'static str)> = None;
    ctx.input(|input| {
        for menu in menus {
            for (label, hotkey) in super::menu::items_for(menu) {
                if hotkey.is_empty() {
                    continue;
                }
                let Some((mods, key)) = parse_hotkey(hotkey) else { continue };
                // Match modifier set exactly (Cmd is treated like Ctrl on macOS).
                if input.modifiers.matches_logically(mods) && input.key_pressed(key) {
                    to_run = Some((*menu, *label));
                    return;
                }
            }
        }

        // Q toggles 3D walk/fly view. Bound outside modifier-gated block so it works anywhere.
        if !any_modifier(&input.modifiers) && input.key_pressed(Key::Q) {
            super::view3d::toggle(state);
        }
        // Esc exits 3D mode immediately, before the regular Esc handler runs.
        if state.view3d_open && input.key_pressed(Key::Escape) {
            super::view3d::toggle(state);
        }

        // Mode keys outside the menu spec — keyboard-first feel from the original.
        // Suppressed while the 3D view is active so WASD/etc. drive the camera, not editing.
        if !any_modifier(&input.modifiers) && !state.view3d_open {
            // Sector mode + selected sector: digit and letter keys edit fields
            // instead of switching modes (matches EdMap's numbered shortcuts).
            // Tab and V still work for mode switching when this is active.
            let sector_hotkey_active =
                state.mode == SelectionMode::Sector && !state.selection.is_empty();
            if sector_hotkey_active {
                if input.key_pressed(Key::Num2) {
                    commands::open_sector_ceiling_picker(state);
                } else if input.key_pressed(Key::Num4) {
                    commands::open_sector_floor_picker(state);
                } else if input.key_pressed(Key::K) {
                    commands::open_sector_walls_picker(state);
                } else if input.key_pressed(Key::Num1)
                    || input.key_pressed(Key::Num3)
                    || input.key_pressed(Key::Num5)
                    || input.key_pressed(Key::Num6)
                    || input.key_pressed(Key::Num7)
                {
                    commands::open_properties(state);
                }
            }
            // V always switches to Vertex mode — keeps a letter escape hatch
            // when sector_hotkey_active blocks the digit-mode-switch keys.
            if input.key_pressed(Key::V) {
                commands::set_mode(state, SelectionMode::Vertex);
            }
            if !sector_hotkey_active && input.key_pressed(Key::Num1) {
                commands::set_mode(state, SelectionMode::Vertex);
            }
            if !sector_hotkey_active && input.key_pressed(Key::Num2) {
                commands::set_mode(state, SelectionMode::LineDef);
            }
            if !sector_hotkey_active && input.key_pressed(Key::Num3) {
                commands::set_mode(state, SelectionMode::Sector);
            }
            if !sector_hotkey_active && input.key_pressed(Key::Num4) {
                commands::set_mode(state, SelectionMode::Thing);
            }
            if input.key_pressed(Key::Tab) {
                let next = match state.mode {
                    SelectionMode::Vertex => SelectionMode::LineDef,
                    SelectionMode::LineDef => SelectionMode::Sector,
                    SelectionMode::Sector => SelectionMode::Thing,
                    SelectionMode::Thing => SelectionMode::Vertex,
                };
                commands::set_mode(state, next);
            }
            if input.key_pressed(Key::Escape) {
                if state.viewer_open {
                    super::viewer::cancel_pick(state);
                    state.viewer_open = false;
                } else if state.dialog.is_some() {
                    state.dialog = None;
                } else if state.line_draw.is_some() {
                    state.line_draw = None;
                    state.status_message = Some("Line-draw cancelled".into());
                } else if state.tag_link_pending.is_some() {
                    state.tag_link_pending = None;
                    state.status_message = Some("Tag tool cancelled".into());
                } else {
                    state.open_menu = None;
                    state.status_message = None;
                }
            }
            // Enter on a selection → open Properties dialog. Skipped while a
            // dialog is already open so it doesn't fight TextEdit submit.
            if input.key_pressed(Key::Enter) && state.dialog.is_none() && !state.viewer_open {
                commands::open_properties(state);
            }
            // F = flip selected linedef(s) — only meaningful in LineDef mode.
            if input.key_pressed(Key::F) {
                commands::flip_selected_linedefs(state);
            }
            // A = auto-align textures along a chain (LineDef mode).
            if input.key_pressed(Key::A) && state.mode == SelectionMode::LineDef {
                commands::auto_align_textures(state);
            }
            // C = clear selection.
            if input.key_pressed(Key::C) {
                state.selection.clear();
            }
            // [ / ] cycle grid size through DOOM's standard powers-of-two.
            if input.key_pressed(Key::OpenBracket) {
                state.grid_size = prev_grid_size(state.grid_size);
            }
            if input.key_pressed(Key::CloseBracket) {
                state.grid_size = next_grid_size(state.grid_size);
            }
        }
        // Ctrl-C / Ctrl-V = copy / paste selection.
        if (input.modifiers.ctrl || input.modifiers.command) && input.key_pressed(Key::C) {
            commands::copy_selection(state);
        }
        if (input.modifiers.ctrl || input.modifiers.command) && input.key_pressed(Key::V) {
            commands::paste_clipboard(state);
        }
        // Ctrl-Z = pop multi-level undo stack.
        if (input.modifiers.ctrl || input.modifiers.command) && input.key_pressed(Key::Z) {
            commands::pop_undo(state);
        }
        // PgUp/PgDn = sector ceiling ±8; Shift = floor; Ctrl = light ±16.
        if input.key_pressed(Key::PageUp) {
            if input.modifiers.ctrl || input.modifiers.command {
                commands::adjust_selected_light(state, 16);
            } else {
                commands::adjust_selected_heights(state, 8, input.modifiers.shift);
            }
        }
        if input.key_pressed(Key::PageDown) {
            if input.modifiers.ctrl || input.modifiers.command {
                commands::adjust_selected_light(state, -16);
            } else {
                commands::adjust_selected_heights(state, -8, input.modifiers.shift);
            }
        }

        // Zoom keys — keep + / - working regardless of shift.
        if input.key_pressed(Key::Plus) || input.key_pressed(Key::Equals) {
            state.view_zoom = (state.view_zoom * 1.25).min(16.0);
        }
        if input.key_pressed(Key::Minus) {
            state.view_zoom = (state.view_zoom / 1.25).max(0.01);
        }
    });

    if let Some((menu, label)) = to_run {
        super::menu::handle_command(state, menu, label, tx);
    }
}

fn any_modifier(m: &Modifiers) -> bool {
    m.ctrl || m.alt || m.shift || m.command || m.mac_cmd
}

/// DOOM-standard grid size sequence; cycle through with `[` / `]`.
const GRID_STEPS: &[i32] = &[1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024];

fn prev_grid_size(current: i32) -> i32 {
    let idx = GRID_STEPS.iter().position(|&g| g >= current).unwrap_or(GRID_STEPS.len());
    GRID_STEPS[idx.saturating_sub(1).max(0)].max(1)
}

fn next_grid_size(current: i32) -> i32 {
    let idx = GRID_STEPS.iter().position(|&g| g > current).unwrap_or(GRID_STEPS.len() - 1);
    GRID_STEPS[idx.min(GRID_STEPS.len() - 1)]
}
