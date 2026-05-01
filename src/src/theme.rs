// ABOUTME: VGA color palette + egui Style installation matching the original EdMap aesthetic.
// ABOUTME: All colors and spacing are tuned to the DOSBox screenshot (1px borders, hard pixels).

use egui::{Color32, FontFamily, FontId, Rounding, Stroke, Style, TextStyle, Visuals};

// 16-color VGA palette, indices match standard EGA/VGA color order.
pub const VGA_BLACK: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
pub const VGA_BLUE: Color32 = Color32::from_rgb(0x00, 0x00, 0xAA);
pub const VGA_GREEN: Color32 = Color32::from_rgb(0x00, 0xAA, 0x00);
pub const VGA_CYAN: Color32 = Color32::from_rgb(0x00, 0xAA, 0xAA);
pub const VGA_RED: Color32 = Color32::from_rgb(0xAA, 0x00, 0x00);
pub const VGA_GRAY: Color32 = Color32::from_rgb(0xAA, 0xAA, 0xAA);
pub const VGA_DARK_GRAY: Color32 = Color32::from_rgb(0x55, 0x55, 0x55);
pub const VGA_BRIGHT_BLUE: Color32 = Color32::from_rgb(0x55, 0x55, 0xFF);
pub const VGA_BRIGHT_GREEN: Color32 = Color32::from_rgb(0x55, 0xFF, 0x55);
pub const VGA_BRIGHT_CYAN: Color32 = Color32::from_rgb(0x55, 0xFF, 0xFF);
pub const VGA_BRIGHT_RED: Color32 = Color32::from_rgb(0xFF, 0x55, 0x55);
pub const VGA_YELLOW: Color32 = Color32::from_rgb(0xFF, 0xFF, 0x55);
pub const VGA_WHITE: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);

// Sidebar background — dark blue panel from the screenshot.
pub const SIDEBAR_BG: Color32 = Color32::from_rgb(0x14, 0x28, 0x8e);
// Menu rows + cascade panels — Turbo-Vision-style gray buttons.
pub const MENU_BG: Color32 = VGA_GRAY;
pub const MENU_FG: Color32 = VGA_BLACK;
// Inverse highlight (active top-level, hovered cascade row).
pub const MENU_HILITE_BG: Color32 = VGA_BLACK;
pub const MENU_HILITE_FG: Color32 = VGA_WHITE;
// 1-pixel "depressed" edges between menu rows.
pub const MENU_EDGE_DARK: Color32 = VGA_DARK_GRAY;
pub const MENU_EDGE_LIGHT: Color32 = VGA_WHITE;
// Map info box uses the same blue as the rest of the sidebar so the whole
// info column reads as one continuous panel (matches original EdMap).
pub const INFO_BOX_BG: Color32 = SIDEBAR_BG;
// Viewport background — deep blue, matches original EdMap map view.
pub const VIEWPORT_BG: Color32 = Color32::from_rgb(0x00, 0x00, 0x28);
// Grid dot color.
pub const GRID_DOT: Color32 = Color32::from_rgb(0x00, 0x00, 0x55);
// LineDef stroke — bright on dark.
pub const LINEDEF_NORMAL: Color32 = VGA_WHITE;
pub const LINEDEF_TWO_SIDED: Color32 = VGA_GRAY;
pub const LINEDEF_SELECTED: Color32 = VGA_BRIGHT_RED;
pub const VERTEX_DOT: Color32 = VGA_BRIGHT_GREEN;
pub const VERTEX_HOVER: Color32 = VGA_YELLOW;
pub const THING_MARK: Color32 = VGA_BRIGHT_CYAN;

/// Draw a 1-pixel Turbo-Vision-style bevel onto a button rectangle.
/// `pressed = false`: bright top+left, dark bottom+right (raised look).
/// `pressed = true` : dark top+left, bright bottom+right (depressed look).
pub fn draw_bevel(painter: &egui::Painter, rect: egui::Rect, pressed: bool) {
    let (light, dark) = if pressed {
        (MENU_EDGE_DARK, MENU_EDGE_LIGHT)
    } else {
        (MENU_EDGE_LIGHT, MENU_EDGE_DARK)
    };
    // Top
    painter.hline(
        rect.left()..=rect.right(),
        rect.top(),
        egui::Stroke::new(1.0, light),
    );
    // Left
    painter.vline(
        rect.left(),
        rect.top()..=rect.bottom(),
        egui::Stroke::new(1.0, light),
    );
    // Bottom
    painter.hline(
        rect.left()..=rect.right(),
        rect.bottom() - 1.0,
        egui::Stroke::new(1.0, dark),
    );
    // Right
    painter.vline(
        rect.right() - 1.0,
        rect.top()..=rect.bottom(),
        egui::Stroke::new(1.0, dark),
    );
}

pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);

    let mut style = Style::default();

    // Proportional family is the BGI sans font when loaded; numeric-aligned
    // surfaces (status block coords, LD# table) keep Monospace.
    let body = FontId::new(14.0, FontFamily::Proportional);
    let small = FontId::new(12.0, FontFamily::Proportional);
    let heading = FontId::new(15.0, FontFamily::Proportional);
    let mono = FontId::new(14.0, FontFamily::Monospace);

    style.text_styles.insert(TextStyle::Body, body.clone());
    style.text_styles.insert(TextStyle::Small, small);
    style.text_styles.insert(TextStyle::Heading, heading);
    style.text_styles.insert(TextStyle::Button, body);
    style.text_styles.insert(TextStyle::Monospace, mono);

    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(VGA_GRAY);
    visuals.window_fill = SIDEBAR_BG;
    visuals.panel_fill = SIDEBAR_BG;
    visuals.extreme_bg_color = VGA_BLACK;
    visuals.faint_bg_color = SIDEBAR_BG;

    // Hard 1-pixel borders, no rounding anywhere — VGA aesthetic.
    let zero_round = Rounding::ZERO;
    visuals.widgets.noninteractive.bg_fill = SIDEBAR_BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, VGA_GRAY);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, VGA_GRAY);
    visuals.widgets.noninteractive.rounding = zero_round;

    visuals.widgets.inactive.bg_fill = SIDEBAR_BG;
    visuals.widgets.inactive.weak_bg_fill = SIDEBAR_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, VGA_GRAY);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, VGA_WHITE);
    visuals.widgets.inactive.rounding = zero_round;

    visuals.widgets.hovered.bg_fill = VGA_GRAY;
    visuals.widgets.hovered.weak_bg_fill = VGA_GRAY;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, VGA_WHITE);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, VGA_BLACK);
    visuals.widgets.hovered.rounding = zero_round;

    visuals.widgets.active.bg_fill = VGA_GRAY;
    visuals.widgets.active.weak_bg_fill = VGA_GRAY;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, VGA_WHITE);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, VGA_BLACK);
    visuals.widgets.active.rounding = zero_round;

    visuals.widgets.open.bg_fill = VGA_GRAY;
    visuals.widgets.open.weak_bg_fill = VGA_GRAY;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, VGA_WHITE);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, VGA_BLACK);
    visuals.widgets.open.rounding = zero_round;

    visuals.menu_rounding = zero_round;
    visuals.window_rounding = zero_round;
    visuals.window_stroke = Stroke::new(1.0, VGA_GRAY);
    visuals.popup_shadow = egui::epaint::Shadow::NONE;
    visuals.window_shadow = egui::epaint::Shadow::NONE;

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(2.0, 1.0);
    style.spacing.button_padding = egui::vec2(4.0, 1.0);
    style.spacing.menu_margin = egui::Margin::same(2.0);
    style.spacing.window_margin = egui::Margin::same(0.0);

    ctx.set_style(style);
}

/// Try to install custom fonts from `assets/`. Loads bgi-sans.ttf if present
/// and inserts it at the head of the Proportional family. Logs a warning to
/// stderr if not found and falls back to egui's defaults.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut installed: Vec<String> = Vec::new();

    // Primary proportional font: PxPlus IBM VGA 9x16 — VileR's pixel-perfect
    // recreation of the IBM VGA 9-pixel ROM bitmap, period-correct for an
    // editor that targets DOS VGA aesthetics.
    let primary_candidates = [
        "PxPlus_IBM_VGA_9x16.ttf",
        "PxPlus_IBM_VGA_8x16.ttf",
        "Px437_IBM_VGA_9x16.ttf",
        "roboto.ttf",
        "Roboto-Regular.ttf",
    ];
    let primary = primary_candidates
        .iter()
        .find_map(|n| read_asset(n).map(|b| (*n, b)));
    if let Some((name, bytes)) = primary {
        let key = "ui-primary".to_string();
        fonts.font_data.insert(
            key.clone(),
            egui::FontData::from_owned(bytes).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, key.clone());
        // Pixel fonts also serve as Monospace — the VGA 9x16 face is
        // monospaced by design, so coordinate columns line up correctly.
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, key);
        installed.push(name.to_string());
    } else {
        eprintln!(
            "theme: no proportional font found in assets/. Drop PxPlus_IBM_VGA_9x16.ttf or \
             roboto.ttf at src/assets/ to override the default."
        );
    }

    if !installed.is_empty() {
        eprintln!("theme: loaded custom fonts: {}", installed.join(", "));
    }
    ctx.set_fonts(fonts);
}

/// Look for an asset file at runtime in a few candidate directories so the
/// app works whether you run it via `cargo run` or from the built binary.
fn read_asset(name: &str) -> Option<Vec<u8>> {
    use std::path::PathBuf;
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let candidates = [
        PathBuf::from(manifest_dir).join("assets").join(name),
        PathBuf::from("assets").join(name),
        PathBuf::from("../assets").join(name),
        PathBuf::from("./src/assets").join(name),
    ];
    for path in &candidates {
        if let Ok(bytes) = std::fs::read(path) {
            return Some(bytes);
        }
    }
    None
}
