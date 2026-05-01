// ABOUTME: DOOM map lump record types: THINGS, LINEDEFS, SIDEDEFS, VERTEXES, SECTORS.
// ABOUTME: Sized per the public DOOM WAD spec; doomwiki.org/wiki/WAD.

use serde::{Deserialize, Serialize};

use super::header::parse_lump_name;
use super::WadError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Vertex {
    pub x: i16,
    pub y: i16,
}

impl Vertex {
    pub const SIZE: usize = 4;

    pub fn parse_all(bytes: &[u8]) -> Result<Vec<Self>, WadError> {
        if bytes.len() % Self::SIZE != 0 {
            return Err(WadError::TruncatedLump {
                name: "VERTEXES".into(),
                expected: bytes.len().next_multiple_of(Self::SIZE),
            });
        }
        Ok(bytes
            .chunks_exact(Self::SIZE)
            .map(|c| Self {
                x: i16::from_le_bytes([c[0], c[1]]),
                y: i16::from_le_bytes([c[2], c[3]]),
            })
            .collect())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LineDef {
    pub start_vertex: u16,
    pub end_vertex: u16,
    pub flags: u16,
    pub special_type: u16,
    pub sector_tag: u16,
    pub front_sidedef: u16,
    pub back_sidedef: u16,
}

impl LineDef {
    pub const SIZE: usize = 14;
    pub const NO_SIDEDEF: u16 = 0xFFFF;

    pub fn parse_all(bytes: &[u8]) -> Result<Vec<Self>, WadError> {
        if bytes.len() % Self::SIZE != 0 {
            return Err(WadError::TruncatedLump {
                name: "LINEDEFS".into(),
                expected: bytes.len().next_multiple_of(Self::SIZE),
            });
        }
        Ok(bytes
            .chunks_exact(Self::SIZE)
            .map(|c| Self {
                start_vertex: u16::from_le_bytes([c[0], c[1]]),
                end_vertex: u16::from_le_bytes([c[2], c[3]]),
                flags: u16::from_le_bytes([c[4], c[5]]),
                special_type: u16::from_le_bytes([c[6], c[7]]),
                sector_tag: u16::from_le_bytes([c[8], c[9]]),
                front_sidedef: u16::from_le_bytes([c[10], c[11]]),
                back_sidedef: u16::from_le_bytes([c[12], c[13]]),
            })
            .collect())
    }

    pub fn is_two_sided(&self) -> bool {
        self.back_sidedef != Self::NO_SIDEDEF
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideDef {
    pub x_offset: i16,
    pub y_offset: i16,
    pub upper_texture: String,
    pub lower_texture: String,
    pub middle_texture: String,
    pub sector: u16,
}

impl SideDef {
    pub const SIZE: usize = 30;

    pub fn parse_all(bytes: &[u8]) -> Result<Vec<Self>, WadError> {
        if bytes.len() % Self::SIZE != 0 {
            return Err(WadError::TruncatedLump {
                name: "SIDEDEFS".into(),
                expected: bytes.len().next_multiple_of(Self::SIZE),
            });
        }
        bytes
            .chunks_exact(Self::SIZE)
            .map(|c| {
                Ok(Self {
                    x_offset: i16::from_le_bytes([c[0], c[1]]),
                    y_offset: i16::from_le_bytes([c[2], c[3]]),
                    upper_texture: parse_lump_name(&c[4..12])?,
                    lower_texture: parse_lump_name(&c[12..20])?,
                    middle_texture: parse_lump_name(&c[20..28])?,
                    sector: u16::from_le_bytes([c[28], c[29]]),
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sector {
    pub floor_height: i16,
    pub ceiling_height: i16,
    pub floor_texture: String,
    pub ceiling_texture: String,
    pub light_level: i16,
    pub sector_type: u16,
    pub tag: u16,
}

impl Sector {
    pub const SIZE: usize = 26;

    pub fn parse_all(bytes: &[u8]) -> Result<Vec<Self>, WadError> {
        if bytes.len() % Self::SIZE != 0 {
            return Err(WadError::TruncatedLump {
                name: "SECTORS".into(),
                expected: bytes.len().next_multiple_of(Self::SIZE),
            });
        }
        bytes
            .chunks_exact(Self::SIZE)
            .map(|c| {
                Ok(Self {
                    floor_height: i16::from_le_bytes([c[0], c[1]]),
                    ceiling_height: i16::from_le_bytes([c[2], c[3]]),
                    floor_texture: parse_lump_name(&c[4..12])?,
                    ceiling_texture: parse_lump_name(&c[12..20])?,
                    light_level: i16::from_le_bytes([c[20], c[21]]),
                    sector_type: u16::from_le_bytes([c[22], c[23]]),
                    tag: u16::from_le_bytes([c[24], c[25]]),
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Thing {
    pub x: i16,
    pub y: i16,
    pub angle: i16,
    pub thing_type: u16,
    pub flags: u16,
}

impl Thing {
    pub const SIZE: usize = 10;

    pub fn parse_all(bytes: &[u8]) -> Result<Vec<Self>, WadError> {
        if bytes.len() % Self::SIZE != 0 {
            return Err(WadError::TruncatedLump {
                name: "THINGS".into(),
                expected: bytes.len().next_multiple_of(Self::SIZE),
            });
        }
        Ok(bytes
            .chunks_exact(Self::SIZE)
            .map(|c| Self {
                x: i16::from_le_bytes([c[0], c[1]]),
                y: i16::from_le_bytes([c[2], c[3]]),
                angle: i16::from_le_bytes([c[4], c[5]]),
                thing_type: u16::from_le_bytes([c[6], c[7]]),
                flags: u16::from_le_bytes([c[8], c[9]]),
            })
            .collect())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapName {
    /// DOOM I / Heretic: ExMy with episode 1..=4 (Heretic uses 1..=3) and mission 1..=9.
    Episode { episode: u8, mission: u8 },
    /// DOOM II: MAP01..MAP32.
    Map { number: u8 },
}

impl MapName {
    pub fn parse(s: &str) -> Option<Self> {
        let bytes = s.as_bytes();
        if bytes.len() == 4 && bytes[0] == b'E' && bytes[2] == b'M' {
            let episode = (bytes[1] as char).to_digit(10)? as u8;
            let mission = (bytes[3] as char).to_digit(10)? as u8;
            Some(Self::Episode { episode, mission })
        } else if bytes.len() == 5 && &bytes[0..3] == b"MAP" {
            let n: u8 = std::str::from_utf8(&bytes[3..5]).ok()?.parse().ok()?;
            Some(Self::Map { number: n })
        } else {
            None
        }
    }

    pub fn lump_name(&self) -> String {
        match self {
            Self::Episode { episode, mission } => format!("E{episode}M{mission}"),
            Self::Map { number } => format!("MAP{number:02}"),
        }
    }
}

/// Fully-parsed map data ready to feed the editor view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapData {
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub linedefs: Vec<LineDef>,
    pub sidedefs: Vec<SideDef>,
    pub sectors: Vec<Sector>,
    pub things: Vec<Thing>,
}
