# EdMap Next-Gen — UX Specification

Derived from string-table extraction of `EDMAPSYS.EXE` (EdMap v1.40, 1994, Jeff Rabenhorst, araya@wam.umd.edu) and the original DOSBox screenshot. This document is the source-of-truth for the React/Tauri rebuild's UX surface. Behaviors not derivable from strings or public DOOM specs are flagged for Track 1 (RE).

## 1. Application identity

- **Title bar / about**: `EdMap v1.40` — `DOOM-I/-II/HERETIC map editor` — `1994 Jeff Rabenhorst`
- **Supported games**: `DOOM`, `DOOM2`, `HERETIC`
- **Required environment (original)**: VGA 640x480, mouse driver, `FILES=20` in `CONFIG.SYS`, optional XMS
- **Companion executables**: `EDMAP.EXE` (launcher), `EDMAPSYS.EXE` (main), `EDMAPCFG.EXE` (config)

## 2. Top-level layout (matches screenshot)

```
+----------------+--------------------------------------------------+
| Sidebar (140px)| Map viewport (rest)                              |
|  - Title       |  - Black/dark-blue background                    |
|  - Menu list   |  - Grid overlay (toggle: Display > Grid on/off)  |
|  - Map info box|  - Vertices, LineDefs, Sectors, Things rendered  |
|  - Status box  |  - Origin marker (toggle: Ctrl-O)                |
|  - Selection   |                                                  |
|    flags box   |                                                  |
|  - Texture box |                                                  |
+----------------+--------------------------------------------------+
```

Color palette (from CSS, confirmed against screenshot):
- Background: `#00002a` (vga-blue-dark) or pure black inside viewport
- Sidebar bg: dark blue panel; selected MAP-NAME box uses `#0000aa`
- Foreground: `#aaaaaa` (gray) text, `#ffffff` (white) primary, `#ffff54` (yellow) emphasis, `#54ff54` (green) ok-status, `#ff5454` (red) error, `#54ffff` (cyan) headers
- Lines: high-contrast white/red/cyan strokes on black

## 3. Menu tree (verbatim from binary)

Top-level (left sidebar, vertical):

| # | Menu | Hotkey style |
|---|------|-------------|
| 1 | Info | (cascades right) |
| 2 | File (map) | |
| 3 | WAD list | |
| 4 | Edit | |
| 5 | Map utilities | |
| 6 | Sectors | |
| 7 | Automatic | |
| 8 | Display | |
| 9 | Check | |

### 3.1 Info
- About EdMap
- Help — `F1`
- Calculator — `Num Lock`
- Map Information
- System Information
- Load config file
- Edit config (EDMAPCFG)
- Preferences

### 3.2 File (map)
- New map
- Open map file — `F3`
- Save map data — `F2`
- Load PWAD map — `Shift-F3`
- Rename map
- Build & save map — `F9`
- Alternate build — `Alt-F9`
- Play map — `Ctrl-F9`
- Quit to DOS — `Alt-X`

### 3.3 WAD list
- List WADs — `F4`
- Save as PWAD... — `Ctrl-F2`
- Add PWAD file — `Ctrl-F4`
- Remove PWAD
- Write ADD file

### 3.4 Edit
- Add/split — `Ins`
- Delete/merge — `BkSp`
- Undo from last save
- Shift object
- Find objects — `Ctrl-F`
- Goto object — `Ctrl-G`
- Next object — `>`
- Previous object — `<`
- Tag line to sector — `F7`

### 3.5 Map utilities
- Shift Map (X/Y/Z)
- Expand/reduce map
- Light adjustment
- Texture replace

### 3.6 Sectors
- Polygon — `Ctrl-P`
- Rotate — `R`
- Size — `Z`
- Texture style — `Alt-F8`
- Edit styles — `Ctrl-F8`
- Grab style — `Shift-F8`
- Align textures (X,Y) — `F8`
- Configure align

