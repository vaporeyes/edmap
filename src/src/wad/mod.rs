// ABOUTME: DOOM WAD file parser. Parses IWAD/PWAD headers, directories, and per-map lumps.
// ABOUTME: Public types are serde-serializable so they can be returned over Tauri IPC.

mod error;
mod header;
mod lump;
mod map;
mod texture;

pub use error::WadError;
pub use header::{WadKind, WadHeader, LumpEntry};
pub use lump::Wad;
pub use map::{MapData, Vertex, LineDef, SideDef, Sector, Thing, MapName};
pub use texture::{
    parse_pnames, parse_textures, Flat, Palette, Patch, PatchName, PatchRef, TextureDef,
    TextureImage, FLAT_DIM,
};
