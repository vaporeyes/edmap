// ABOUTME: Top-level WAD container — owns file bytes and the directory, exposes lump lookup.
// ABOUTME: Map loading walks the directory for the named map header and the 10 lumps that follow.

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use super::header::{LumpEntry, WadHeader};
use super::map::{LineDef, MapData, Sector, SideDef, Thing, Vertex};
use super::WadError;

/// Map lumps follow the map-name marker in this fixed order. We require the five
/// the editor needs; SEGS/SSECTORS/NODES/BLOCKMAP/REJECT are nodes-builder output
/// and are absent for unbuilt maps (which is the common case in an editor).
const MAP_LUMPS: &[&str] = &[
    "THINGS", "LINEDEFS", "SIDEDEFS", "VERTEXES", "SEGS",
    "SSECTORS", "NODES", "SECTORS", "BLOCKMAP", "REJECT",
];

#[derive(Debug, Clone)]
pub struct Wad {
    pub header: WadHeader,
    pub directory: Vec<LumpEntry>,
    bytes: Vec<u8>,
}

impl Wad {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, WadError> {
        let bytes = std::fs::read(path)?;
        Self::from_bytes(bytes)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, WadError> {
        let header = WadHeader::parse(&bytes)?;
        let dir_off = header.directory_offset as u64;
        let dir_size = header.num_lumps as u64 * LumpEntry::SIZE as u64;
        let file_len = bytes.len() as u64;
        if dir_off + dir_size > file_len {
            return Err(WadError::BadDirectory {
                offset: dir_off,
                size: dir_size,
                file_len,
            });
        }
        let mut directory = Vec::with_capacity(header.num_lumps as usize);
        for i in 0..header.num_lumps as usize {
            let off = dir_off as usize + i * LumpEntry::SIZE;
            directory.push(LumpEntry::parse(&bytes[off..off + LumpEntry::SIZE])?);
        }
        Ok(Self { header, directory, bytes })
    }

    pub fn lump_bytes(&self, entry: &LumpEntry) -> &[u8] {
        let start = entry.offset as usize;
        let end = start + entry.size as usize;
        &self.bytes[start..end.min(self.bytes.len())]
    }

    pub fn find_lump(&self, name: &str) -> Option<&LumpEntry> {
        self.directory.iter().find(|e| e.name == name)
    }

    pub fn lump_bytes_by_name(&self, name: &str) -> Option<&[u8]> {
        self.find_lump(name).map(|e| self.lump_bytes(e))
    }

    /// Names of all non-marker lumps strictly between `start_marker` and
    /// `end_marker` (e.g. F_START..F_END for flats, S_START..S_END for sprites).
    /// Multiple ranges with the same markers are concatenated.
    pub fn lumps_between(&self, start_marker: &str, end_marker: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut inside = false;
        for entry in &self.directory {
            if entry.name == start_marker {
                inside = true;
                continue;
            }
            if entry.name == end_marker {
                inside = false;
                continue;
            }
            if inside && entry.size != 0 {
                out.push(entry.name.clone());
            }
        }
        out
    }

    /// All map names found in the directory (entries that are zero-sized markers
    /// matching ExMy or MAPxx and followed by THINGS).
    pub fn map_names(&self) -> Vec<String> {
        let mut maps = Vec::new();
        for (i, entry) in self.directory.iter().enumerate() {
            if entry.size != 0 {
                continue;
            }
            if !is_map_marker_name(&entry.name) {
                continue;
            }
            // Confirm by checking the next directory entry is THINGS.
            if let Some(next) = self.directory.get(i + 1) {
                if next.name == "THINGS" {
                    maps.push(entry.name.clone());
                }
            }
        }
        maps
    }