### 3.7 Automatic (one-click constructions)
- Lift — `Alt-L`
- Door — `Alt-D`
- Stairs — `Alt-S`
- Teleporter — `Alt-T`

### 3.8 Display
- Enhance map — `Ctrl-E`
- Full screen — `Ctrl-S`
- Snap/grid
- Grid on/off
- Origin on/off — `Ctrl-O`
- Center map
- Viewer — `F10`
- Refresh display

### 3.9 Check
- Error list — `Ctrl-L`
- Quick check — `F5`
- Check all — `Ctrl-F5`
- Textures
- Associations
- Heights/widths
- LineDefs
- Begin & end
- Sector integrety [sic — preserve typo from original for fidelity]

## 4. Sidebar info panel (live status)

Order top-to-bottom in the sidebar below the menu list:

### 4.1 Map name box (yellow on dark blue)
- Line 1: `MAP 1` (or `untitled` for new map; format `ExMy` for DOOM I/Heretic, `MAPxx` for DOOM II)
- Line 2: `original map` (or PWAD source name)
- Line 3: `214.82k free` (memory free indicator, green)

### 4.2 Help hint
- `press F1`
- `for help`

### 4.3 Status block
- `G:` grid size, `S:` snap size — formatted as `G: 64  S: 8` (string fragments `G:``` and `S:``` confirm this)
- `Z: 1.00x` — current zoom
- `-912, 1400` — current cursor map coordinates (X, Y)

### 4.4 Selection mode tabs
Four-tab selector (only one active at a time, matches screenshot underline on `Ld`):
- `Vx` — Vertex
- `Ld` — LineDef (default per screenshot)
- `Se` — Sector
- `Th` — Thing

Counter below: `338/370` — `selected / total` for the active mode.

### 4.5 Selection flags (LineDef mode shown in screenshot)
Bullet style: `●` (filled, active) / `○` (hollow, inactive) / `⁞` (mixed across multi-select).

LineDef flags (from binary):
- block all
- block enemy
- two-sided
- upper pegged
- lower pegged
- secret wall
- block sound
- never map
- start on map

### 4.6 Action / length panel (LineDef mode)
- `(no action)` or action descriptor
- `length 840.0` — current LineDef length (real units)
- `493: 0,0` — texture offsets (X, Y) shown as `xxx, yyy` per binary

### 4.7 Texture preview tile
- `M: MODWALL2` — middle texture name from screenshot
- For 2-sided: shows `U:` upper / `M:` main / `L:` lower
- Internal palette/preview tile rendered with PLAYPAL palette

## 5. Mode-specific selection flags

(From binary — populated when respective mode is active.)

### 5.1 Thing flags
- skills 1 & 2
- skill 3
- skills 4 & 5
- deaf guard
- multi only

### 5.2 Sector fields
- ceiling height
- floor height
- light (0–255)
- type (sector type)
- tag/trigger number

### 5.3 SideDef fields
- texture X-offset
- texture Y-offset
- width / above / main / below texture-space sizes

## 6. Dialogs (modal panels)

Each entry: title, key fields, OK/cancel labels. All quote-marks below are taken from the binary verbatim. Italics indicate behavior to verify in DOSBox.

### 6.1 Quit
- `Quit EdMap` — `Are you sure you want to quit EdMap?` — `Quit` / `Cancel`

### 6.2 Open Map
- `Open Map` — `Select an Episode/Mission map:` (DOOM/Heretic) or `Select a map:` + `(Enter a map number)` (DOOM II)
- Buttons: `reload`, `cancel`, `other`

### 6.3 Save warning / overwrite
- `save as..` — `Saving... seems to have more than one map. Saving to this file will erase all other maps. To save to another file, select W AD list, S ave as PWAD... (Ctrl-F4). Do you still want to save to this file?`
- `overwrite on save` — `This file already exists. Do you wish to overwrite it?`

