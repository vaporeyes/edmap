// ABOUTME: Editor state — currently-loaded WAD, current map, view transform, selection.
// ABOUTME: Single source of truth shared across sidebar, viewport, and menu commands.

use std::path::PathBuf;

use crate::wad::{MapData, Wad};

/// Action queued behind the Save warning dialog. After the user picks
/// Yes/No/Cancel we run this to continue (or abandon) the original intent.
#[derive(Debug, Clone)]
pub enum PendingAction {
    Quit,
    NewMap,
    OpenWad,
}

/// Modal dialog currently shown over the viewport. Variants own transient
/// input state so the dialog can be drawn statelessly each frame.
#[derive(Debug, Clone)]
pub enum Dialog {
    About,
    MapInformation,
    SystemInformation,
    SnapGrid { grid: String, snap: String },
    GotoObject { input: String },
    WadList,
    OpenMapPicker { maps: Vec<String>, selected: usize },
    Notice { title: String, message: String },
    SaveWarning { pending: PendingAction },
    /// Step-through list of issues from the Check menu. `cursor` is the index
    /// of the currently-shown result; "Next/Previous" walk it.
    ErrorList { results: Vec<super::checks::CheckResult>, cursor: usize },
    Polygon { sides: String, radius: String },
    Door { key: DoorKey, fast: bool },
    EditVertex { idx: usize, x: String, y: String },
    EditLineDef {
        idx: usize,
        flags: String,
        special: String,
        tag: String,
        front_sidedef: String,
        back_sidedef: String,
    },
    EditSector {
        idx: usize,
        floor_height: String,
        ceiling_height: String,
        light: String,
        sector_type: String,
        tag: String,
        floor_texture: String,
        ceiling_texture: String,
    },
    EditThing {
        idx: usize,
        x: String,
        y: String,
        angle: String,
        thing_type: String,
        flags: String,
    },
    Stairs {
        steps: String,
        rise: String,
        depth: String,
        width: String,
        direction: StairsDirection,
        top_texture: String,
        side_texture: String,
    },
}

/// Door key requirement for Door auto-construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorKey {
    Keyless,
    Blue,
    Yellow,
    Red,
}

impl DoorKey {
    pub fn label(self) -> &'static str {
        match self {
            DoorKey::Keyless => "keyless",
            DoorKey::Blue => "blue key",
            DoorKey::Yellow => "yellow key",
            DoorKey::Red => "red key",
        }
    }
}

/// Compass direction for Stairs construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StairsDirection {
    North,
    East,
    South,
    West,
}

impl StairsDirection {
    pub fn label(self) -> &'static str {
        match self {
            StairsDirection::North => "North",
            StairsDirection::East => "East",
            StairsDirection::South => "South",
            StairsDirection::West => "West",
        }
    }
    /// Unit vector (forward direction). +X = East, +Y = North.
    pub fn forward(self) -> (i32, i32) {
        match self {
            StairsDirection::North => (0, 1),
            StairsDirection::East => (1, 0),
            StairsDirection::South => (0, -1),
            StairsDirection::West => (-1, 0),
        }
    }
    /// Unit vector (right-hand perpendicular, i.e. step's "width" axis).
    pub fn right(self) -> (i32, i32) {
        match self {
            StairsDirection::North => (1, 0),
            StairsDirection::East => (0, -1),
            StairsDirection::South => (-1, 0),
            StairsDirection::West => (0, 1),
        }
    }
}

/// Currently-shown tab in the texture viewer (F10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerCategory {
    Walls = 0,
    Flats = 1,
    Sprites = 2,
}

/// What the viewer is being used to pick. When `Some`, clicking a tile
/// writes the texture name into the matching field of the saved dialog.
#[derive(Debug, Clone, Copy)]
pub enum PickTarget {
    SectorFloor,
    SectorCeiling,
}

