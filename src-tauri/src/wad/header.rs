// ABOUTME: WAD file header and directory entry types per DOOM WAD format.
// ABOUTME: 12-byte header (magic + numlumps + infotableofs); 16-byte directory entries.

use serde::{Deserialize, Serialize};

use super::WadError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WadKind {
    Iwad,
    Pwad,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WadHeader {
    pub kind: WadKind,
    pub num_lumps: u32,
    pub directory_offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LumpEntry {
    pub offset: u32,
    pub size: u32,
    pub name: String,
}

impl WadHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self, WadError> {
        if bytes.len() < 12 {
            return Err(WadError::BadMagic([0; 4]));
        }
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        let kind = match &magic {
            b"IWAD" => WadKind::Iwad,
            b"PWAD" => WadKind::Pwad,
            _ => return Err(WadError::BadMagic(magic)),
        };
        let num_lumps = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let directory_offset = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        Ok(Self { kind, num_lumps, directory_offset })
    }
}

impl LumpEntry {
    pub const SIZE: usize = 16;

    pub fn parse(bytes: &[u8]) -> Result<Self, WadError> {
        if bytes.len() < Self::SIZE {
            return Err(WadError::BadLumpName);
        }
        let offset = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let size = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let name = parse_lump_name(&bytes[8..16])?;
        Ok(Self { offset, size, name })
    }
}

pub(crate) fn parse_lump_name(bytes: &[u8]) -> Result<String, WadError> {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    let slice = &bytes[..end];
    if !slice.iter().all(|&b| b.is_ascii()) {
        return Err(WadError::BadLumpName);
    }
    Ok(std::str::from_utf8(slice).map_err(|_| WadError::BadLumpName)?.to_string())
}
