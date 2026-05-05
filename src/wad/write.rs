// ABOUTME: WAD writer — serializes MapData back into DOOM map lumps and assembles PWAD bytes.
// ABOUTME: Two modes: fresh (one map only) and preserve (keep other lumps from a source Wad).

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use super::map::{LineDef, MapData, Sector, SideDef, Thing, Vertex};
#[cfg(not(target_arch = "wasm32"))]
use super::WadError;
use super::Wad;

/// 8-byte uppercase NUL-padded lump name.
#[cfg(not(target_arch = "wasm32"))]
fn name_bytes(name: &str) -> [u8; 8] {
    let mut buf = [0u8; 8];
    let bytes = name.as_bytes();
    let len = bytes.len().min(8);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf
}

#[cfg(not(target_arch = "wasm32"))]
fn write_str8(out: &mut Vec<u8>, name: &str) {
    out.extend_from_slice(&name_bytes(name));
}

/// Single lump in the directory.
#[cfg(not(target_arch = "wasm32"))]
struct OutLump {
    name: String,
    bytes: Vec<u8>,
}

/// Serialize the 5 editor map lumps in their canonical order.
/// Order matters: the map marker is followed by THINGS, LINEDEFS, SIDEDEFS,
/// VERTEXES, SEGS, SSECTORS, NODES, SECTORS, BLOCKMAP, REJECT. We emit only
/// the editor-managed five; nodes-builder lumps are absent (a node builder
/// can fill them in after).
#[cfg(not(target_arch = "wasm32"))]
fn serialize_map_lumps(map: &MapData) -> Vec<OutLump> {
    let mut things = Vec::with_capacity(map.things.len() * Thing::SIZE);
    for t in &map.things {
        things.extend_from_slice(&t.x.to_le_bytes());
        things.extend_from_slice(&t.y.to_le_bytes());
        things.extend_from_slice(&t.angle.to_le_bytes());
        things.extend_from_slice(&t.thing_type.to_le_bytes());
        things.extend_from_slice(&t.flags.to_le_bytes());
    }

    let mut linedefs = Vec::with_capacity(map.linedefs.len() * LineDef::SIZE);
    for ld in &map.linedefs {
        linedefs.extend_from_slice(&ld.start_vertex.to_le_bytes());
        linedefs.extend_from_slice(&ld.end_vertex.to_le_bytes());
        linedefs.extend_from_slice(&ld.flags.to_le_bytes());
        linedefs.extend_from_slice(&ld.special_type.to_le_bytes());
        linedefs.extend_from_slice(&ld.sector_tag.to_le_bytes());
        linedefs.extend_from_slice(&ld.front_sidedef.to_le_bytes());
        linedefs.extend_from_slice(&ld.back_sidedef.to_le_bytes());
    }

    let mut sidedefs = Vec::with_capacity(map.sidedefs.len() * SideDef::SIZE);
    for sd in &map.sidedefs {
        sidedefs.extend_from_slice(&sd.x_offset.to_le_bytes());
        sidedefs.extend_from_slice(&sd.y_offset.to_le_bytes());
        write_str8(&mut sidedefs, &sd.upper_texture);
        write_str8(&mut sidedefs, &sd.lower_texture);
        write_str8(&mut sidedefs, &sd.middle_texture);
        sidedefs.extend_from_slice(&sd.sector.to_le_bytes());
    }

    let mut vertexes = Vec::with_capacity(map.vertices.len() * Vertex::SIZE);
    for v in &map.vertices {
        vertexes.extend_from_slice(&v.x.to_le_bytes());
        vertexes.extend_from_slice(&v.y.to_le_bytes());
    }

    let mut sectors = Vec::with_capacity(map.sectors.len() * Sector::SIZE);
    for s in &map.sectors {
        sectors.extend_from_slice(&s.floor_height.to_le_bytes());
        sectors.extend_from_slice(&s.ceiling_height.to_le_bytes());
        write_str8(&mut sectors, &s.floor_texture);
        write_str8(&mut sectors, &s.ceiling_texture);
        sectors.extend_from_slice(&s.light_level.to_le_bytes());
        sectors.extend_from_slice(&s.sector_type.to_le_bytes());
        sectors.extend_from_slice(&s.tag.to_le_bytes());
    }

    vec![
        OutLump { name: "THINGS".into(), bytes: things },
        OutLump { name: "LINEDEFS".into(), bytes: linedefs },
        OutLump { name: "SIDEDEFS".into(), bytes: sidedefs },
        OutLump { name: "VERTEXES".into(), bytes: vertexes },
        OutLump { name: "SECTORS".into(), bytes: sectors },
    ]
}

