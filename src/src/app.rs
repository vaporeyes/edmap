// ABOUTME: Top-level egui app — composes title bar, left sidebar, and map viewport.
// ABOUTME: Owns the editor state (current WAD, current map, selection, view transform).

mod checks;
mod commands;
mod dialog;
mod hittest;
mod keybindings;
mod menu;
mod sidebar;
mod state;
mod textures;
mod viewer;
mod viewport;

use eframe::egui;

use crate::theme;
pub use state::{EditorState, SelectionMode};
use textures::TextureBank;

pub struct EdMapApp {
    state: EditorState,
    /// Texture bank rebuilt whenever the active WAD changes. Keyed by wad path.
    bank: TextureBank,
    bank_for_path: Option<std::path::PathBuf>,
}

impl EdMapApp {
    pub fn new() -> Self {
        Self {
            state: EditorState::default(),
            bank: TextureBank::default(),
            bank_for_path: None,
        }
    }

    fn refresh_bank_if_needed(&mut self) {
        if self.state.wad_path == self.bank_for_path {
            return;
        }
        self.bank = match (&self.state.wad, &self.state.wad_path) {
            (Some(wad), _) => TextureBank::rebuild_from(wad),
            _ => TextureBank::default(),
        };
        self.bank_for_path = self.state.wad_path.clone();
    }
}

impl eframe::App for EdMapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        keybindings::dispatch(ctx, &mut self.state);
        self.refresh_bank_if_needed();
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
        // Texture viewer (F10) — drawn before modals so dialogs from it overlay correctly.
        viewer::draw(ctx, &mut self.state, &mut self.bank);
        // Modals draw last so they're above everything else.
        dialog::draw(ctx, &mut self.state);
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
