// ABOUTME: Export the current map view as a PNG. Pure software rasterizer so it
// ABOUTME: doesn't depend on the egui frame's framebuffer or current zoom/pan.

use crate::wad::MapData;

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub width: u32,
    pub height: u32,
    pub with_grid: bool,
    pub grid_size: i32,
    pub with_vertices: bool,
    pub with_things: bool,
    pub with_thing_bboxes: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            width: 1024,
            height: 1024,
            with_grid: false,
            grid_size: 64,
            with_vertices: true,
            with_things: true,
            with_thing_bboxes: false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("map has no vertices")]
    EmptyMap,
    #[error("PNG encode failed: {0}")]
    Encode(String),
}

#[cfg(not(target_arch = "wasm32"))]
type Rgb = [u8; 3];
#[cfg(not(target_arch = "wasm32"))]
const BG: Rgb = [0, 0, 0];
#[cfg(not(target_arch = "wasm32"))]
const GRID: Rgb = [0x40, 0x40, 0x40];
#[cfg(not(target_arch = "wasm32"))]
const LINEDEF_NORMAL: Rgb = [0xC0, 0xC0, 0xC0];
#[cfg(not(target_arch = "wasm32"))]
const LINEDEF_TWO_SIDED: Rgb = [0x80, 0x80, 0x80];
#[cfg(not(target_arch = "wasm32"))]
const VERTEX_DOT: Rgb = [0xFF, 0xFF, 0xFF];
#[cfg(not(target_arch = "wasm32"))]
const THING_MARK: Rgb = [0x55, 0xFF, 0x55];
#[cfg(not(target_arch = "wasm32"))]
const BBOX: Rgb = [0x55, 0x55, 0x55];

#[cfg(not(target_arch = "wasm32"))]
struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Canvas {
    fn new(width: u32, height: u32, fill: Rgb) -> Self {
        let mut pixels = Vec::with_capacity((width * height * 3) as usize);
        for _ in 0..(width * height) {
            pixels.extend_from_slice(&fill);
        }
        Self { width, height, pixels }
    }

    fn put(&mut self, x: i32, y: i32, c: Rgb) {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return;
        }
        let idx = ((y as u32 * self.width + x as u32) * 3) as usize;
        self.pixels[idx] = c[0];
        self.pixels[idx + 1] = c[1];
        self.pixels[idx + 2] = c[2];
    }

    /// Bresenham line drawing.
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, c: Rgb) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            self.put(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn fill_rect(&mut self, cx: i32, cy: i32, half: i32, c: Rgb) {
        for dy in -half..=half {
            for dx in -half..=half {
                self.put(cx + dx, cy + dy, c);
            }
        }
    }

    fn stroke_rect(&mut self, cx: i32, cy: i32, half: i32, c: Rgb) {
        let l = cx - half;
        let r = cx + half;
        let t = cy - half;
        let b = cy + half;
        self.line(l, t, r, t, c);
        self.line(r, t, r, b, c);
        self.line(r, b, l, b, c);
        self.line(l, b, l, t, c);
    }

    fn encode_png(self) -> Result<Vec<u8>, ExportError> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, self.width, self.height);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .map_err(|e| ExportError::Encode(e.to_string()))?;
            writer
                .write_image_data(&self.pixels)
                .map_err(|e| ExportError::Encode(e.to_string()))?;
        }
        Ok(out)
    }
}

/// Render the map at the given size and return PNG bytes.
#[cfg(not(target_arch = "wasm32"))]
pub fn render(map: &MapData, opts: &ExportOptions) -> Result<Vec<u8>, ExportError> {
    if map.vertices.is_empty() {
        return Err(ExportError::EmptyMap);
    }

    // Compute world bbox from vertices and Things (so off-bbox Things don't get clipped).
    let (mut min_x, mut max_x) = (i32::MAX, i32::MIN);
    let (mut min_y, mut max_y) = (i32::MAX, i32::MIN);
    for v in &map.vertices {
        min_x = min_x.min(v.x as i32);
        max_x = max_x.max(v.x as i32);
        min_y = min_y.min(v.y as i32);
        max_y = max_y.max(v.y as i32);
    }
    if opts.with_things {
        for t in &map.things {
            min_x = min_x.min(t.x as i32);
            max_x = max_x.max(t.x as i32);
            min_y = min_y.min(t.y as i32);
            max_y = max_y.max(t.y as i32);
        }
    }

    let world_w = (max_x - min_x).max(1) as f32;
    let world_h = (max_y - min_y).max(1) as f32;
    // 5% padding on each side.
    let pad = 0.05_f32;
    let avail_w = opts.width as f32 * (1.0 - 2.0 * pad);
    let avail_h = opts.height as f32 * (1.0 - 2.0 * pad);
    let scale = (avail_w / world_w).min(avail_h / world_h);

    let cx = opts.width as f32 * 0.5;
    let cy = opts.height as f32 * 0.5;
    let world_cx = (min_x + max_x) as f32 * 0.5;
    let world_cy = (min_y + max_y) as f32 * 0.5;

    let to_px = |wx: i32, wy: i32| -> (i32, i32) {
        let sx = cx + (wx as f32 - world_cx) * scale;
        // PNG Y goes down; world Y goes up.
        let sy = cy - (wy as f32 - world_cy) * scale;
        (sx.round() as i32, sy.round() as i32)
    };

    let mut canvas = Canvas::new(opts.width, opts.height, BG);

    // Grid dots — at every grid_size world step, in image bounds.
    if opts.with_grid && opts.grid_size > 0 {
        let g = opts.grid_size;
        let start_x = (min_x / g) * g;
        let start_y = (min_y / g) * g;
        let mut wx = start_x;
        while wx <= max_x {
            let mut wy = start_y;
            while wy <= max_y {
                let (sx, sy) = to_px(wx, wy);
                canvas.put(sx, sy, GRID);
                wy += g;
            }
            wx += g;
        }
    }

    // LineDefs.
    for ld in &map.linedefs {
        let (Some(a), Some(b)) = (
            map.vertices.get(ld.start_vertex as usize),
            map.vertices.get(ld.end_vertex as usize),
        ) else { continue };
        let (ax, ay) = to_px(a.x as i32, a.y as i32);
        let (bx, by) = to_px(b.x as i32, b.y as i32);
        let color = if ld.is_two_sided() { LINEDEF_TWO_SIDED } else { LINEDEF_NORMAL };
        canvas.line(ax, ay, bx, by, color);
    }

    // Vertex dots.
    if opts.with_vertices {
        for v in &map.vertices {
            let (sx, sy) = to_px(v.x as i32, v.y as i32);
            canvas.fill_rect(sx, sy, 1, VERTEX_DOT);
        }
    }

    // Things as X markers.
    if opts.with_things {
        for t in &map.things {
            let (sx, sy) = to_px(t.x as i32, t.y as i32);
            let s = 3;
            canvas.line(sx - s, sy - s, sx + s, sy + s, THING_MARK);
            canvas.line(sx - s, sy + s, sx + s, sy - s, THING_MARK);
            if opts.with_thing_bboxes {
                let r = super::things_table::radius_of(t.thing_type) as f32 * scale;
                let half = r.round() as i32;
                if half > 0 {
                    canvas.stroke_rect(sx, sy, half, BBOX);
                }
            }
        }
    }

    canvas.encode_png()
}