/// Build the full PWAD binary for a flat list of lumps. Caller orders them.
#[cfg(not(target_arch = "wasm32"))]
fn build_pwad(lumps: &[OutLump]) -> Vec<u8> {
    let mut out = Vec::new();

    // Header placeholder (12 bytes): magic + num_lumps + dir_offset
    out.extend_from_slice(b"PWAD");
    out.extend_from_slice(&0u32.to_le_bytes()); // num_lumps placeholder
    out.extend_from_slice(&0u32.to_le_bytes()); // dir_offset placeholder

    // Lump payloads — track each lump's offset+size for the directory.
    let mut entries: Vec<(u32, u32, &str)> = Vec::with_capacity(lumps.len());
    for lump in lumps {
        let offset = out.len() as u32;
        let size = lump.bytes.len() as u32;
        out.extend_from_slice(&lump.bytes);
        entries.push((offset, size, &lump.name));
    }

    // Directory.
    let dir_offset = out.len() as u32;
    for (offset, size, name) in &entries {
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&name_bytes(name));
    }

    // Patch header.
    let num_lumps = entries.len() as u32;
    out[4..8].copy_from_slice(&num_lumps.to_le_bytes());
    out[8..12].copy_from_slice(&dir_offset.to_le_bytes());
    out
}

/// Produce a PWAD that contains *only* the given map.
#[cfg(not(target_arch = "wasm32"))]
pub fn pwad_with_one_map(map: &MapData) -> Vec<u8> {
    let mut lumps = vec![OutLump { name: map.name.clone(), bytes: Vec::new() }];
    lumps.append(&mut serialize_map_lumps(map));
    build_pwad(&lumps)
}

/// Produce a PWAD that preserves every lump from `src` except those associated
/// with `map.name` (the map marker plus the contiguous run of recognized map
/// lumps that follow it). Then appends the freshly-serialized map at the end.
/// This is what F2 Save uses when you've loaded an existing PWAD.
#[cfg(not(target_arch = "wasm32"))]
pub fn pwad_preserving_others(src: &Wad, map: &MapData) -> Vec<u8> {
    // Find the map marker and the contiguous run of map lumps to skip.
    let mut skip = std::collections::HashSet::new();
    if let Some(idx) = src
        .directory
        .iter()
        .position(|e| e.name == map.name && e.size == 0)
    {
        skip.insert(idx);
        for offset in 1..=10 {
            let Some(entry) = src.directory.get(idx + offset) else { break };
            if is_map_lump(&entry.name) {
                skip.insert(idx + offset);
            } else {
                break;
            }
        }
    }

    // Carry over every other lump.
    let mut lumps: Vec<OutLump> = Vec::with_capacity(src.directory.len() + 6);
    for (i, entry) in src.directory.iter().enumerate() {
        if skip.contains(&i) {
            continue;
        }
        lumps.push(OutLump {
            name: entry.name.clone(),
            bytes: src.lump_bytes(entry).to_vec(),
        });
    }

    // Append the rebuilt map.
    lumps.push(OutLump { name: map.name.clone(), bytes: Vec::new() });
    lumps.append(&mut serialize_map_lumps(map));

    build_pwad(&lumps)
}

#[cfg(not(target_arch = "wasm32"))]
fn is_map_lump(name: &str) -> bool {
    matches!(
        name,
        "THINGS"
            | "LINEDEFS"
            | "SIDEDEFS"
            | "VERTEXES"
            | "SEGS"
            | "SSECTORS"
            | "NODES"
            | "SECTORS"
            | "BLOCKMAP"
            | "REJECT"
    )
}

