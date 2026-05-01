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
// Viewport background — pure black with grid dots painted on top.
pub const VIEWPORT_BG: Color32 = VGA_BLACK;
// Grid dot color.
pub const GRID_DOT: Color32 = Color32::from_rgb(0x00, 0x00, 0x55);
// LineDef stroke — bright on dark.
pub const LINEDEF_NORMAL: Color32 = VGA_WHITE;
pub const LINEDEF_TWO_SIDED: Color32 = VGA_GRAY;
pub const LINEDEF_SELECTED: Color32 = VGA_BRIGHT_RED;
pub const VERTEX_DOT: Color32 = VGA_BRIGHT_GREEN;
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
    let mut style = Style::default();

    let body = FontId::new(13.0, FontFamily::Monospace);
    let small = FontId::new(11.0, FontFamily::Monospace);
    let heading = FontId::new(14.0, FontFamily::Monospace);

    style.text_styles.insert(TextStyle::Body, body.clone());
    style.text_styles.insert(TextStyle::Small, small);
    style.text_styles.insert(TextStyle::Heading, heading);
    style.text_styles.insert(TextStyle::Button, body.clone());
    style.text_styles.insert(TextStyle::Monospace, body);

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
