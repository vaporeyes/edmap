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

/// A DOOM `patch_t` decoded into 2D indexed pixels. `None` means transparent.
/// Posts are merged into the column buffer; column gaps stay transparent.
#[derive(Debug, Clone)]
pub struct Patch {
    pub width: u16,
    pub height: u16,
    pub left_offset: i16,
    pub top_offset: i16,
    /// Row-major width*height palette indices. `None` = transparent pixel.
    pub pixels: Vec<Option<u8>>,
}

impl Patch {
    pub fn parse(bytes: &[u8]) -> Result<Self, WadError> {
        if bytes.len() < 8 {
            return Err(WadError::TruncatedLump {
                name: "patch".into(),
                expected: 8,
            });
        }
        let width = u16::from_le_bytes(bytes[0..2].try_into().unwrap());
        let height = u16::from_le_bytes(bytes[2..4].try_into().unwrap());
        let left_offset = i16::from_le_bytes(bytes[4..6].try_into().unwrap());
        let top_offset = i16::from_le_bytes(bytes[6..8].try_into().unwrap());

        let cols_table_off = 8usize;
        let cols_table_end = cols_table_off
            .checked_add(width as usize * 4)
            .ok_or_else(|| WadError::TruncatedLump {
                name: "patch column table".into(),
                expected: usize::MAX,
            })?;
        if bytes.len() < cols_table_end {
            return Err(WadError::TruncatedLump {
                name: "patch column table".into(),
                expected: cols_table_end,
            });
        }

        let mut pixels = vec![None; width as usize * height as usize];
        for col in 0..width as usize {
            let off_bytes = &bytes[cols_table_off + col * 4..cols_table_off + col * 4 + 4];
            let mut p = u32::from_le_bytes(off_bytes.try_into().unwrap()) as usize;
            // Walk posts. Each post: topdelta(u8), length(u8), pad(u8), len bytes pixels, pad(u8).
            // Terminator: topdelta == 0xFF.
            loop {
                if p >= bytes.len() {
                    break;
                }
                let topdelta = bytes[p];
                if topdelta == 0xFF {
                    break;
                }
                if p + 2 >= bytes.len() {
                    break;
                }
                let length = bytes[p + 1] as usize;
                // p + 2 is unused padding; pixels start at p + 3.
                let data_start = p + 3;
                let data_end = data_start + length;
                if data_end > bytes.len() {
                    break;
                }
                for row in 0..length {
                    let y = topdelta as usize + row;
                    if y < height as usize {
                        let idx = y * width as usize + col;
                        pixels[idx] = Some(bytes[data_start + row]);
                    }
                }
                p = data_end + 1; // skip trailing padding byte
            }
        }

        Ok(Self {
            width,
            height,
            left_offset,
            top_offset,
            pixels,
        })
    }
}

/// 64x64 floor/ceiling flat — raw palette indices, row-major.
pub const FLAT_DIM: usize = 64;

#[derive(Debug, Clone)]
pub struct Flat {
    pub pixels: Vec<u8>, // length FLAT_DIM * FLAT_DIM
}

impl Flat {
    pub fn parse(bytes: &[u8]) -> Result<Self, WadError> {
        if bytes.len() < FLAT_DIM * FLAT_DIM {
            return Err(WadError::TruncatedLump {
                name: "flat".into(),
                expected: FLAT_DIM * FLAT_DIM,
            });
        }
        Ok(Self { pixels: bytes[..FLAT_DIM * FLAT_DIM].to_vec() })
    }
}

/// Rasterized composite texture: row-major width*height palette indices,
/// `None` for transparent pixels (some DOOM textures have holes).
#[derive(Debug, Clone)]
pub struct TextureImage {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<Option<u8>>,
}

impl TextureImage {
    /// Compose by blitting each patch (resolved by index against `pnames`)
    /// at its origin into a fresh transparent canvas.
    pub fn compose<F>(def: &TextureDef, pnames: &[PatchName], mut load_patch: F) -> Self
    where
        F: FnMut(&str) -> Option<Patch>,
    {
        let w = def.width as usize;
        let h = def.height as usize;
        let mut pixels = vec![None; w * h];
        for pref in &def.patches {
            let Some(name) = pnames.get(pref.patch_index as usize) else { continue };
            let Some(patch) = load_patch(name) else { continue };
            blit(
                &patch,
                pref.origin_x as i32,
                pref.origin_y as i32,
                w as i32,
                h as i32,
                &mut pixels,
            );
        }
        Self { width: def.width, height: def.height, pixels }
    }
}

