// ABOUTME: Top-level egui app — composes title bar, left sidebar, and map viewport.
// ABOUTME: Owns the editor state (current WAD, current map, selection, view transform).

mod commands;
mod keybindings;
mod menu;
mod sidebar;
mod state;
mod viewport;

use eframe::egui;

use crate::theme;
pub use state::EditorState;

pub struct EdMapApp {
    state: EditorState,
}

impl EdMapApp {
    pub fn new() -> Self {
        Self { state: EditorState::default() }
    }
}

impl eframe::App for EdMapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        keybindings::dispatch(ctx, &mut self.state);
        title_bar(ctx);

        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(160.0)
            .frame(egui::Frame::none().fill(theme::SIDEBAR_BG))
            .show(ctx, |ui| {
                sidebar::draw(ui, &mut self.state);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::VIEWPORT_BG))
            .show(ctx, |ui| {
                viewport::draw(ui, &mut self.state);
            });

        // Cascading menu bar lives across the top of the viewport row, sourced from
        // sidebar state so we can keep menu open across frames.
        menu::draw_open_menu(ctx, &mut self.state);
    }
}

fn title_bar(ctx: &egui::Context) {
    egui::TopBottomPanel::top("title")
        .exact_height(16.0)
        .frame(
            egui::Frame::none()
                .fill(theme::MENU_BG)
                .inner_margin(egui::Margin::symmetric(4.0, 1.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.colored_label(theme::MENU_FG, "EdMap");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(theme::MENU_FG, "v1.40");
                });
            });
        });
}
