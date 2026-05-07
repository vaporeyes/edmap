// ABOUTME: Synthesizes a tiny built-in IWAD: PLAYPAL plus empty PNAMES/TEXTURE1
// ABOUTME: and the F_/S_ marker pairs so PWADs can resolve required IWAD lumps.

const PALETTE_LUMP: &[u8; 768] = &synth_palette();

const fn synth_palette() -> [u8; 768] {
    let mut out = [0u8; 768];
    let mut i = 0usize;
    while i < 256 {
        let r = ((i as u32 * 7) & 0xFF) as u8;
        let g = ((i as u32 * 5 + 50) & 0xFF) as u8;
        let b = ((i as u32 * 3 + 100) & 0xFF) as u8;
        out[i * 3] = r;
        out[i * 3 + 1] = g;
        out[i * 3 + 2] = b;
        i += 1;
    }
    out
}

/// A lump record while building: name + payload bytes.
struct Lump {
    name: &'static str,
    data: Vec<u8>,
}

/// Build a minimal IWAD byte buffer. Contains:
/// PLAYPAL, empty PNAMES, empty TEXTURE1, and zero-size F_START/F_END,
/// S_START/S_END, P_START/P_END marker pairs.
pub fn bytes() -> Vec<u8> {
    let lumps: Vec<Lump> = vec![
        Lump { name: "PLAYPAL", data: PALETTE_LUMP.to_vec() },
        Lump { name: "PNAMES",  data: 0u32.to_le_bytes().to_vec() },
        Lump { name: "TEXTURE1", data: 0u32.to_le_bytes().to_vec() },
        Lump { name: "P_START", data: Vec::new() },
        Lump { name: "P_END",   data: Vec::new() },
        Lump { name: "F_START", data: Vec::new() },
        Lump { name: "F_END",   data: Vec::new() },
        Lump { name: "S_START", data: Vec::new() },
        Lump { name: "S_END",   data: Vec::new() },
    ];
    write_wad(b"IWAD", &lumps)
}

fn write_wad(magic: &[u8; 4], lumps: &[Lump]) -> Vec<u8> {
    let mut out = Vec::new();
    // 12-byte header placeholder; patched after the directory offset is known.
    out.extend_from_slice(magic);
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());

    let mut entries: Vec<(u32, u32, &str)> = Vec::with_capacity(lumps.len());
    for lump in lumps {
        let off = out.len() as u32;
        out.extend_from_slice(&lump.data);
        entries.push((off, lump.data.len() as u32, lump.name));
    }

    let dir_offset = out.len() as u32;
    for (off, size, name) in &entries {
        out.extend_from_slice(&off.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        let mut padded = [0u8; 8];
        let nb = name.as_bytes();
        let n = nb.len().min(8);
        padded[..n].copy_from_slice(&nb[..n]);
        out.extend_from_slice(&padded);
    }

    out[4..8].copy_from_slice(&(entries.len() as u32).to_le_bytes());
    out[8..12].copy_from_slice(&dir_offset.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wad::{Wad, Palette};

    #[test]
    fn parses_and_has_palette() {
        let bytes = bytes();
        let wad = Wad::from_bytes(bytes).expect("parse");
        let pal = wad.lump_bytes_by_name("PLAYPAL").expect("playpal");
        assert_eq!(pal.len(), 768);
        Palette::parse_first(pal).expect("valid palette");
        assert!(wad.lump_bytes_by_name("PNAMES").is_some());
        assert!(wad.lump_bytes_by_name("TEXTURE1").is_some());
        // Markers exist as zero-size lumps.
        assert!(wad.find_lump("F_START").is_some());
        assert!(wad.find_lump("F_END").is_some());
        assert!(wad.find_lump("S_START").is_some());
        assert!(wad.find_lump("S_END").is_some());
    }
}
