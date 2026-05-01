# EdMap-rebuild changelog

Chronological log of the tracks completed during the 2026 rebuild.
Style mimics the original EdMap.txt's "new to v1.X" notes: terse,
feature-oriented, no fluff.

## Track 14 — Texture-picker integration with property editor

- Pick button next to Sector floor/ceiling texture fields opens the F10
  viewer in pick mode (Flats tab default)
- `state.viewer_pick: Option<PickTarget>` + `state.dialog_pending` for the
  stash-and-restore flow
- Esc / [X] cancel restores the dialog with edits intact
- 19 tests pass

## Track 13 — Per-object property editor

- Press Enter on selected object → mode-aware Edit dialog
- Vertex (x, y); LineDef (flags, special, tag, sidedef indices);
  Sector (heights, light, type, tag, textures); Thing (x, y, angle, type, flags)
- Texture names clamped to 8 ASCII chars uppercase
- 18 tests pass

## Track 12 — Door auto-construction (Alt-D)

- Closes the selected sector (ceiling = floor)
- Walks 2-sided boundary linedefs and applies door-action special
  (1/26/27/28/117 depending on key + speed)
- Refuses if the sector has no 2-sided boundaries
- 17 tests pass

## Track 11 — Polygon + Stairs auto-constructions

- Polygon (Ctrl-P): N-gon sector at cursor with CCW winding (3..64 sides)
- Stairs (Alt-S): N rectangular sectors stacked along N/E/S/W direction,
  each step's floor stepped by `rise`
- Both set is_dirty + select the new sector
- 16 tests pass

## Track 10 — Add/split (Ins)

- Vertex mode: insert vertex at cursor, snap to grid
- LineDef mode: split selected linedef at cursor projection (clamped to 5..95% to avoid degenerate splits)
- Thing mode: insert default Player 1 start at cursor
- New linedef inherits all flags + sidedef refs from the original
- 15 tests pass

## Track 9 — Map check engine + Error List

- 10 detectors: zero/short/long LineDef length, 2S flag coherence,
  missing 2-sided textures, missing/multiple Player 1 start, no exit
- F5 Quick check, Ctrl-F5 Check all, Ctrl-L reopen
- ErrorList dialog with Previous / Next / Goto / Close
- Goto sets selection + centers viewport
- 13 tests pass

## Track 8 — Save warning + Undo from last save

- Quit / New map / Open map all check is_dirty and queue behind
  Dialog::SaveWarning with Yes/No/cancel
- PendingAction enum carries the deferred action through the dialog
- Edit > Undo from last save restores the snapshot captured on load/save

## Track 7 — Robust PWAD writer

- `wad/write.rs`: serialize_map_lumps + build_pwad
- Two output modes: fresh (one map only) and preserve (keep all other lumps)
- F2 saves to current path (rejects IWAD); Ctrl-F2 native save-as picker
- Round-trip + preserve unit tests

## Track 6 — Drag-to-move + Backspace-delete + dirty tracking

- Primary-button drag translates selection (per-mode) with snap residual
- Auto-select on drag-start so click-then-drag isn't required
- Backspace deletes selected; Vertex delete refuses if linedefs reference it
- ` *` marker in MAP info box when is_dirty

## Track 5 — Viewport click-to-select

- `app/hittest.rs`: nearest_vertex / linedef / thing / sector_under
- Hover preview in red; selected objects redrawn thicker + red
- Sector mode highlights all boundary linedefs of selected sectors

## Track 4c — Sidebar parity

- Numbered 1-9 LineDef flag list with •/○ markers
- (no action) / length / SD#:offset rows
- Texture name rows: U/M/L for front sidedef, N/B/R for back
- Per-mode panels: Vertex (x/y), Sector (heights/light/type/tag/textures),
  Thing (x/y/angle/type/flags)

## Track 4b — Texture viewer (F10)

- `wad/texture.rs`: Patch decoder (full DOOM patch_t with transparent posts),
  Flat (64×64), TextureImage::compose
- `app/textures.rs`: TextureBank lazy-decodes via egui::TextureHandle cache
- Walls / Floors-Ceilings / Sprites tabs; paged tile grid

## Track 4a — Core editor commands + dialogs

- About / Map Information / System Information dialogs
- Snap/grid editor, Goto object, WadList, OpenMapPicker, Notice
- Replaced 9 menu items' "not implemented" toasts with real handlers

## Track 3c — Menus + keybindings

- All 9 cascading menus rendered live with verbatim labels + hotkeys
- Single keybindings dispatcher walks the menu spec for hotkey routing
- 1/2/3/4/Tab mode keys, Esc cancellation, +/- zoom

## Track 3b — Pivoted to egui

- Replaced Tauri+React+Vite with eframe/egui
- Theme: full 16-color VGA palette, Turbo-Vision-style bevels, no rounding
- Sidebar with title, menu list, MAP info box, status, mode tabs, compass
- Map viewport with grid, origin, vertex/linedef/thing rendering

## Track 3a — Initial WAD parser in Rust

- `wad/{header,lump,map,texture,error}.rs`
- Map enumeration, all 5 map lump record types
- 2 unit tests against synthetic PWAD

## Track 2 — UX spec from strings

- Extracted full strings table from EDMAPSYS.EXE
- Documented all 9 menus, 17 dialog families, error catalog, internal data files
- `specs/001-edmap-nextgen/ux-spec.md`

## Track 1 — Initial RE pass

- Established radare2 + DOSBox-X workflow
- Mapped binary anatomy: Borland TP MZ, DGROUP base ~0xCD8E
- Identified 3 functions in the LineDef check chain
- 8 open questions documented in `specs/001-edmap-nextgen/re-notes.md`
