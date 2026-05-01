// ABOUTME: Map validation engine — runs a set of checks against MapData and returns issues.
// ABOUTME: Messages chosen verbatim from EdMap's binary string table for fidelity.

use crate::wad::{LineDef, MapData};

use super::state::SelectionMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

/// One detected issue in the map.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub severity: Severity,
    pub label: String,
    pub message: String,
    /// Optional pointer to the offending object so the user can Goto it.
    pub at: Option<(SelectionMode, usize)>,
}

/// Knob for which set to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckSet {
    Quick,
    All,
}

/// EdMap-derived thresholds. Approximations until DOSBox-X bisection pins them.
const SHORT_LINE_THRESHOLD: f32 = 4.0;
const LONG_LINE_THRESHOLD: f32 = 1024.0;

/// DOOM exit-action linedef specials. (Source: public DOOM linedef-actions table.)
/// Codes 11 and 51 = normal exit / secret exit (DOOM I/II).
/// Codes 52 and 124 = walk-over exits.
const EXIT_SPECIALS: &[u16] = &[11, 51, 52, 124];

/// Player 1 start has thing-type 1 in the DOOM thing table.
const PLAYER1_START: u16 = 1;

pub fn run(map: &MapData, set: CheckSet) -> Vec<CheckResult> {
    let mut out = Vec::new();
    check_linedefs(map, set, &mut out);
    check_sidedef_flag_coherence(map, &mut out);
    if set == CheckSet::All {
        check_two_sided_textures(map, &mut out);
    }
    check_player_starts(map, &mut out);
    check_exit(map, &mut out);
    out
}

fn check_linedefs(map: &MapData, _set: CheckSet, out: &mut Vec<CheckResult>) {
    for (i, ld) in map.linedefs.iter().enumerate() {
        let (Some(a), Some(b)) = (
            map.vertices.get(ld.start_vertex as usize),
            map.vertices.get(ld.end_vertex as usize),
        ) else {
            out.push(CheckResult {
                severity: Severity::Error,
                label: "Invalid LineDef".into(),
                message: format!("LineDef {i} references missing vertex."),
                at: Some((SelectionMode::LineDef, i)),
            });
            continue;
        };
        let dx = (a.x - b.x) as f32;
        let dy = (a.y - b.y) as f32;
        let len = (dx * dx + dy * dy).sqrt();
        if len == 0.0 {
            out.push(CheckResult {
                severity: Severity::Error,
                label: "Bad LineDef length (0)".into(),
                message: "This LineDef has no length.".into(),
                at: Some((SelectionMode::LineDef, i)),
            });
        } else if len < SHORT_LINE_THRESHOLD {
            out.push(CheckResult {
                severity: Severity::Warning,
                label: "Bad LineDef length (short)".into(),
                message: "This LineDef is too short.".into(),
                at: Some((SelectionMode::LineDef, i)),
            });
        } else if len > LONG_LINE_THRESHOLD {
            out.push(CheckResult {
                severity: Severity::Warning,
                label: "Warning: long LineDef".into(),
                message: format!("LineDef {i} length {len:.1} exceeds {LONG_LINE_THRESHOLD:.0}."),
                at: Some((SelectionMode::LineDef, i)),
            });
        }
    }
}

fn check_sidedef_flag_coherence(map: &MapData, out: &mut Vec<CheckResult>) {
    for (i, ld) in map.linedefs.iter().enumerate() {
        let two_sided_bit = ld.flags & LineDef::FLAG_TWO_SIDED != 0;
        let has_back = ld.back_sidedef != LineDef::NO_SIDEDEF;
        if two_sided_bit && !has_back {
            out.push(CheckResult {
                severity: Severity::Error,
                label: "2S bit set, no 2nd SideDef".into(),
                message: format!("LineDef {i} has the two-sided flag set but no back SideDef."),
                at: Some((SelectionMode::LineDef, i)),
            });
        } else if !two_sided_bit && has_back {
            out.push(CheckResult {
                severity: Severity::Error,
                label: "2nd SideDef, 2S bit is off".into(),
                message: format!("LineDef {i} has a back SideDef but the two-sided flag is off."),
                at: Some((SelectionMode::LineDef, i)),
            });
        }
    }
}

