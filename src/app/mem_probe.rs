// ABOUTME: Real memory measurements for the sidebar's "free" line — system
// ABOUTME: free RAM (refreshed periodically) and in-memory map data size.

use std::time::{Duration, Instant};

use crate::wad::{LineDef, MapData, Sector, SideDef, Thing, Vertex};

/// Owns a sysinfo::System instance and re-refreshes its memory stats every
/// REFRESH_INTERVAL. Cheap to call every frame; only does work occasionally.
pub struct MemProbe {
    sys: sysinfo::System,
    last_refresh: Instant,
    last_free_kb: u64,
    last_total_kb: u64,
}

const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

impl MemProbe {
    pub fn new() -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        let last_free_kb = sys.available_memory() / 1024;
        let last_total_kb = sys.total_memory() / 1024;
        Self {
            sys,
            last_refresh: Instant::now(),
            last_free_kb,
            last_total_kb,
        }
    }

    pub fn refresh_if_due(&mut self) {
        if self.last_refresh.elapsed() >= REFRESH_INTERVAL {
            self.sys.refresh_memory();
            self.last_free_kb = self.sys.available_memory() / 1024;
            self.last_total_kb = self.sys.total_memory() / 1024;
            self.last_refresh = Instant::now();
        }
    }

    /// Free system memory in kilobytes (1024 bytes).
    pub fn free_kb(&self) -> u64 { self.last_free_kb }

    /// Total system memory in kilobytes.
    pub fn total_kb(&self) -> u64 { self.last_total_kb }
}

impl Default for MemProbe {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory map data size in bytes — sum of record sizes per record type.
pub fn map_data_bytes(map: &MapData) -> usize {
    map.vertices.len() * Vertex::SIZE
        + map.linedefs.len() * LineDef::SIZE
        + map.sidedefs.len() * SideDef::SIZE
        + map.sectors.len() * Sector::SIZE
        + map.things.len() * Thing::SIZE
}

/// Format kilobytes as a compact string. Uses k for <1024, M for ≥1024.
pub fn fmt_kb(kb: u64) -> String {
    if kb < 1024 {
        format!("{}k", kb)
    } else if kb < 1024 * 1024 {
        format!("{:.2}M", kb as f64 / 1024.0)
    } else {
        format!("{:.2}G", kb as f64 / (1024.0 * 1024.0))
    }
}