    pub fn load_map(&self, name: &str) -> Result<MapData, WadError> {
        let marker_idx = self
            .directory
            .iter()
            .position(|e| e.name == name && e.size == 0)
            .ok_or_else(|| WadError::MapNotFound(name.into()))?;

        let mut found = std::collections::HashMap::new();
        for offset in 1..=MAP_LUMPS.len() {
            let Some(entry) = self.directory.get(marker_idx + offset) else { break };
            if MAP_LUMPS.contains(&entry.name.as_str()) {
                found.insert(entry.name.as_str(), entry.clone());
            } else {
                break;
            }
        }

        let take_required = |key: &'static str| -> Result<&LumpEntry, WadError> {
            found.get(key).ok_or_else(|| WadError::IncompleteMap(name.into(), key))
        };

        let things = Thing::parse_all(self.lump_bytes(take_required("THINGS")?))?;
        let linedefs = LineDef::parse_all(self.lump_bytes(take_required("LINEDEFS")?))?;
        let sidedefs = SideDef::parse_all(self.lump_bytes(take_required("SIDEDEFS")?))?;
        let vertices = Vertex::parse_all(self.lump_bytes(take_required("VERTEXES")?))?;
        let sectors = Sector::parse_all(self.lump_bytes(take_required("SECTORS")?))?;

        Ok(MapData {
            name: name.to_string(),
            vertices,
            linedefs,
            sidedefs,
            sectors,
            things,
        })
    }
}

