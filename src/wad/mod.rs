// ABOUTME: DOOM WAD file parser. Parses IWAD/PWAD headers, directories, and per-map lumps.
// ABOUTME: Public types are serde-serializable so they can be returned over Tauri IPC.

mod error;
mod header;
mod lump;
mod map;
mod texture;
mod write;

pub use error::WadError;
pub use header::WadKind;
pub use lump::Wad;
pub use map::{MapData, Vertex, LineDef, SideDef, Sector, Thing, MapName};
pub use texture::{
    parse_pnames, parse_textures, Flat, Palette, Patch, PatchName, TextureDef,
    TextureImage, FLAT_DIM,
};

#[cfg(not(target_arch = "wasm32"))]
pub use write::save_map_to_path;
