// ABOUTME: Entry point for the egui-based EdMap rebuild.
// ABOUTME: Sets up eframe with VGA-styled visuals and launches the app.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod theme;
mod wad;

use app::EdMapApp;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("EdMap v2.0.0")
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

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();

        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    theme::install(&cc.egui_ctx);
                    Ok(Box::new(EdMapApp::new()))
                }),
            )
            .await
            .expect("failed to start eframe");
    });
}
