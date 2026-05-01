// ABOUTME: Texture viewer (F10) — paged grid of textures with category tabs.
// ABOUTME: Walls / Floors-Ceilings / Sprites tabs; click a tile to print a status message.

use eframe::egui::{self, Align2, Color32, Sense, Vec2};

use super::state::{EditorState, ViewerCategory};
use super::textures::TextureBank;
use crate::theme;

const TILE_SIZE: f32 = 96.0;
const TILE_LABEL_HEIGHT: f32 = 14.0;
const TILE_PADDING: f32 = 4.0;

pub fn draw(ctx: &egui::Context, state: &mut EditorState, bank: &mut TextureBank) {
    if !state.viewer_open {
        return;
    }
    let mut keep_open = true;
    let screen = ctx.screen_rect();

    egui::Area::new(egui::Id::new("texture_viewer_area"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(theme::SIDEBAR_BG)
                .stroke(egui::Stroke::new(1.0, theme::VGA_GRAY))
                .inner_margin(egui::Margin::same(0.0))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    let max_w = (screen.width() * 0.9).min(960.0).max(560.0);
                    let max_h = (screen.height() * 0.85).min(680.0).max(420.0);
                    ui.set_max_width(max_w);
                    ui.set_max_height(max_h);

                    title_strip(ui, &mut keep_open, state);
                    tabs(ui, state);

                    let names = current_names(state, bank);
                    if names.is_empty() {
                        ui.add_space(20.0);
                        ui.colored_label(theme::VGA_DARK_GRAY, empty_message(state.viewer_category));
                    } else {
                        grid(ui, state, bank, &names);
                    }
                });
        });

    if !keep_open {
        state.viewer_open = false;
    }
}

fn title_strip(ui: &mut egui::Ui, keep_open: &mut bool, state: &EditorState) {
    let desired = egui::vec2(ui.available_width(), 16.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::MENU_HILITE_BG);
    let title = match state.viewer_category {
        ViewerCategory::Walls => "Choose a Wall Texture",
        ViewerCategory::Flats => "Choose a Floor/Ceiling Texture",
        ViewerCategory::Sprites => "Sprites",
    };
    painter.text(
        egui::pos2(rect.left() + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        title,
        egui::FontId::new(12.0, egui::FontFamily::Monospace),
        theme::MENU_HILITE_FG,
    );
    // [X] close affordance on the right.
    let close_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - 22.0, rect.top() + 1.0),
        egui::vec2(20.0, 14.0),
    );
    painter.text(
        close_rect.center(),
        Align2::CENTER_CENTER,
        "[X]",
        egui::FontId::new(12.0, egui::FontFamily::Monospace),
        theme::MENU_HILITE_FG,
    );
    let close_resp = ui.interact(close_rect, egui::Id::new("viewer_close"), Sense::click());
    if close_resp.clicked() {
        *keep_open = false;
    }
    let _ = Color32::TRANSPARENT;
}

fn tabs(ui: &mut egui::Ui, state: &mut EditorState) {
    let desired = egui::vec2(ui.available_width(), 16.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, theme::MENU_BG);

    let labels = [
        (ViewerCategory::Walls, "Walls"),
        (ViewerCategory::Flats, "Floors/Ceilings"),
        (ViewerCategory::Sprites, "Sprites"),
    ];
    let tab_w = rect.width() / labels.len() as f32;
    for (i, (cat, label)) in labels.iter().enumerate() {
        let tab_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + i as f32 * tab_w, rect.top()),
            egui::vec2(tab_w, rect.height()),
        );
        let active = state.viewer_category == *cat;
        let bg = if active { theme::MENU_HILITE_BG } else { theme::MENU_BG };
        let fg = if active { theme::MENU_HILITE_FG } else { theme::MENU_FG };
        painter.rect_filled(tab_rect, 0.0, bg);
        painter.text(
            tab_rect.center(),
            Align2::CENTER_CENTER,
            *label,
            egui::FontId::new(12.0, egui::FontFamily::Monospace),
            fg,
        );
        // Subtle separator between tabs.
        if i + 1 < labels.len() {
            painter.vline(
                tab_rect.right(),
                tab_rect.top()..=tab_rect.bottom(),
                egui::Stroke::new(1.0, theme::MENU_EDGE_DARK),
            );
        }
        let resp = ui.interact(tab_rect, egui::Id::new(("viewer_tab", i)), Sense::click());
        if resp.clicked() {
            state.viewer_category = *cat;
        }
    }
}