impl PickTarget {
    /// Default tab to show when this picker opens.
    pub fn default_category(self) -> ViewerCategory {
        match self {
            PickTarget::SectorFloor | PickTarget::SectorCeiling => ViewerCategory::Flats,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Vertex,
    LineDef,
    Sector,
    Thing,
}

impl SelectionMode {
    pub fn label(self) -> &'static str {
        match self {
            SelectionMode::Vertex => "Vx",
            SelectionMode::LineDef => "Ld",
            SelectionMode::Sector => "Se",
            SelectionMode::Thing => "Th",
        }
    }
}

pub struct EditorState {
    pub wad_path: Option<PathBuf>,
    pub wad: Option<Wad>,
    pub map: Option<MapData>,
    pub mode: SelectionMode,
    pub selection: Vec<usize>,
    pub view_center: egui::Pos2,
    pub view_zoom: f32,
    pub grid_size: i32,
    pub snap_size: i32,
    pub grid_visible: bool,
    pub origin_visible: bool,
    pub cursor_world: egui::Pos2,
    pub open_menu: Option<&'static str>,
    pub status_message: Option<String>,
    pub dialog: Option<Dialog>,
    /// Index of the object currently under the cursor (in the active mode).
    /// Set by the viewport's hover pass each frame so the sidebar can show
    /// details for the hovered object when nothing is selected. Cleared each
    /// frame at the top of the viewport draw.
    pub hover_object: Option<usize>,
    pub viewer_open: bool,
    pub viewer_category: ViewerCategory,
    /// When `Some`, a click in the viewer grid writes a texture name to
    /// `dialog_pending` and closes the viewer instead of just recording it.
    pub viewer_pick: Option<PickTarget>,
    /// Dialog stashed while the viewer is in pick mode. Restored when the
    /// viewer closes (with or without a pick).
    pub dialog_pending: Option<Dialog>,
    /// True when the in-memory map has unsaved changes.
    pub is_dirty: bool,
    /// Drag state: world-space remainder accumulated during a mouse drag so
    /// integer-coord snap doesn't lose sub-pixel motion across frames.
    pub drag_residual: egui::Vec2,
    pub drag_active: bool,
    /// Snapshot of the map after the last load or save. Restored by
    /// Edit > Undo from last save. Cleared when no map is loaded.
    pub undo_baseline: Option<crate::wad::MapData>,
    /// Last set of check results (for Ctrl-L "reopen Error List").
    pub last_check_results: Vec<super::checks::CheckResult>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            wad_path: None,
            wad: None,
            map: None,
            mode: SelectionMode::Vertex,
            selection: Vec::new(),
            view_center: egui::pos2(0.0, 0.0),
            view_zoom: 1.0,
            grid_size: 64,
            snap_size: 8,
            grid_visible: true,
            origin_visible: true,
            cursor_world: egui::pos2(0.0, 0.0),
            open_menu: None,
            status_message: None,
            dialog: None,
            hover_object: None,
            viewer_open: false,
            viewer_category: ViewerCategory::Walls,
            viewer_pick: None,
            dialog_pending: None,
            is_dirty: false,
            drag_residual: egui::Vec2::ZERO,
            drag_active: false,
            undo_baseline: None,
            last_check_results: Vec::new(),
        }
    }
}

impl EditorState {
    pub fn map_name(&self) -> &str {
        self.map.as_deref_name().unwrap_or("untitled")
    }

    pub fn total_for_mode(&self) -> usize {
        let Some(map) = &self.map else { return 0 };
        match self.mode {
            SelectionMode::Vertex => map.vertices.len(),
            SelectionMode::LineDef => map.linedefs.len(),
            SelectionMode::Sector => map.sectors.len(),
            SelectionMode::Thing => map.things.len(),
        }
    }
}

trait MapDataExt {
    fn as_deref_name(&self) -> Option<&str>;
}

impl MapDataExt for Option<MapData> {
    fn as_deref_name(&self) -> Option<&str> {
        self.as_ref().map(|m| m.name.as_str())
    }
}
