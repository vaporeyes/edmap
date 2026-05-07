// ABOUTME: Texture asset cache — lazily decodes patches/flats/composites from the active WAD.
// ABOUTME: Caches egui::TextureHandles keyed by uppercase asset name; rebuilt when WAD changes.

use std::collections::HashMap;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

use crate::wad::{
    parse_pnames, parse_textures, Flat, Palette, Patch, PatchName, TextureDef, TextureImage, Wad,
    FLAT_DIM,
};

#[derive(Default)]
pub struct TextureBank {
    pub palette: Option<Palette>,
    pub pnames: Vec<PatchName>,
    pub walls: Vec<TextureDef>,
    pub flat_names: Vec<String>,
    pub sprite_names: Vec<String>,

    handles: HashMap<String, TextureHandle>,
    /// Names we tried to decode but failed — don't retry every frame.
    failed: HashMap<String, ()>,
    /// Sprite (width, height) cache so the 3D billboard builder can size quads
    /// to real pixel dimensions without a full RGBA decode every frame.
    sprite_dims: HashMap<String, (u32, u32)>,
}

impl TextureBank {
    pub fn rebuild_from(wad: &Wad) -> Self {
        let palette = wad
            .lump_bytes_by_name("PLAYPAL")
            .and_then(|b| Palette::parse_first(b).ok());
        let pnames = wad
            .lump_bytes_by_name("PNAMES")
            .and_then(|b| parse_pnames(b).ok())
            .unwrap_or_default();
        let mut walls = Vec::new();
        for tex_lump in ["TEXTURE1", "TEXTURE2"] {
            if let Some(b) = wad.lump_bytes_by_name(tex_lump) {
                if let Ok(mut defs) = parse_textures(b) {
                    walls.append(&mut defs);
                }
            }
        }
        let flat_names = wad.lumps_between("F_START", "F_END");
        let sprite_names = wad.lumps_between("S_START", "S_END");
        Self {
            palette,
            pnames,
            walls,
            flat_names,
            sprite_names,
            handles: HashMap::new(),
            failed: HashMap::new(),
            sprite_dims: HashMap::new(),
        }
    }

    /// Sprite (width, height) lookup. Reads only the 4-byte patch header so
    /// the 3D billboard builder can size quads accurately before any pixel
    /// decode happens. Falls back to the placeholder sprite dimensions
    /// when the lump is absent or malformed.
    pub fn sprite_dims(&mut self, wad: &Wad, name: &str) -> (u32, u32) {
        if let Some(&d) = self.sprite_dims.get(name) {
            return d;
        }
        let dims = wad.lump_bytes_by_name(name)
            .filter(|b| b.len() >= 4)
            .map(|b| {
                let w = i16::from_le_bytes([b[0], b[1]]).max(1) as u32;
                let h = i16::from_le_bytes([b[2], b[3]]).max(1) as u32;
                (w, h)
            })
            .unwrap_or((16, 24)); // matches placeholder_sprite_rgba
        self.sprite_dims.insert(name.into(), dims);
        dims
    }

    /// Get-or-build a wall-texture handle. Returns None if any required lump
    /// is missing or decoding failed.
    pub fn wall(&mut self, ctx: &egui::Context, wad: &Wad, name: &str) -> Option<&TextureHandle> {
        let key = format!("W:{name}");
        if self.failed.contains_key(&key) {
            return None;
        }
        if !self.handles.contains_key(&key) {
            let color_image = self.palette.as_ref()
                .and_then(|palette| {
                    let def = self.walls.iter().find(|d| d.name == name)?.clone();
                    let pnames = self.pnames.clone();
                    let img = TextureImage::compose(&def, &pnames, |patch_name| {
                        wad.lump_bytes_by_name(patch_name).and_then(|b| Patch::parse(b).ok())
                    });
                    Some(compose_to_color_image(&img, palette))
                })
                .unwrap_or_else(|| placeholder_wall_color_image(name));
            let handle = ctx.load_texture(&key, color_image, TextureOptions::NEAREST);
            self.handles.insert(key.clone(), handle);
        }
        self.handles.get(&key)
    }

    pub fn flat(&mut self, ctx: &egui::Context, wad: &Wad, name: &str) -> Option<&TextureHandle> {
        let key = format!("F:{name}");
        if self.failed.contains_key(&key) {
            return None;
        }
        if !self.handles.contains_key(&key) {
            let color_image = self.palette.as_ref()
                .and_then(|palette| {
                    let bytes = wad.lump_bytes_by_name(name)?;
                    let flat = Flat::parse(bytes).ok()?;
                    Some(flat_to_color_image(&flat, palette))
                })
                .unwrap_or_else(|| placeholder_flat_color_image(name));
            let handle = ctx.load_texture(&key, color_image, TextureOptions::NEAREST);
            self.handles.insert(key.clone(), handle);
        }
        self.handles.get(&key)
    }