fn current_names(state: &EditorState, bank: &TextureBank) -> Vec<String> {
    match state.viewer_category {
        ViewerCategory::Walls => bank.walls.iter().map(|d| d.name.clone()).collect(),
        ViewerCategory::Flats => bank.flat_names.clone(),
        ViewerCategory::Sprites => bank.sprite_names.clone(),
    }
}

fn empty_message(category: ViewerCategory) -> &'static str {
    match category {
        ViewerCategory::Walls => "No wall textures (TEXTURE1/2 missing).",
        ViewerCategory::Flats => "No flats (F_START..F_END missing).",
        ViewerCategory::Sprites => "No sprites (S_START..S_END missing).",
    }
}

fn grid(ui: &mut egui::Ui, state: &mut EditorState, bank: &mut TextureBank, names: &[String]) {
    let scroll_id = ("viewer_scroll", state.viewer_category as u8);
    egui::ScrollArea::vertical().id_source(scroll_id).show(ui, |ui| {
        let avail_w = ui.available_width().max(TILE_SIZE);
        let cell_w = TILE_SIZE + TILE_PADDING * 2.0;
        let cols = (avail_w / cell_w).floor().max(1.0) as usize;
        let cell_h = TILE_SIZE + TILE_LABEL_HEIGHT + TILE_PADDING * 2.0;

        let rows = names.len().div_ceil(cols);
        let total_size = egui::vec2(avail_w, rows as f32 * cell_h);
        let (rect, _) = ui.allocate_exact_size(total_size, Sense::hover());

        for (i, name) in names.iter().enumerate() {
            let row = i / cols;
            let col = i % cols;
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + col as f32 * cell_w,
                    rect.top() + row as f32 * cell_h,
                ),
                egui::vec2(cell_w, cell_h),
            );

            let resp = ui.interact(cell_rect, egui::Id::new(("viewer_cell", i)), Sense::click());
            let hovered = resp.hovered();

            // Tile background.
            ui.painter_at(cell_rect).rect_filled(
                cell_rect,
                0.0,
                if hovered { theme::MENU_HILITE_BG } else { theme::VIEWPORT_BG },
            );

            // Texture image (lazy-load via the bank).
            let img_rect = egui::Rect::from_min_size(
                egui::pos2(cell_rect.left() + TILE_PADDING, cell_rect.top() + TILE_PADDING),
                egui::vec2(TILE_SIZE, TILE_SIZE),
            );
            let handle_opt = if let Some(wad) = state.wad.as_ref() {
                match state.viewer_category {
                    ViewerCategory::Walls => bank.wall(ui.ctx(), wad, name),
                    ViewerCategory::Flats => bank.flat(ui.ctx(), wad, name),
                    ViewerCategory::Sprites => bank.sprite(ui.ctx(), wad, name),
                }
            } else {
                None
            };
            if let Some(handle) = handle_opt {
                let image = egui::Image::new(handle).fit_to_exact_size(img_rect.size());
                image.paint_at(ui, img_rect);
            } else {
                ui.painter_at(img_rect).rect_stroke(
                    img_rect,
                    0.0,
                    egui::Stroke::new(1.0, theme::MENU_EDGE_DARK),
                );
                ui.painter_at(img_rect).text(
                    img_rect.center(),
                    Align2::CENTER_CENTER,
                    "?",
                    egui::FontId::new(20.0, egui::FontFamily::Monospace),
                    theme::VGA_DARK_GRAY,
                );
            }

            // Label below.
            let label_rect = egui::Rect::from_min_size(
                egui::pos2(cell_rect.left(), img_rect.bottom() + 1.0),
                egui::vec2(cell_rect.width(), TILE_LABEL_HEIGHT),
            );
            let label_color = if hovered { theme::MENU_HILITE_FG } else { theme::VGA_WHITE };
            ui.painter_at(label_rect).text(
                label_rect.center(),
                Align2::CENTER_CENTER,
                name,
                egui::FontId::new(11.0, egui::FontFamily::Monospace),
                label_color,
            );

            if resp.clicked() {
                state.status_message = Some(format!("Selected texture: {name}"));
            }
        }
    });
}
