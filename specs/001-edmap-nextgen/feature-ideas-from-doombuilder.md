# Feature ideas borrowed from Doom Builder v1

Synthesized from "Doom Builder: An Illustrated Guide" v1.2 (John W. Anderson,
2004) — covers Doom Builder up to build 310. Most features below would slot
cleanly into our existing menu / dialog / commands architecture; rough effort
estimates are 1 turn (small), 2-3 turns (medium), or 5+ turns (large).

## High value, small effort

| Idea | What | Why useful |
|---|---|---|
| **Flip LineDef (F)** | Swap front and back sidedef indices on selected LineDefs | One-keystroke fix when door/lift/teleport actions need to be activated from the right side |
| **Auto-align textures (A)** | Recursive walk along connected linedefs sharing the same texture; sets x-offset on each so the texture appears continuous | Chains of corridor walls become seamless without editing 20 sidedefs by hand |
| **Curve LineDef** | Turn a selected LineDef into a smooth arc with N vertices, given curve distance + delta angle | Round corners, pillars, curved corridors. Single dialog box: vertices/distance/angle |
| **Stitch Vertices** | When placing a vertex within N pixels of an existing one, merge them automatically. Toggleable preference | Prevents tiny invisible gaps that crash the renderer |
| **Categorized thing-type picker** | Replace the raw `Type:` number in EditThing with a tree: Player Starts / Teleports / Monsters / Weapons / Ammunition / Health / Powerups / Keys / Obstacles / Light Sources / Decoration | Right now you have to know thing-type 3001 = Imp; tree turns it into pick-from-list |
| **Categorized linedef-action picker** | Same idea for the EditLineDef `Special:` field. Tree by Door / Lift / Crusher / Stairs / Teleport / Light / Exit / Scroll, then by single/walkover/switch/trigger | Eliminates the Doom action-table memorization barrier |
| **Things filter** | Show only certain Thing categories on the map (e.g. only Monsters, only Items). Filter dialog with checkboxes for category + skill flags | Lets designers verify ammo/health balance per skill without visual clutter |
| **Things bounding-box render** | Toggleable: draw each Thing as a square sized to its true DOOM radius (Imp=40, Cyberdemon=80, Spider Mastermind=256, etc.) | Visually confirms a corridor is wide enough for the monster to fit |
| **PgUp/PgDn for sector heights** | Selected sector(s): PgUp raises ceiling by 8, PgDn lowers; Ctrl+PgUp/Dn does brightness | Faster than reopening the EditSector dialog for each height tweak |
| **Next-unused tag button** | In EditLineDef and EditSector dialogs, a `Next Unused` button next to the Tag field | We already have `next_unused_tag()` internally; just expose it |

## High value, medium effort

