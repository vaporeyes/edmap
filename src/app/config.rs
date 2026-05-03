// ABOUTME: Persistent user preferences (test-map exe/args template) stored as JSON.
// ABOUTME: Loaded at app start, saved when the Test Map Settings dialog closes.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestMapConfig {
    /// Path to the source port executable (gzdoom, dsda-doom, prboom-plus, etc.).
    pub exe: String,
    /// Whitespace-split argument template. Placeholders substituted at launch:
    ///   %F = path to the temp PWAD just written
    ///   %L = full map name (MAP01, E1M3, etc.)
    ///   %E = episode digit parsed from name (1 if unknown)
    ///   %M = map digit parsed from name (1 if unknown)
    pub args: String,
}

impl Default for TestMapConfig {
    fn default() -> Self {
        Self {
            exe: String::new(),
            args: "-file %F -warp %E %M".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View3DConfig {
    /// Reverse vertical mouse-look (drag down = look up).
    pub invert_mouse_y: bool,
    /// Reverse horizontal mouse-look (drag right = turn left).
    pub invert_mouse_x: bool,
    /// Mouse-look multiplier. 1.0 = default; lower = slower, higher = snappier.
    pub mouse_sensitivity: f32,
    /// Movement speed multiplier applied to base 320 units/sec.
    pub move_speed: f32,
    /// Multiplier applied while Shift is held.
    pub sprint_multiplier: f32,
    /// Vertical field-of-view in degrees.
    pub fov_degrees: f32,
}

impl Default for View3DConfig {
    fn default() -> Self {
        Self {
            invert_mouse_y: false,
            invert_mouse_x: false,
            mouse_sensitivity: 1.0,
            move_speed: 1.0,
            sprint_multiplier: 2.5,
            fov_degrees: 75.0,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EdMapConfig {
    #[serde(default)]
    pub test_map: TestMapConfig,
    #[serde(default)]
    pub view3d: View3DConfig,
}

/// Returns the on-disk config path, or None if HOME is not set.
fn config_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config").join("edmap").join("config.json"))
}

impl EdMapConfig {
    pub fn load() -> Self {
        let Some(path) = config_path() else { return Self::default() };
        let Ok(bytes) = std::fs::read(&path) else { return Self::default() };
        serde_json::from_slice(&bytes).unwrap_or_default()
    }

    /// Best-effort save. Errors are reported via the returned Result so the
    /// caller can surface them in the status bar.
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = config_path() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "$HOME not set",
            ));
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&path, json)
    }
}

/// Parse a map name like "MAP01" or "E1M3" into (episode, map). Falls back to
/// (1, 1) for anything we can't recognize.
pub fn parse_map_warp(name: &str) -> (u32, u32) {
    let upper = name.trim().to_ascii_uppercase();
    if let Some(rest) = upper.strip_prefix("MAP") {
        if let Ok(m) = rest.parse::<u32>() {
            return (1, m);
        }
    }
    if let Some(rest) = upper.strip_prefix('E') {
        if let Some((e, m)) = rest.split_once('M') {
            if let (Ok(e), Ok(m)) = (e.parse::<u32>(), m.parse::<u32>()) {
                return (e, m);
            }
        }
    }
    (1, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_map_naming() {
        assert_eq!(parse_map_warp("MAP01"), (1, 1));
        assert_eq!(parse_map_warp("MAP27"), (1, 27));
        assert_eq!(parse_map_warp("E1M3"), (1, 3));
        assert_eq!(parse_map_warp("E4M9"), (4, 9));
        assert_eq!(parse_map_warp("garbage"), (1, 1));
    }
}
