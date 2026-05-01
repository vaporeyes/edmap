// ABOUTME: Editor state — currently-loaded WAD, current map, view transform, selection.
// ABOUTME: Single source of truth shared across sidebar, viewport, and menu commands.

use std::path::PathBuf;

use crate::wad::{MapData, Wad};

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
