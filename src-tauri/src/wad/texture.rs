// ABOUTME: PLAYPAL palette and TEXTURE1/2 + PNAMES asset metadata parsing.
// ABOUTME: Per the public DOOM WAD format spec.

use serde::{Deserialize, Serialize};

use super::header::parse_lump_name;
use super::WadError;

/// 256-color RGB palette. PLAYPAL contains 14 palettes; we store the first (game default).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette(pub Vec<[u8; 3]>);

impl Palette {
    pub const ENTRIES: usize = 256;

    pub fn parse_first(bytes: &[u8]) -> Result<Self, WadError> {
        if bytes.len() < Self::ENTRIES * 3 {
            return Err(WadError::TruncatedLump {
                name: "PLAYPAL".into(),
                expected: Self::ENTRIES * 3,
            });
        }
        let entries = bytes[..Self::ENTRIES * 3]
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect();
        Ok(Self(entries))
    }
}

/// 8-byte patch name from PNAMES. Names are uppercase, NUL-padded.
pub type PatchName = String;

pub fn parse_pnames(bytes: &[u8]) -> Result<Vec<PatchName>, WadError> {
    if bytes.len() < 4 {
        return Err(WadError::TruncatedLump {
            name: "PNAMES".into(),
            expected: 4,
        });
    }
    let count = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let need = 4 + count * 8;
    if bytes.len() < need {
        return Err(WadError::TruncatedLump {
            name: "PNAMES".into(),
            expected: need,
        });
    }
    (0..count)
        .map(|i| {
            let off = 4 + i * 8;
            parse_lump_name(&bytes[off..off + 8])
        })
        .collect()
}

/// Composite texture definition from TEXTURE1/TEXTURE2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureDef {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub patches: Vec<PatchRef>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PatchRef {
    pub origin_x: i16,
    pub origin_y: i16,
    pub patch_index: u16,
}

pub fn parse_textures(bytes: &[u8]) -> Result<Vec<TextureDef>, WadError> {
    if bytes.len() < 4 {
        return Err(WadError::TruncatedLump {
            name: "TEXTUREx".into(),
            expected: 4,
        });
    }
    let count = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let table_end = 4 + count * 4;
    if bytes.len() < table_end {
        return Err(WadError::TruncatedLump {
            name: "TEXTUREx".into(),
            expected: table_end,
        });
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let off = u32::from_le_bytes(bytes[4 + i * 4..8 + i * 4].try_into().unwrap()) as usize;
        if off + 22 > bytes.len() {
            return Err(WadError::TruncatedLump {
                name: "TEXTUREx".into(),
                expected: off + 22,
            });
        }
        let name = parse_lump_name(&bytes[off..off + 8])?;
        // bytes[8..12]: masked (unused) -- skip
        let width = u16::from_le_bytes(bytes[off + 12..off + 14].try_into().unwrap());
        let height = u16::from_le_bytes(bytes[off + 14..off + 16].try_into().unwrap());
        // bytes[16..20]: columndirectory (unused in modern engines)
        let patch_count = u16::from_le_bytes(bytes[off + 20..off + 22].try_into().unwrap()) as usize;
        let mut patches = Vec::with_capacity(patch_count);
        for p in 0..patch_count {
            let p_off = off + 22 + p * 10;
            if p_off + 10 > bytes.len() {
                return Err(WadError::TruncatedLump {
                    name: "TEXTUREx".into(),
                    expected: p_off + 10,
                });
            }
            patches.push(PatchRef {
                origin_x: i16::from_le_bytes(bytes[p_off..p_off + 2].try_into().unwrap()),
                origin_y: i16::from_le_bytes(bytes[p_off + 2..p_off + 4].try_into().unwrap()),
                patch_index: u16::from_le_bytes(bytes[p_off + 4..p_off + 6].try_into().unwrap()),
            });
            // bytes[p_off+6..p_off+10]: stepdir + colormap (unused)
        }
        out.push(TextureDef { name, width, height, patches });
    }
    Ok(out)
}