### 6.4 PWAD list
- Title: `Active PWADs` / `list WADs`
- Per-row info: `(Internal-WAD data file)` / Sprites / Flats / Sounds / Music / Patches / Textures / Patch-list counts
- Buttons: `Information`, `Remove`
- `WAD List\Cannot remove the IWAD.`

### 6.5 PWAD load
- `Enter additional PWAD...` — `**CANCEL`
- Errors: `PWAD Load\... does not exist.`, `Error in PWAD: ...`

### 6.6 Palette adjustment
- `PALETTE ADJUSTMENT` — `Palette` / `cancel` / `adjust`
- `Adjust color` — slider per channel

### 6.7 Checking options
- `CHECKING OPTIONS` — `Checking options` / `cancel`
- Toggles:
  - `Long-Wall-Error`
  - `Bad LineDefs`
  - `Things in sector`
  - `Things association`
  - `Other assoc/triggers`
- Texture sub-toggles:
  - `Missing/HOM`
  - `Short/Tutti-Fruitti`
  - `Bad otherwise`

### 6.8 Snap options
- `SNAP OPTIONS` — `Snap options` / `cancel`
- Categories with per-item enable:
  - **When inserting**: `LineDef break (vertex)`, `New 64x64 sector`, `New thing-objects`
  - **When constructing**: `Polygons`, `Stairs`
  - **When dragging**: `Thing-objects`, `Multiple Thing-objects`, `Multiple Vertices`, `Rotating`, `Resizing`

### 6.9 Preferences
- `Preferences` — `cancel`
- Sections:
  - **Play map**: `Use panel? Skill?`
  - **Mouse**: `X-Sensitivity`, `Y-Sensitivity`, `Double-Speed`, `Allow move? (Int.33h,04h)`
  - **Editor**: `Verify sector operations?`, `Pick textures from viewer?`, `Num Lock for calculator?`, `Always save (when asked)?`, `Timed save: minutes`
  - **Additional**: `Checking options`, `Snap options`, `Palette`

### 6.10 Automatic constructions
Each opens a parametric dialog producing a fully-formed map element.

- **Lift** (`Lift`): Repeatable?, Side texture, Use floor?, Floor texture, Fast lift?
- **Door** (`Door`): Door texture, Sill texture, Bottom texture, Close?, Moving sill?, Fast door?, key options (`keyless`/`blue key`/`yellow key`/`green key`/`red key`), durations (`6 sec`, `Stay open`, `Moving sill`, `Fixed sill`, `Fast`, `Normal`)
- **Stairs**: Step top texture, Step side texture, Step rise size, Step depth, Step width, ceiling height, number of steps, Staircase length, direction (`North`/`East`/`South`/`West`), `rising/triggered?`
- **Teleporter**: `two way?`, `first pad dir`, `second pad dir`, `(floor) texture`, `texture ceiling?`, `sector type`
- **Polygon**: `number of sides`, `radius (vertices)`, `Place center of polygon`. `Enter -1 to make a new sector`.

### 6.11 Texture replace
- `Texture Replace` — `cancel` / `specify`
- Sub-modes: `Replace specified texture`, `Select wall textures`, `Select floor/ceiling textures`
- Confirm: `Are you sure you want to do this?`
- Result: `N textures replaced.`

### 6.12 Map shift / expand / light adjust
- `Map shift` — X / Y / Z (height) deltas
- `Map expand` — X / Y / Z scale factors
- `Map light adjustment` — `A: Amplify`, `B: Brighten`, formula: `new light = old light × A/100 + B`, clamped `[0..255]`

### 6.13 Find / Goto
- `Find Objects`: by trigger/tag, action, kind of action, sector type, thing type, thing kind. Notes: clears multi-selection.
- `Goto vertex... (0-N)`, `Goto LineDef...`, `Goto sector...`, `Goto thing...`

### 6.14 Sector edit
- `SideDef`, `Sector (0-N)...`, `Ceiling height...`, `Floor height...`, `Light (0-255)...`, `Trigger warning`, `Trigger number...`