fn check_two_sided_textures(map: &MapData, out: &mut Vec<CheckResult>) {
    // For a 2-sided line whose adjoining sectors differ in floor or ceiling
    // height, the upper/lower texture on each side must be present, otherwise
    // DOOM renders the gap as Hall-of-Mirrors.
    for (i, ld) in map.linedefs.iter().enumerate() {
        if !ld.is_two_sided() || ld.back_sidedef == LineDef::NO_SIDEDEF {
            continue;
        }
        let (Some(front), Some(back)) = (
            map.sidedefs.get(ld.front_sidedef as usize),
            map.sidedefs.get(ld.back_sidedef as usize),
        ) else { continue };
        let (Some(fs), Some(bs)) = (
            map.sectors.get(front.sector as usize),
            map.sectors.get(back.sector as usize),
        ) else { continue };

        if bs.ceiling_height < fs.ceiling_height && is_blank(&front.upper_texture) {
            out.push(CheckResult {
                severity: Severity::Error,
                label: "Missing upper texture".into(),
                message: format!(
                    "LineDef {i} front needs an upper texture (ceiling height differs)."
                ),
                at: Some((SelectionMode::LineDef, i)),
            });
        }
        if fs.ceiling_height < bs.ceiling_height && is_blank(&back.upper_texture) {
            out.push(CheckResult {
                severity: Severity::Error,
                label: "Missing upper texture".into(),
                message: format!(
                    "LineDef {i} back needs an upper texture (ceiling height differs)."
                ),
                at: Some((SelectionMode::LineDef, i)),
            });
        }
        if bs.floor_height > fs.floor_height && is_blank(&front.lower_texture) {
            out.push(CheckResult {
                severity: Severity::Error,
                label: "Missing lower texture".into(),
                message: format!(
                    "LineDef {i} front needs a lower texture (floor height differs)."
                ),
                at: Some((SelectionMode::LineDef, i)),
            });
        }
        if fs.floor_height > bs.floor_height && is_blank(&back.lower_texture) {
            out.push(CheckResult {
                severity: Severity::Error,
                label: "Missing lower texture".into(),
                message: format!(
                    "LineDef {i} back needs a lower texture (floor height differs)."
                ),
                at: Some((SelectionMode::LineDef, i)),
            });
        }
    }
}

fn is_blank(name: &str) -> bool {
    name.is_empty() || name == "-"
}

fn check_player_starts(map: &MapData, out: &mut Vec<CheckResult>) {
    let starts: Vec<usize> = map
        .things
        .iter()
        .enumerate()
        .filter_map(|(i, t)| if t.thing_type == PLAYER1_START { Some(i) } else { None })
        .collect();
    match starts.len() {
        0 => out.push(CheckResult {
            severity: Severity::Error,
            label: "Missing Start-x things".into(),
            message: "No Player 1 start found.".into(),
            at: None,
        }),
        1 => {}
        _ => {
            for &idx in &starts[1..] {
                out.push(CheckResult {
                    severity: Severity::Error,
                    label: "Multiple Start-1 things".into(),
                    message: "Only one Player 1 start is allowed.".into(),
                    at: Some((SelectionMode::Thing, idx)),
                });
            }
        }
    }
}

fn check_exit(map: &MapData, out: &mut Vec<CheckResult>) {
    let has_exit = map
        .linedefs
        .iter()
        .any(|ld| EXIT_SPECIALS.contains(&ld.special_type));
    if !has_exit {
        out.push(CheckResult {
            severity: Severity::Error,
            label: "No exit".into(),
            message: "This map has no exit.".into(),
            at: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wad::{LineDef, MapData, SideDef, Vertex};

    fn empty_map() -> MapData {
        MapData {
            name: "TEST".into(),
            vertices: vec![],
            linedefs: vec![],
            sidedefs: vec![],
            sectors: vec![],
            things: vec![],
        }
    }

    #[test]
    fn empty_map_reports_missing_start_and_exit() {
        let r = run(&empty_map(), CheckSet::Quick);
        assert!(r.iter().any(|c| c.label.contains("Start")));
        assert!(r.iter().any(|c| c.label.contains("exit")));
    }

    #[test]
    fn detects_zero_length_linedef() {
        let mut m = empty_map();
        m.vertices = vec![Vertex { x: 100, y: 100 }];
        m.linedefs = vec![LineDef {
            start_vertex: 0,
            end_vertex: 0,
            flags: 0,
            special_type: 0,
            sector_tag: 0,
            front_sidedef: LineDef::NO_SIDEDEF,
            back_sidedef: LineDef::NO_SIDEDEF,
        }];
        let r = run(&m, CheckSet::Quick);
        assert!(r.iter().any(|c| c.label == "Bad LineDef length (0)"));
    }

    #[test]
    fn detects_2s_flag_without_back_sidedef() {
        let mut m = empty_map();
        m.vertices = vec![Vertex { x: 0, y: 0 }, Vertex { x: 64, y: 0 }];
        m.sidedefs = vec![SideDef {
            x_offset: 0,
            y_offset: 0,
            upper_texture: "-".into(),
            lower_texture: "-".into(),
            middle_texture: "-".into(),
            sector: 0,
        }];
        m.linedefs = vec![LineDef {
            start_vertex: 0,
            end_vertex: 1,
            flags: LineDef::FLAG_TWO_SIDED,
            special_type: 0,
            sector_tag: 0,
            front_sidedef: 0,
            back_sidedef: LineDef::NO_SIDEDEF,
        }];
        let r = run(&m, CheckSet::Quick);
        assert!(r.iter().any(|c| c.label == "2S bit set, no 2nd SideDef"));
    }
}
