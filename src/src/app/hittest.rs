// ABOUTME: Hit-testing for viewport selection — finds the nearest object to a world-space cursor.
// ABOUTME: Tolerance is in world units; the viewport derives it from a screen-pixel budget / zoom.

use crate::wad::{LineDef, MapData};

/// Find the closest vertex within `tolerance` world units. Returns the index
/// or None if nothing is close enough.
pub fn nearest_vertex(map: &MapData, cursor: (f32, f32), tolerance: f32) -> Option<usize> {
    let (cx, cy) = cursor;
    let tol2 = tolerance * tolerance;
    let mut best: Option<(usize, f32)> = None;
    for (i, v) in map.vertices.iter().enumerate() {
        let dx = v.x as f32 - cx;
        let dy = v.y as f32 - cy;
        let d2 = dx * dx + dy * dy;
        if d2 > tol2 {
            continue;
        }
        match best {
            Some((_, bd)) if bd <= d2 => {}
            _ => best = Some((i, d2)),
        }
    }
    best.map(|(i, _)| i)
}

/// Find the closest LineDef segment within `tolerance` world units.
pub fn nearest_linedef(map: &MapData, cursor: (f32, f32), tolerance: f32) -> Option<usize> {
    let (cx, cy) = cursor;
    let tol2 = tolerance * tolerance;
    let mut best: Option<(usize, f32)> = None;
    for (i, ld) in map.linedefs.iter().enumerate() {
        let (Some(a), Some(b)) = (
            map.vertices.get(ld.start_vertex as usize),
            map.vertices.get(ld.end_vertex as usize),
        ) else { continue };
        let d2 = point_to_segment_sq(
            cx,
            cy,
            a.x as f32,
            a.y as f32,
            b.x as f32,
            b.y as f32,
        );
        if d2 > tol2 {
            continue;
        }
        match best {
            Some((_, bd)) if bd <= d2 => {}
            _ => best = Some((i, d2)),
        }
    }
    best.map(|(i, _)| i)
}

/// Find the closest Thing center within `tolerance` world units.
pub fn nearest_thing(map: &MapData, cursor: (f32, f32), tolerance: f32) -> Option<usize> {
    let (cx, cy) = cursor;
    let tol2 = tolerance * tolerance;
    let mut best: Option<(usize, f32)> = None;
    for (i, t) in map.things.iter().enumerate() {
        let dx = t.x as f32 - cx;
        let dy = t.y as f32 - cy;
        let d2 = dx * dx + dy * dy;
        if d2 > tol2 {
            continue;
        }
        match best {
            Some((_, bd)) if bd <= d2 => {}
            _ => best = Some((i, d2)),
        }
    }
    best.map(|(i, _)| i)
}

/// Find the sector under `cursor`. Strategy: locate the closest LineDef, then
/// pick its front- or back-facing sector based on which side of the line the
/// cursor sits on. Falls back to None if no LineDef is within `tolerance`.
pub fn sector_under(map: &MapData, cursor: (f32, f32), tolerance: f32) -> Option<usize> {
    let ld_idx = nearest_linedef(map, cursor, tolerance)?;
    let ld = map.linedefs.get(ld_idx)?;
    let a = map.vertices.get(ld.start_vertex as usize)?;
    let b = map.vertices.get(ld.end_vertex as usize)?;

    // 2D cross product: (b - a) × (cursor - a). Positive => cursor on left side
    // of a→b, which is the FRONT side per DOOM convention (front sidedef).
    let (cx, cy) = cursor;
    let cross = (b.x as f32 - a.x as f32) * (cy - a.y as f32)
        - (b.y as f32 - a.y as f32) * (cx - a.x as f32);
    let want_front = cross >= 0.0;

    let sd_idx = if want_front { ld.front_sidedef } else { ld.back_sidedef };
    if sd_idx == LineDef::NO_SIDEDEF {
        // Cursor is on the side that has no sidedef — try the other side.
        let other = if want_front { ld.back_sidedef } else { ld.front_sidedef };
        if other == LineDef::NO_SIDEDEF {
            return None;
        }
        return map.sidedefs.get(other as usize).map(|sd| sd.sector as usize);
    }
    map.sidedefs.get(sd_idx as usize).map(|sd| sd.sector as usize)
}

/// Squared Euclidean distance from point (px,py) to segment (ax,ay)→(bx,by).
fn point_to_segment_sq(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = bx - ax;
    let dy = by - ay;
    let len2 = dx * dx + dy * dy;
    if len2 < f32::EPSILON {
        let ex = px - ax;
        let ey = py - ay;
        return ex * ex + ey * ey;
    }
    let t = ((px - ax) * dx + (py - ay) * dy) / len2;
    let t = t.clamp(0.0, 1.0);
    let qx = ax + t * dx;
    let qy = ay + t * dy;
    let ex = px - qx;
    let ey = py - qy;
    ex * ex + ey * ey
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wad::{LineDef, MapData, SideDef, Vertex};

    fn make_simple_map() -> MapData {
        // Square sector 0..256 with one sidedef per linedef facing sector 0.
        let vertices = vec![
            Vertex { x: 0, y: 0 },
            Vertex { x: 256, y: 0 },
            Vertex { x: 256, y: 256 },
            Vertex { x: 0, y: 256 },
        ];
        let mk_ld = |sv, ev| LineDef {
            start_vertex: sv,
            end_vertex: ev,
            flags: 0,
            special_type: 0,
            sector_tag: 0,
            front_sidedef: 0,
            back_sidedef: LineDef::NO_SIDEDEF,
        };
        let linedefs = vec![mk_ld(0, 1), mk_ld(1, 2), mk_ld(2, 3), mk_ld(3, 0)];
        let sidedefs = vec![SideDef {
            x_offset: 0,
            y_offset: 0,
            upper_texture: "-".into(),
            lower_texture: "-".into(),
            middle_texture: "STARTAN2".into(),
            sector: 0,
        }];
        MapData {
            name: "TEST".into(),
            vertices,
            linedefs,
            sidedefs,
            sectors: vec![],
            things: vec![],
        }
    }

    #[test]
    fn vertex_picks_within_tolerance() {
        let map = make_simple_map();
        // (4,3) is sqrt(25)=5 away from (0,0) → exactly at tolerance.
        assert_eq!(nearest_vertex(&map, (4.0, 3.0), 5.0), Some(0));
        // (258,2) is sqrt(8) ≈ 2.83 away from (256,0) → well within tolerance 5.
        assert_eq!(nearest_vertex(&map, (258.0, 2.0), 5.0), Some(1));
        assert_eq!(nearest_vertex(&map, (128.0, 128.0), 5.0), None);
    }

    #[test]
    fn linedef_picks_segment_midpoint() {
        let map = make_simple_map();
        // Cursor near middle of bottom edge (linedef 0).
        assert_eq!(nearest_linedef(&map, (128.0, 1.0), 5.0), Some(0));
        // Cursor far from any line.
        assert_eq!(nearest_linedef(&map, (128.0, 128.0), 5.0), None);
    }

    #[test]
    fn point_to_segment_endpoint_clamp() {
        // (px, py) before segment start: distance is to the start point.
        let d2 = point_to_segment_sq(-10.0, 0.0, 0.0, 0.0, 100.0, 0.0);
        assert!((d2 - 100.0).abs() < 1e-3);
        // After segment end.
        let d2 = point_to_segment_sq(110.0, 0.0, 0.0, 0.0, 100.0, 0.0);
        assert!((d2 - 100.0).abs() < 1e-3);
    }
}
