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
    /// Categorized picker for thing-types or linedef-actions. Routes the
    /// chosen code back to whichever Edit dialog is stashed in dialog_pending.
    Picker { kind: PickerKind, expanded: usize },
    RotateSelection { degrees: String },
    ScaleSelection { percent: String },
    FindReplace {
        kind: FindKind,
        find: String,
        replace: String,
        replace_mode: bool,
    },
    Preferences,
    Polygon { sides: String, radius: String },
    Door { key: DoorKey, fast: bool },
    CurveLineDef { vertices_per_line: String, curve_distance: String, delta_angle: String },
    ThingsFilter { categories: [bool; 11] },
    Lift { repeatable: bool, fast: bool },
    Teleporter,
    ShiftMap { dx: String, dy: String, dz: String },
    ExpandMap { sx: String, sy: String, sz: String },
    LightAdjust { a: String, b: String },
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
    /// Configure the external source-port used by Test Map (Ctrl-F9).
    TestMapSettings { exe: String, args: String },
    /// Export the current map view as a PNG.
    ExportPicture {
        width: String,
        height: String,
        with_grid: bool,
        with_vertices: bool,
        with_things: bool,
        with_thing_bboxes: bool,
    },
}

/// Visual intensity for the map grid dots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridIntensity {
    Dim,
    Normal,
    Bright,
}

impl GridIntensity {
    pub fn cycle(self) -> Self {
        match self {
            GridIntensity::Dim => GridIntensity::Normal,
            GridIntensity::Normal => GridIntensity::Bright,
            GridIntensity::Bright => GridIntensity::Dim,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            GridIntensity::Dim => "dim",
            GridIntensity::Normal => "normal",
            GridIntensity::Bright => "bright",
        }
    }
    pub fn color(self) -> egui::Color32 {
        match self {
            GridIntensity::Dim => crate::theme::GRID_DOT_DIM,
            GridIntensity::Normal => crate::theme::GRID_DOT,
            GridIntensity::Bright => crate::theme::GRID_DOT_BRIGHT,
        }
    }
}

/// Which categorized picker is active.
#[derive(Debug, Clone, Copy)]
pub enum PickerKind {
    ThingType,
    LineDefAction,
}

/// Search categories for Find / Find & Replace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindKind {
    LineDefTexture,
    SectorFloorTexture,
    SectorCeilingTexture,
    LineDefAction,
    SectorTag,
    ThingType,
    LineDefIndex,
    SectorIndex,
    ThingIndex,
    VertexIndex,
}

impl FindKind {
    pub fn label(self) -> &'static str {
        match self {
            FindKind::LineDefTexture => "LineDef texture",
            FindKind::SectorFloorTexture => "Sector floor texture",
            FindKind::SectorCeilingTexture => "Sector ceiling texture",
            FindKind::LineDefAction => "LineDef action #",
            FindKind::SectorTag => "Sector tag #",
            FindKind::ThingType => "Thing type #",
            FindKind::LineDefIndex => "LineDef index",
            FindKind::SectorIndex => "Sector index",
            FindKind::ThingIndex => "Thing index",
            FindKind::VertexIndex => "Vertex index",
        }
    }

    pub fn all() -> &'static [FindKind] {
        &[
            FindKind::LineDefTexture,
            FindKind::SectorFloorTexture,
            FindKind::SectorCeilingTexture,
            FindKind::LineDefAction,
            FindKind::SectorTag,
            FindKind::ThingType,
            FindKind::LineDefIndex,
            FindKind::SectorIndex,
            FindKind::ThingIndex,
            FindKind::VertexIndex,
        ]
    }

    pub fn supports_replace(self) -> bool {
        matches!(
            self,
            FindKind::LineDefTexture
                | FindKind::SectorFloorTexture
                | FindKind::SectorCeilingTexture
                | FindKind::LineDefAction
                | FindKind::SectorTag
                | FindKind::ThingType
        )
    }
}

/// In-memory clipboard for Copy/Paste. Mode-specific: holds cloned objects
/// with positions stored relative to the copy-time centroid so paste lands
/// at the cursor with the same internal layout.
#[derive(Debug, Clone)]
pub enum Clipboard {
    Vertices(Vec<crate::wad::Vertex>),
    Things(Vec<crate::wad::Thing>),
}

/// Line-Draw Mode active state. Stores the chain of placed vertex indices
/// in placement order; the first index is also the "close on this vertex
/// to complete a sector" anchor.
#[derive(Debug, Clone)]
pub struct LineDrawState {
    pub chain: Vec<u16>,
}

/// User overrides for the theme colors. Each None falls back to the static
/// constant in `crate::theme`.
#[derive(Debug, Clone, Default)]
pub struct ThemeOverrides {
    pub viewport_bg: Option<egui::Color32>,
    pub linedef_normal: Option<egui::Color32>,
    pub linedef_two_sided: Option<egui::Color32>,
    pub linedef_selected: Option<egui::Color32>,
    pub vertex_dot: Option<egui::Color32>,
    pub vertex_hover: Option<egui::Color32>,
    pub thing_mark: Option<egui::Color32>,
    pub grid_dot: Option<egui::Color32>,
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
    pub grid_intensity: GridIntensity,
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
    /// When set, only Things in those categories render. All-true = unfiltered.
    pub thing_filter: [bool; 11],
    /// When true, viewport renders each Thing's actual DOOM radius as a square.
    pub things_bbox_visible: bool,
    /// Internal clipboard for Ctrl-C / Ctrl-V. Stores cloned objects with
    /// world coordinates relative to the bounding-box centroid at copy time.
    pub clipboard: Option<Clipboard>,
    /// Line-Draw Mode state: when active, right-click places a vertex (snapped),
    /// left-click anchors a linedef from the previous vertex. Closing on the
    /// initial vertex completes a sector. Esc cancels.
    pub line_draw: Option<LineDrawState>,
    /// Live color overrides for the editor's theme. None = use theme default.
    pub theme_overrides: ThemeOverrides,
    pub calculator_open: bool,
    pub calculator: Option<super::calculator::CalculatorState>,
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
    /// Multi-level undo stack — snapshots pushed before each mutation.
    /// Capped at UNDO_DEPTH; oldest entries dropped when full.
    pub undo_stack: Vec<crate::wad::MapData>,
    /// Last set of check results (for Ctrl-L "reopen Error List").
    pub last_check_results: Vec<super::checks::CheckResult>,
    /// Persistent user preferences (test-map exe/args). Loaded at app start.
    pub config: super::config::EdMapConfig,
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
            grid_intensity: GridIntensity::Normal,
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
            thing_filter: [true; 11],
            things_bbox_visible: false,
            clipboard: None,
            line_draw: None,
            theme_overrides: ThemeOverrides::default(),
            calculator_open: false,
            calculator: None,
            is_dirty: false,
            drag_residual: egui::Vec2::ZERO,
            drag_active: false,
            undo_baseline: None,
            undo_stack: Vec::new(),
            last_check_results: Vec::new(),
            config: super::config::EdMapConfig::default(),
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