### 6.15 Play-map options
- `Play-map options`: nomonsters / respawn / fast / nosound / nomusic / nosfx / turbo / deathmatch / altdeath
- Skill: `1: I'm too young to die.` / `2: Hey, not too rough.` / `3: Hurt me plenty.` / `4: Ultra-violence.` / `5: Nightmare!`
- Errors: `Run\Missing Start-1 thing.`, `Run\At least 4 DeathMatch-starts required.`

### 6.16 Write ADD file
- `Create` confirmation: `These PWADs will be added to the PWAD list at startup, and run with DOOM during Play-map. Are you sure you want to do this?`
- Result: `ADD file\... saved. When EdMap is loaded these files will be added to the PWAD list.`

### 6.17 Edit sector styles
- `Edit sector styles` — requires one-sided LineDef selected
- `Add style`, `Delete style`, `Cancel`
- Move/swap records, validation: `Sector styles\Invalid style name`, `Sector styles\Cannot add any more styles.`

## 7. Texture viewer (F10)

Two viewers, paged tile grid:
- `Choose a Wall Texture` — `Click 2nd for more`, `Press F10 for viewer`. Empty entry: `(none)`.
- `Choose a Floor/Ceiling Texture` — `("..." = animated image)`. Animations include flats like `BLOOD?1234`, `NUKAGE?1234`, `LAVA?1234`, `SLIME0?1234`, `FWATER?1234`.
- Sub-views: Wall / Floor / Sprite / Patch / 1 frame / N frames
- Errors: `Error in file: "PTX-6.DAT".`, `Viewer\Error reading ...`, `alloc:Exceeded maximum images in viewer!`

## 8. Check / error reporting

`error list` panel — list of detected issues with `next` / repeat navigation.

Detector messages (verbatim, used as labels):
- **No 1S main texture** — `No main texture on this one-sided linedef. (Hall of Mirrors effect/HOM).`
- **Transparent texture on 1S main**
- **Bad main texture size** — `tutti-fruitti`
- **Missing textures on 2S** / **Missing texture on 2S**
- **Transparent upper/main/lower texture**
- **Multi-patch on 2S main** — `This 2-sided LineDef has a multi-patch main texture.`
- **Warning: hanging texture on 2S main** — `This texture is too short to fill its space. 2S main textures do not tile; it will hang from the ceiling.`
- **2S bit set, no 2nd SideDef** / **2nd SideDef, 2S bit is off**
- **Bad LineDef length (0)** / **Bad LineDef length (short)** — `This LineDef has no length.` / `This LineDef is too short.`
- **Warning: long LineDef** — threshold *unknown* → Track 1 RE
- **Too many scrolling walls**
- **Manual-Door on 1S LineDef**
- **No tag for LineDef action**
- **No sectors match tag number**
- **Missing teleporter destination**, **No teleporter sector destination.**, **Mult teleporter sector destinations.**, **Multiple teleporter destinations.**
- **Key absent in difficulty levels but no door** / **Door but no key** (blue/yellow/green keys)
- **Missing SpawnSpot, spawnspots!**
- **Thing not in a sector**
- **Sector too short for Thing height**
- **Multiple Start-N things**, **Missing Start-x things**, **Missing deathmatch starts**, **Too many deathmatch starts**
- **No exit** — `This map has no exit`
- Completion: `*Check complete\No errors detected.` etc per category

Counter format: `N error[s] found:`, `(Map)`, `repeat`, `once`, `use..`, `(keep)`.

## 9. Calculator

Simple expression evaluator: `Calculator`. Operators visible: `+ - * /`, exponent (`?</u;` likely `^`). Toggle: Num Lock per Preferences.

## 10. Status / messaging strings