fn is_map_marker_name(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() == 4 && b[0] == b'E' && b[2] == b'M' {
        return b[1].is_ascii_digit() && b[3].is_ascii_digit();
    }
    if b.len() == 5 && &b[0..3] == b"MAP" {
        return b[3].is_ascii_digit() && b[4].is_ascii_digit();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid PWAD with one map (MAP01) and a single triangle sector.
    /// Verifies header, directory walk, and per-lump parse.
    fn synthesize_test_wad() -> Vec<u8> {
        let mut bytes = Vec::new();
        // Header placeholder; fill at end.
        bytes.extend_from_slice(b"PWAD");
        bytes.extend_from_slice(&0u32.to_le_bytes()); // num_lumps placeholder
        bytes.extend_from_slice(&0u32.to_le_bytes()); // dir_offset placeholder

        // Lump: THINGS — 1 thing (player 1 start at 0,0)
        let things_off = bytes.len() as u32;
        let mut thing = Vec::new();
        thing.extend_from_slice(&0i16.to_le_bytes()); // x
        thing.extend_from_slice(&0i16.to_le_bytes()); // y
        thing.extend_from_slice(&0i16.to_le_bytes()); // angle
        thing.extend_from_slice(&1u16.to_le_bytes()); // type 1 = player 1 start
        thing.extend_from_slice(&7u16.to_le_bytes()); // flags
        let things_size = thing.len() as u32;
        bytes.extend_from_slice(&thing);

        // LINEDEFS — 3 linedefs forming a triangle
        let linedefs_off = bytes.len() as u32;
        let linedef_bytes = |sv: u16, ev: u16| -> [u8; 14] {
            let mut b = [0u8; 14];
            b[0..2].copy_from_slice(&sv.to_le_bytes());
            b[2..4].copy_from_slice(&ev.to_le_bytes());
            b[4..6].copy_from_slice(&1u16.to_le_bytes()); // flags: blocking
            b[6..8].copy_from_slice(&0u16.to_le_bytes()); // special
            b[8..10].copy_from_slice(&0u16.to_le_bytes()); // tag
            b[10..12].copy_from_slice(&0u16.to_le_bytes()); // front sd
            b[12..14].copy_from_slice(&0xFFFFu16.to_le_bytes()); // back sd
            b
        };
        bytes.extend_from_slice(&linedef_bytes(0, 1));
        bytes.extend_from_slice(&linedef_bytes(1, 2));
        bytes.extend_from_slice(&linedef_bytes(2, 0));
        let linedefs_size = (3 * 14) as u32;

        // SIDEDEFS — 1 sidedef
        let sidedefs_off = bytes.len() as u32;
        let mut sd = Vec::new();
        sd.extend_from_slice(&0i16.to_le_bytes()); // x_off
        sd.extend_from_slice(&0i16.to_le_bytes()); // y_off
        sd.extend_from_slice(b"-\0\0\0\0\0\0\0"); // upper
        sd.extend_from_slice(b"-\0\0\0\0\0\0\0"); // lower
        sd.extend_from_slice(b"STARTAN2"); // middle
        sd.extend_from_slice(&0u16.to_le_bytes()); // sector
        let sidedefs_size = sd.len() as u32;
        bytes.extend_from_slice(&sd);

        // VERTEXES — 3 vertices
        let vertexes_off = bytes.len() as u32;
        for (x, y) in [(0i16, 0i16), (128, 0), (64, 128)] {
            bytes.extend_from_slice(&x.to_le_bytes());
            bytes.extend_from_slice(&y.to_le_bytes());
        }
        let vertexes_size = (3 * 4) as u32;

        // SECTORS — 1 sector
        let sectors_off = bytes.len() as u32;
        let mut sec = Vec::new();
        sec.extend_from_slice(&0i16.to_le_bytes()); // floor h
        sec.extend_from_slice(&128i16.to_le_bytes()); // ceiling h
        sec.extend_from_slice(b"FLOOR4_8"); // floor tex
        sec.extend_from_slice(b"CEIL3_5\0"); // ceiling tex
        sec.extend_from_slice(&160i16.to_le_bytes()); // light
        sec.extend_from_slice(&0u16.to_le_bytes()); // type
        sec.extend_from_slice(&0u16.to_le_bytes()); // tag
        let sectors_size = sec.len() as u32;
        bytes.extend_from_slice(&sec);

        // Directory at end
        let dir_offset = bytes.len() as u32;
        let push_entry = |bytes: &mut Vec<u8>, off: u32, size: u32, name: &[u8]| {
            bytes.extend_from_slice(&off.to_le_bytes());
            bytes.extend_from_slice(&size.to_le_bytes());
            let mut padded = [0u8; 8];
            padded[..name.len()].copy_from_slice(name);
            bytes.extend_from_slice(&padded);
        };
        push_entry(&mut bytes, 0, 0, b"MAP01");
        push_entry(&mut bytes, things_off, things_size, b"THINGS");
        push_entry(&mut bytes, linedefs_off, linedefs_size, b"LINEDEFS");
        push_entry(&mut bytes, sidedefs_off, sidedefs_size, b"SIDEDEFS");
        push_entry(&mut bytes, vertexes_off, vertexes_size, b"VERTEXES");
        push_entry(&mut bytes, sectors_off, sectors_size, b"SECTORS");
        let num_lumps = 6u32;

        // Patch header
        bytes[4..8].copy_from_slice(&num_lumps.to_le_bytes());
        bytes[8..12].copy_from_slice(&dir_offset.to_le_bytes());

        bytes
    }

    #[test]
    fn parses_synthetic_pwad() {
        let wad_bytes = synthesize_test_wad();
        let wad = Wad::from_bytes(wad_bytes).expect("parse wad");
        assert_eq!(wad.header.num_lumps, 6);
        assert_eq!(wad.map_names(), vec!["MAP01"]);
        let map = wad.load_map("MAP01").expect("load map");
        assert_eq!(map.vertices.len(), 3);
        assert_eq!(map.linedefs.len(), 3);
        assert_eq!(map.sidedefs.len(), 1);
        assert_eq!(map.sectors.len(), 1);
        assert_eq!(map.things.len(), 1);
        assert_eq!(map.sectors[0].floor_texture, "FLOOR4_8");
        assert_eq!(map.sectors[0].ceiling_texture, "CEIL3_5");
        assert_eq!(map.sidedefs[0].middle_texture, "STARTAN2");
        assert_eq!(map.things[0].thing_type, 1);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bad = vec![0u8; 12];
        bad[0..4].copy_from_slice(b"XWAD");
        let err = Wad::from_bytes(bad).unwrap_err();
        matches!(err, WadError::BadMagic(_));
    }
}