    /// Decode a sprite (Patch lump between S_START/S_END) to raw RGBA8 with
    /// real alpha for the transparent posts. Returns (w, h, bytes).
    pub fn sprite_rgba(&self, wad: &Wad, name: &str) -> Option<(u32, u32, Vec<u8>)> {
        let palette = self.palette.as_ref()?;
        let Some(bytes) = wad.lump_bytes_by_name(name) else {
            return Some(placeholder_sprite_rgba(name));
        };
        let Some(patch) = Patch::parse(bytes).ok() else {
            return Some(placeholder_sprite_rgba(name));
        };
        let w = patch.width as u32;
        let h = patch.height as u32;
        let mut out = Vec::with_capacity((w as usize) * (h as usize) * 4);
        for &p in &patch.pixels {
            match p {
                Some(idx) => {
                    let [r, g, b] = palette.0.get(idx as usize).copied().unwrap_or([0, 0, 0]);
                    out.extend_from_slice(&[r, g, b, 255]);
                }
                None => out.extend_from_slice(&[0, 0, 0, 0]),
            }
        }
        Some((w, h, out))
    }

    /// Decode a flat (floor/ceiling) lump to raw RGBA8 pixel data.
    /// FLATs are always 64×64; returns (64, 64, bytes) on success.
    pub fn flat_rgba(&self, wad: &Wad, name: &str) -> Option<(u32, u32, Vec<u8>)> {
        let palette = self.palette.as_ref()?;
        let Some(bytes) = wad.lump_bytes_by_name(name) else {
            return Some(placeholder_flat_rgba(name));
        };
        let Some(flat) = Flat::parse(bytes).ok() else {
            return Some(placeholder_flat_rgba(name));
        };
        let mut out = Vec::with_capacity(FLAT_DIM * FLAT_DIM * 4);
        for &idx in &flat.pixels {
            let [r, g, b] = palette.0.get(idx as usize).copied().unwrap_or([0, 0, 0]);
            out.extend_from_slice(&[r, g, b, 255]);
        }
        Some((FLAT_DIM as u32, FLAT_DIM as u32, out))
    }

    /// Decode a wall texture to raw RGBA8 pixel data (for the 3D GL renderer).
    /// Bypasses the egui handle cache entirely — caller is responsible for
    /// uploading the bytes to GL and caching the result on its side.
    pub fn wall_rgba(&self, wad: &Wad, name: &str) -> Option<(u32, u32, Vec<u8>)> {
        let palette = self.palette.as_ref()?;
        let Some(def) = self.walls.iter().find(|d| d.name == name) else {
            return Some(placeholder_wall_rgba(name));
        };
        let img = TextureImage::compose(def, &self.pnames, |patch_name| {
            wad.lump_bytes_by_name(patch_name).and_then(|b| Patch::parse(b).ok())
        });
        let w = img.width as u32;
        let h = img.height as u32;
        let mut bytes = Vec::with_capacity((w as usize) * (h as usize) * 4);
        for &p in &img.pixels {
            match p {
                Some(idx) => {
                    let [r, g, b] = palette.0.get(idx as usize).copied().unwrap_or([0, 0, 0]);
                    bytes.extend_from_slice(&[r, g, b, 255]);
                }
                None => bytes.extend_from_slice(&[0, 0, 0, 0]),
            }
        }
        Some((w, h, bytes))
    }

    pub fn sprite(&mut self, ctx: &egui::Context, wad: &Wad, name: &str) -> Option<&TextureHandle> {
        // Sprites are stored as patches between S_START..S_END.
        let key = format!("S:{name}");
        if self.failed.contains_key(&key) {
            return None;
        }
        if !self.handles.contains_key(&key) {
            let color_image = self.palette.as_ref()
                .and_then(|palette| {
                    let bytes = wad.lump_bytes_by_name(name)?;
                    let patch = Patch::parse(bytes).ok()?;
                    Some(patch_to_color_image(&patch, palette))
                })
                .unwrap_or_else(|| placeholder_sprite_color_image(name));
            let handle = ctx.load_texture(&key, color_image, TextureOptions::NEAREST);
            self.handles.insert(key.clone(), handle);
        }
        self.handles.get(&key)
    }
}

fn compose_to_color_image(img: &TextureImage, palette: &Palette) -> ColorImage {
    let w = img.width as usize;
    let h = img.height as usize;
    let mut pixels = Vec::with_capacity(w * h);
    for &p in &img.pixels {
        pixels.push(match p {
            Some(idx) => index_to_color(idx, palette, false),
            None => egui::Color32::TRANSPARENT,
        });
    }
    ColorImage { size: [w, h], pixels }
}

