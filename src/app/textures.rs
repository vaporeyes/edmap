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
        }
    }

    /// Get-or-build a wall-texture handle. Returns None if any required lump
    /// is missing or decoding failed.
    pub fn wall(&mut self, ctx: &egui::Context, wad: &Wad, name: &str) -> Option<&TextureHandle> {
        let key = format!("W:{name}");
        if self.failed.contains_key(&key) {
            return None;
        }
        if !self.handles.contains_key(&key) {
            let palette = self.palette.as_ref()?;
            let def = self.walls.iter().find(|d| d.name == name)?.clone();
            let pnames = self.pnames.clone();
            let img = TextureImage::compose(&def, &pnames, |patch_name| {
                wad.lump_bytes_by_name(patch_name).and_then(|b| Patch::parse(b).ok())
            });
            let color_image = compose_to_color_image(&img, palette);
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
            let palette = self.palette.as_ref()?;
            let bytes = wad.lump_bytes_by_name(name)?;
            let flat = Flat::parse(bytes).ok()?;
            let color_image = flat_to_color_image(&flat, palette);
            let handle = ctx.load_texture(&key, color_image, TextureOptions::NEAREST);
            self.handles.insert(key.clone(), handle);
        }
        self.handles.get(&key)
    }

    /// Decode a flat (floor/ceiling) lump to raw RGBA8 pixel data.
    /// FLATs are always 64×64; returns (64, 64, bytes) on success.
    pub fn flat_rgba(&self, wad: &Wad, name: &str) -> Option<(u32, u32, Vec<u8>)> {
        let palette = self.palette.as_ref()?;
        let bytes = wad.lump_bytes_by_name(name)?;
        let flat = Flat::parse(bytes).ok()?;
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
        let def = self.walls.iter().find(|d| d.name == name)?;
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
            let palette = self.palette.as_ref()?;
            let bytes = wad.lump_bytes_by_name(name)?;
            let patch = Patch::parse(bytes).ok()?;
            let color_image = patch_to_color_image(&patch, palette);
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