fn blit(patch: &Patch, ox: i32, oy: i32, dst_w: i32, dst_h: i32, dst: &mut [Option<u8>]) {
    for sy in 0..patch.height as i32 {
        let dy = oy + sy;
        if dy < 0 || dy >= dst_h {
            continue;
        }
        for sx in 0..patch.width as i32 {
            let dx = ox + sx;
            if dx < 0 || dx >= dst_w {
                continue;
            }
            let src = patch.pixels[(sy * patch.width as i32 + sx) as usize];
            if src.is_some() {
                dst[(dy * dst_w + dx) as usize] = src;
            }
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal 2x2 patch with one post per column. Verifies offsets, posts,
    /// transparent gaps, and palette mapping into TextureImage.
    fn synthesize_patch_bytes() -> Vec<u8> {
        // Header: w=2, h=3, leftoff=0, topoff=0
        let mut b = Vec::new();
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&3u16.to_le_bytes());
        b.extend_from_slice(&0i16.to_le_bytes());
        b.extend_from_slice(&0i16.to_le_bytes());
        // Two column offsets — placeholders, fill later.
        let col_table_off = b.len();
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        // Column 0: one post starting at row 0, length 2 (rows 0,1 set; row 2 transparent).
        let col0_start = b.len() as u32;
        b.push(0x00); // topdelta
        b.push(0x02); // length
        b.push(0x00); // padding
        b.push(0x10); // pixel row 0
        b.push(0x11); // pixel row 1
        b.push(0x00); // trailing padding
        b.push(0xFF); // terminator
        // Column 1: one post at row 1, length 2 (rows 1,2 set; row 0 transparent).
        let col1_start = b.len() as u32;
        b.push(0x01); // topdelta
        b.push(0x02);
        b.push(0x00);
        b.push(0x20);
        b.push(0x21);
        b.push(0x00);
        b.push(0xFF);
        // Patch column-table offsets.
        b[col_table_off..col_table_off + 4].copy_from_slice(&col0_start.to_le_bytes());
        b[col_table_off + 4..col_table_off + 8].copy_from_slice(&col1_start.to_le_bytes());
        b
    }

    #[test]
    fn parses_patch_with_transparent_gaps() {
        let bytes = synthesize_patch_bytes();
        let patch = Patch::parse(&bytes).expect("parse patch");
        assert_eq!(patch.width, 2);
        assert_eq!(patch.height, 3);
        // Layout: row-major (y * width + x)
        assert_eq!(patch.pixels[0 * 2 + 0], Some(0x10)); // col 0, row 0
        assert_eq!(patch.pixels[1 * 2 + 0], Some(0x11)); // col 0, row 1
        assert_eq!(patch.pixels[2 * 2 + 0], None);       // col 0, row 2 (gap)
        assert_eq!(patch.pixels[0 * 2 + 1], None);       // col 1, row 0 (gap)
        assert_eq!(patch.pixels[1 * 2 + 1], Some(0x20)); // col 1, row 1
        assert_eq!(patch.pixels[2 * 2 + 1], Some(0x21)); // col 1, row 2
    }

    #[test]
    fn composes_texture_from_two_offset_patches() {
        // 4x3 composite, two 2x3 patches placed at x=0 and x=2.
        let pa = Patch::parse(&synthesize_patch_bytes()).unwrap();
        let pb = Patch::parse(&synthesize_patch_bytes()).unwrap();
        let def = TextureDef {
            name: "TEST".into(),
            width: 4,
            height: 3,
            patches: vec![
                PatchRef { origin_x: 0, origin_y: 0, patch_index: 0 },
                PatchRef { origin_x: 2, origin_y: 0, patch_index: 1 },
            ],
        };
        let pnames = vec!["A".to_string(), "B".to_string()];
        let img = TextureImage::compose(&def, &pnames, |name| match name {
            "A" => Some(pa.clone()),
            "B" => Some(pb.clone()),
            _ => None,
        });
        assert_eq!(img.width, 4);
        assert_eq!(img.height, 3);
        // The right-side patch's column 0 should land at composite x=2.
        assert_eq!(img.pixels[0 * 4 + 2], Some(0x10));
        assert_eq!(img.pixels[1 * 4 + 2], Some(0x11));
    }

    #[test]
    fn parses_flat_64x64() {
        let mut bytes = vec![0xAB; FLAT_DIM * FLAT_DIM];
        bytes[0] = 0x01;
        bytes[FLAT_DIM * FLAT_DIM - 1] = 0xFF;
        let flat = Flat::parse(&bytes).expect("parse flat");
        assert_eq!(flat.pixels.len(), FLAT_DIM * FLAT_DIM);
        assert_eq!(flat.pixels[0], 0x01);
        assert_eq!(flat.pixels[flat.pixels.len() - 1], 0xFF);
    }
}