| Idea | What | Why useful |
|---|---|---|
| **Find / Find & Replace** | Search by: vertex#, linedef#, sector#, thing#, texture name, flat name, action#, sector tag, thing type. Optional `Within current selection` scope. Replace-all for textures/flats/types/tags | Mass-edit a level: replace BRICK6→BRICK7 in 40 sidedefs in one click |
| **Copy / Paste objects (Ctrl-C / Ctrl-V)** | Copy selected vertices/linedefs/sectors/things to internal clipboard, paste at cursor. Multi-object selections paste with relative geometry preserved | Essential for "I want this corridor 3 more times" workflows |
| **Flip Selection (Horizontal/Vertical)** | Mirror selected objects along their bounding-box axis | Symmetry for arenas, mirrored hallways |
| **Rotate / Scale Selection** | Like our Map Shift/Expand but operating only on selected geometry. Numeric dialog: degrees / percentage | We have map-wide; this is per-selection. Common need |
| **Join Sectors / Merge Sectors** | Two flavors: Join = both sectors get same sector index but shared linedef stays; Merge = shared linedef deleted, single combined sector | Cleanup after polygon construction; combining staircase steps |
| **Gradient distribution** | Select 3+ sectors, then Gradient Floors/Ceilings/Brightness evenly distributes the value from first→last sector | Stairs floor heights and dimming corridors become one click each |
| **Line-Draw Mode** | Toggleable mode where right-click places a vertex with a "rubber-band" linedef following the cursor; left-click anchors. Closing on the starting vertex completes a sector | Replaces "Polygon" for arbitrary-shape sectors. The DOOM Builder primary workflow |
| **Save selection as Prefab** | Export a sub-region of the map (vertices + linedefs + sidedefs + sectors) to a .epfab file. Insert prefab from file places it at cursor with optional rotate | Reusable pieces — toilets, columns, computer banks, etc. |
| **Map backup on save** | Before overwriting a PWAD, write a `.bak` copy alongside it (configurable count) | Cheap insurance against accidental save-over |
| **Remove unused textures** | Strip non-required texture names from sidedefs (e.g. middle texture on a 2-sided line that doesn't need one) | Saves ~30% of SIDEDEF lump size on average |
| **Fix missing textures** | Auto-fill missing required textures with the default texture from preferences | Drives the "Check > Missing textures" finding to a one-click resolve |
| **Fix zero-length linedefs** | Sweep + delete linedefs whose endpoints coincide | Removes a class of map errors automatically |
| **Customizable hotkeys** | Preferences > Shortcut Keys tab — show every command + current key, allow rebinding | We already collect hotkeys from the menu spec; just make the table editable |
| **Color preferences** | Tab where the user can override every theme color (selected linedef, hovered vertex, sector tag highlight, grid, etc.) | Personal taste + accessibility |

## High value, larger effort

| Idea | What | Why useful |
|---|---|---|
| **3D Edit Mode (W)** | Real-time textured walkthrough of the map. WASD movement, mouselook, click-to-paste-texture, scroll-to-raise-ceiling | The single most-loved Doom Builder feature. Lets designers eyeball alignment + lighting without launching DOOM. Big effort: needs a software-rendered Doom-style BSP renderer or an OpenGL pipeline |
| **Test Map (F8/F9)** | Launch external DOOM port (gzdoom, dsda-doom, prboom+) with current PWAD. Configurable executable + parameters with placeholders (`%F`=pwad, `%L`=map name, `%E/%M`=episode/map) | Closes the design loop. The placeholder system from DB scales to ZDoom, jDOOM, etc. |
| **External node builder integration** | Before save/test, shell out to bsp/ZDBSP/ZenNode to build the SEGS/SSECTORS/NODES/BLOCKMAP/REJECT lumps. Configurable per-port profile | Required for a real PWAD that DOOM can actually load. The "Quick Build" vs "Final Build" split lets users pick fast vs. optimized |
| **Export picture** | Save the current map view as PNG with optional grid / line lengths / vertex dots / things overlay. Sized to fit or scaled by % | Map documentation, sharing, Wiki uploads |
| **Compress sidedefs on save** | After node build, dedupe identical SIDEDEF records and remap LINEDEFs to share | Shrinks PWADs ~30%. Trivially safe — no behavior change |
| **REJECT table optimization** | Compute the line-of-sight matrix that DOOM uses to early-cull monsters | ~80% speedup in monster-heavy levels. Specialized algorithm |
| **Alternate texture WAD support** | Load a second PWAD whose only purpose is providing additional TEXTURE1/PNAMES/F_START..END to the current map | Theme packs (gothic, tech, medieval) without modifying the main IWAD |
| **Script editor** | Multi-line script lump editor for Hexen / ZDoom ACS / jDoom DD_DEFNS. Syntax highlighting, import/export, compile button | Out of scope for vanilla DOOM but reasonable if we want to support modern ports |

## Workflow ideas (UX polish, no big feature)

- **Status messages auto-clear after N seconds** — currently they stick until manually cleared.
- **Right-click on object opens its Edit dialog directly** — currently we Enter; right-click is unused. DB uses right-click for both selection and dialog.
- **Drag-box selection** — left-button drag on empty area draws a selection rectangle, selecting all objects within.
- **C key clears selection** — currently we click empty space; C is faster.
- **`[` / `]` keys cycle grid size** through `1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024`.
- **Status bar at bottom of viewport** — vertices/linedefs/sidedefs/sectors/things counts, grid, snap, zoom, X, Y. We have them in the sidebar but a dedicated status row scales to wider screens.
- **Undo levels configurable** (currently single-level "Undo to last save"; DB has 50 levels by default). Snapshot-on-every-mutation, Ctrl-Z walks the stack.
- **Show line lengths in viewport** (toggle) — small numbers near each linedef.
- **Snap to grid offset** — grid origin can be shifted X and Y; useful when starting from non-(0,0) maps.

## Recommended next phase

Three options, ordered by what makes the editor *feel* the most professional:

1. **Auto-align textures (A) + Flip LineDef (F) + Curve LineDef** — three small features, all single-key, all directly visible in the map view. Maybe 1.5 turns.
2. **Categorized pickers for Thing-type and LineDef-action** — turn the EditThing/EditLineDef dialogs from "type a number you must memorize" into "click from a tree". Big UX win, medium effort.
3. **Find / Find & Replace** — the most-cited DB power feature. Wire `Edit > Find Objects (Ctrl-F)` properly with a multi-search-type dialog + Replace mode. Texture/flat/tag/action/sector-type all replaceable in one place.

My pick: **#2** (categorized pickers), because every other feature we add later that involves picking a thing-type or action will share that picker — it pays compound interest.
