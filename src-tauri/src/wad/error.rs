// ABOUTME: Error types for WAD parsing. Surface-level errors map to user-facing messages.
// ABOUTME: Keeps thiserror as the single source of truth for error rendering.

use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WadError {
    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("invalid WAD: bad magic {0:?}")]
    BadMagic([u8; 4]),

    #[error("invalid WAD: directory out of bounds (offset {offset}, size {size}, file len {file_len})")]
    BadDirectory { offset: u64, size: u64, file_len: u64 },

    #[error("invalid WAD: lump '{name}' truncated (expected {expected} bytes)")]
    TruncatedLump { name: String, expected: usize },

    #[error("map '{0}' not found in WAD")]
    MapNotFound(String),

    #[error("map '{0}' is incomplete (missing required lump '{1}')")]
    IncompleteMap(String, &'static str),

    #[error("invalid lump name (non-ASCII bytes)")]
    BadLumpName,
}