- `Loading...`, `Saving`, `Done.`, `Hit a key`, `(CTRL-C ABORT)`
- `Building map to run in {DOOM/DOOM2/HERETIC}`
- `Memory to disk/XMS swapping disabled.`
- Error sentinel: `**ERROR`

## 11. Internal data files (load on startup)

These need to be either bundled with the Rust app or replaced by hard-coded data:
- `MOUSEPIC.DAT` — mouse cursor pictures (bundle as static data)
- `EDMAP.CFG` — config file (we can use TOML/JSON instead; preserve EDMAPCFG semantics)
- `PTX-6.DAT`, `ptx2s.dat`, `ptx2s-8.dat` — viewer tile data
- `TEXTINDX.DAT`, `TEXTLIST.TXT` — text strings
- `HELPINDX.DAT`, `HELP.TXT` — help text
- `ACTSLIST.DAT`, `ACTIONS.TXT` — LineDef action catalog
- `FLORLST*.DAT`, `FLORLIST.TXT` — flat (floor texture) catalog
- `WALLLST*.DAT` — wall catalog
- `PATCIDX*.DAT` — patch index
- `SESTYLE*` — sector styles
- `THINGS.TXT`, `SETYPES.TXT` — thing types and sector types definitions

Format of these files is *unknown* → Track 1 RE *only if* we can't reconstruct the catalogs from public DOOM specs (we likely can: DOOM action types and thing types are fully documented in the DOOM specs).

## 12. Open questions for Track 1 (RE)

1. Exact threshold for "long LineDef" warning.
2. Texture-align algorithm (`F8` Align textures (X,Y), `Alt-F8` Texture style).
3. Sector resize/rotate math (`R` / `Z`) — pivot, scale center, rounding.
4. Polygon vertex placement formula (`Ctrl-P`).
5. Stairs auto-construct stepping math.
6. ADD file format (custom EdMap concept).
7. Sector style file format (`SESTYLE*`).
8. `MOUSEPIC.DAT` binary layout (only needed if we want pixel-perfect cursor).

## 13. Out-of-scope for v1

- Direct DOOM execution (`Play-map`) — we don't ship a DOOM engine. Provide an "Export to PWAD + run with external port" affordance instead.
- 16-bit BGI graphics (`EGAVGA.BGI`, `IBM8514`, `HERC`, `PC3270`, fonts `TRIP`/`LITT`/`SANS`/`GOTH`/`SCRI`/`SIMP`/`TSCR`/`LCOM`/`EURO`) — replace with web fonts mimicking VGA bitmap aesthetic.
- XMS / disk swapping — modern systems have plenty of RAM.

## 14. Reference: DOOM lump types in WAD

(Public spec, not derived from RE.)

Per-map lumps in order: `THINGS`, `LINEDEFS`, `SIDEDEFS`, `VERTEXES`, `SEGS`, `SSECTORS`, `NODES`, `SECTORS`, `BLOCKMAP`, `REJECT`. Asset markers: `S_START`/`S_END` (sprites), `F_START`/`F_END` (flats), `P_START`/`P_END` (patches), `Px_START`/`Px_END` (per-episode patches), `Fx_START`/`Fx_END`. Shared lumps: `PNAMES`, `TEXTURE1`, `TEXTURE2`, `PLAYPAL`, `DEMO1`, `F_SKY1`. Map names: `ExMy` (DOOM I / Heretic) or `MAPxx` (DOOM II).

## 15. Implementation status (cross-reference)

For the current in-app status of every menu item, see the README's status
table and `CHANGELOG.md` track summaries. As of this writing 16 of the 49
menu items are real (Open / Save / Save-as / About / Map Information /
System Information / List WADs / Add PWAD / Quit / Add-split / Delete /
Goto / Properties / Quick check / Check all / Error list / Polygon / Door
/ Stairs / Grid+Origin toggles / Center map / Refresh / Snap-grid edit /
Viewer / Undo from last save). The remaining 33 surface
"[Menu] Item: not implemented yet" notices and are tracked for follow-on
phases.
