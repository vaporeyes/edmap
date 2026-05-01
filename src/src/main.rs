// ABOUTME: Entry point for the egui-based EdMap rebuild.
// ABOUTME: Sets up eframe with VGA-styled visuals and launches the app.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod theme;
mod wad;

use app::EdMapApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("EdMap v1.40")
            .with_inner_size([960.0, 720.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "EdMap",
        native_options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(EdMapApp::new()))
        }),
    )
}