fn flat_to_color_image(flat: &Flat, palette: &Palette) -> ColorImage {
    let mut pixels = Vec::with_capacity(FLAT_DIM * FLAT_DIM);
    for &idx in &flat.pixels {
        pixels.push(index_to_color(idx, palette, true));
    }
    ColorImage { size: [FLAT_DIM, FLAT_DIM], pixels }
}

fn patch_to_color_image(patch: &Patch, palette: &Palette) -> ColorImage {
    let w = patch.width as usize;
    let h = patch.height as usize;
    let mut pixels = Vec::with_capacity(w * h);
    for &p in &patch.pixels {
        pixels.push(match p {
            Some(idx) => index_to_color(idx, palette, false),
            None => egui::Color32::TRANSPARENT,
        });
    }
    ColorImage { size: [w, h], pixels }
}

fn index_to_color(idx: u8, palette: &Palette, opaque: bool) -> egui::Color32 {
    let [r, g, b] = palette.0.get(idx as usize).copied().unwrap_or([0, 0, 0]);
    if opaque {
        egui::Color32::from_rgb(r, g, b)
    } else {
        egui::Color32::from_rgba_unmultiplied(r, g, b, 255)
    }
}

/// Stable 24-bit hash of an asset name; used to colour placeholder textures
/// so different missing names look visually distinct.
fn name_hash(name: &str) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for &b in name.as_bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    h
}

fn placeholder_colors(name: &str) -> ([u8; 3], [u8; 3]) {
    let h = name_hash(name);
    let a = [
        ((h >> 16) & 0xFF) as u8 | 0x40,
        ((h >> 8) & 0xFF) as u8 | 0x40,
        (h & 0xFF) as u8 | 0x40,
    ];
    let b = [a[0] / 2, a[1] / 2, a[2] / 2];
    (a, b)
}

/// 64x64 RGBA checkerboard tinted by the missing texture's name hash.
fn placeholder_wall_rgba(name: &str) -> (u32, u32, Vec<u8>) {
    let (ca, cb) = placeholder_colors(name);
    let dim = 64usize;
    let mut out = Vec::with_capacity(dim * dim * 4);
    for y in 0..dim {
        for x in 0..dim {
            let c = if ((x / 8) + (y / 8)) & 1 == 0 { ca } else { cb };
            out.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    (dim as u32, dim as u32, out)
}

/// 64x64 placeholder using a different pattern so flats are visually
/// distinguishable from walls when both are missing.
fn placeholder_flat_rgba(name: &str) -> (u32, u32, Vec<u8>) {
    let (ca, cb) = placeholder_colors(name);
    let dim = 64usize;
    let mut out = Vec::with_capacity(dim * dim * 4);
    for y in 0..dim {
        for x in 0..dim {
            // Concentric rings, 8px apart.
            let dx = x as i32 - 32;
            let dy = y as i32 - 32;
            let r = ((dx * dx + dy * dy) as f32).sqrt() as i32;
            let c = if (r / 6) & 1 == 0 { ca } else { cb };
            out.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    (dim as u32, dim as u32, out)
}

/// Small (16x24) placeholder sprite. Filled rectangle with a contrasting
/// border so things are visible as upright billboards.
fn placeholder_sprite_rgba(name: &str) -> (u32, u32, Vec<u8>) {
    let (ca, cb) = placeholder_colors(name);
    let w = 16usize;
    let h = 24usize;
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let edge = x == 0 || y == 0 || x == w - 1 || y == h - 1;
            let c = if edge { cb } else { ca };
            out.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    (w as u32, h as u32, out)
}

/// Convert RGBA bytes from the placeholder generators into an egui ColorImage
/// so the 2D viewport can render the same placeholder visuals as the 3D view.
fn rgba_to_color_image(w: u32, h: u32, bytes: &[u8]) -> ColorImage {
    let pixels = bytes
        .chunks_exact(4)
        .map(|c| egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]))
        .collect();
    ColorImage { size: [w as usize, h as usize], pixels }
}

fn placeholder_wall_color_image(name: &str) -> ColorImage {
    let (w, h, bytes) = placeholder_wall_rgba(name);
    rgba_to_color_image(w, h, &bytes)
}

fn placeholder_flat_color_image(name: &str) -> ColorImage {
    let (w, h, bytes) = placeholder_flat_rgba(name);
    rgba_to_color_image(w, h, &bytes)
}

fn placeholder_sprite_color_image(name: &str) -> ColorImage {
    let (w, h, bytes) = placeholder_sprite_rgba(name);
    rgba_to_color_image(w, h, &bytes)
}