/// Convenience: build PWAD bytes (preserving src lumps if provided) and write to disk.
/// Also writes a `.bak` copy of the existing file (if any) before overwriting,
/// matching Doom Builder's default safety behavior.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_map_to_path(
    path: impl AsRef<Path>,
    src: Option<&Wad>,
    map: &MapData,
) -> Result<(), WadError> {
    let path_ref = path.as_ref();
    // Best-effort backup: if the target already exists, try to copy it to .bak.
    // Failures are non-fatal — a save shouldn't fail because backup couldn't write.
    if path_ref.exists() {
        let bak = path_ref.with_extension("wad.bak");
        let _ = std::fs::copy(path_ref, &bak);
    }
    let bytes = match src {
        Some(s) => pwad_preserving_others(s, map),
        None => pwad_with_one_map(map),
    };
    std::fs::write(path_ref, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wad::Wad;

    fn synth_map() -> MapData {
        MapData {
            name: "MAP01".into(),
            vertices: vec![
                Vertex { x: 0, y: 0 },
                Vertex { x: 128, y: 0 },
                Vertex { x: 64, y: 128 },
            ],
            linedefs: vec![LineDef {
                start_vertex: 0,
                end_vertex: 1,
                flags: 1,
                special_type: 0,
                sector_tag: 0,
                front_sidedef: 0,
                back_sidedef: LineDef::NO_SIDEDEF,
            }],
            sidedefs: vec![SideDef {
                x_offset: 4,
                y_offset: -8,
                upper_texture: "-".into(),
                lower_texture: "-".into(),
                middle_texture: "STARTAN2".into(),
                sector: 0,
            }],
            sectors: vec![Sector {
                floor_height: 0,
                ceiling_height: 128,
                floor_texture: "FLOOR4_8".into(),
                ceiling_texture: "CEIL3_5".into(),
                light_level: 192,
                sector_type: 0,
                tag: 0,
            }],
            things: vec![Thing { x: 32, y: 32, angle: 90, thing_type: 1, flags: 7 }],
        }
    }

    #[test]
    fn roundtrip_pwad_with_one_map() {
        let original = synth_map();
        let bytes = pwad_with_one_map(&original);
        let wad = Wad::from_bytes(bytes).expect("parse round-trip wad");
        let parsed = wad.load_map("MAP01").expect("load round-trip map");

        assert_eq!(parsed.name, "MAP01");
        assert_eq!(parsed.vertices.len(), 3);
        assert_eq!(parsed.linedefs.len(), 1);
        assert_eq!(parsed.sidedefs.len(), 1);
        assert_eq!(parsed.sectors.len(), 1);
        assert_eq!(parsed.things.len(), 1);

        assert_eq!(parsed.vertices[1].x, 128);
        assert_eq!(parsed.vertices[1].y, 0);
        assert_eq!(parsed.linedefs[0].flags, 1);
        assert_eq!(parsed.sidedefs[0].x_offset, 4);
        assert_eq!(parsed.sidedefs[0].y_offset, -8);
        assert_eq!(parsed.sidedefs[0].middle_texture, "STARTAN2");
        assert_eq!(parsed.sectors[0].floor_texture, "FLOOR4_8");
        assert_eq!(parsed.sectors[0].ceiling_texture, "CEIL3_5");
        assert_eq!(parsed.sectors[0].light_level, 192);
        assert_eq!(parsed.things[0].angle, 90);
        assert_eq!(parsed.things[0].flags, 7);
    }

    #[test]
    fn preserve_keeps_unrelated_lumps() {
        // Build a synthetic source WAD with one map (MAP01) and a PLAYPAL lump.
        let mut src_lumps = vec![
            OutLump { name: "PLAYPAL".into(), bytes: vec![0xAB; 768] },
            OutLump { name: "MAP01".into(), bytes: Vec::new() },
        ];
        src_lumps.append(&mut serialize_map_lumps(&synth_map()));
        let src_bytes = build_pwad(&src_lumps);
        let src = Wad::from_bytes(src_bytes).unwrap();

        // Modify map (move vertex 0) and save preserving others.
        let mut updated = synth_map();
        updated.vertices[0].x = 99;
        let out_bytes = pwad_preserving_others(&src, &updated);
        let out = Wad::from_bytes(out_bytes).unwrap();

        // PLAYPAL must survive untouched.
        let pal = out.lump_bytes_by_name("PLAYPAL").expect("playpal preserved");
        assert_eq!(pal.len(), 768);
        assert!(pal.iter().all(|&b| b == 0xAB));

        // Map must reflect the edit.
        let map_out = out.load_map("MAP01").unwrap();
        assert_eq!(map_out.vertices[0].x, 99);
    }
}
