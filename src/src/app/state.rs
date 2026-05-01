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
}

/// Currently-shown tab in the texture viewer (F10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerCategory {
    Walls = 0,
    Flats = 1,
    Sprites = 2,
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
    pub viewer_open: bool,
    pub viewer_category: ViewerCategory,
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
            viewer_open: false,
            viewer_category: ViewerCategory::Walls,
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
