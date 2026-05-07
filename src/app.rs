// ABOUTME: Top-level egui app — composes title bar, left sidebar, and map viewport.
// ABOUTME: Owns the editor state (current WAD, current map, selection, view transform).

mod checks;
mod calculator;
mod commands;
mod config;
mod dialog;
mod export_picture;
mod hittest;
mod keybindings;
mod map_titles;
mod mem_probe;
mod menu;
mod sidebar;
mod picker_data;
mod state;
mod textures;
mod things_table;
mod view3d;
mod view3d_gl;
mod viewer;
mod viewport;

use eframe::egui;

use crate::theme;
pub use state::{EditorState, SelectionMode};
use textures::TextureBank;

/// Channel for receiving results from async operations (WASM file picker).
pub enum AsyncCommand {
    LoadWad { name: String, bytes: Vec<u8> },
}

#[cfg(target_arch = "wasm32")]
const DEMO_WAD: &[u8] = include_bytes!("../maps/infinity/INFINITY.WAD");

pub struct EdMapApp {
    state: EditorState,
    /// Texture bank rebuilt whenever the active WAD changes. Keyed by wad path.
    bank: TextureBank,
    bank_for_path: Option<std::path::PathBuf>,
    mem_probe: mem_probe::MemProbe,
    /// Lazily-initialized GL renderer for Phase 2 3D view. Behind Arc<Mutex<>>
    /// so it can be cloned into the egui_glow PaintCallback closure (which
    /// requires Send + Sync + 'static).
    view3d_gl: std::sync::Arc<std::sync::Mutex<view3d_gl::Renderer3D>>,
    /// Channel for receiving results from async operations (WASM file picker).
    tx: std::sync::mpsc::Sender<AsyncCommand>,
    rx: std::sync::mpsc::Receiver<AsyncCommand>,
}

impl EdMapApp {
    pub fn new() -> Self {
        let mut state = EditorState::default();
        state.config = config::EdMapConfig::load();
        let (tx, rx) = std::sync::mpsc::channel();

        #[cfg(target_arch = "wasm32")]
        {
            let _ = tx.send(AsyncCommand::LoadWad {
                name: "INFINITY.WAD (demo)".into(),
                bytes: DEMO_WAD.to_vec(),
            });
        }

        Self {
            state,
            bank: TextureBank::default(),
            bank_for_path: None,
            mem_probe: mem_probe::MemProbe::new(),
            view3d_gl: std::sync::Arc::new(std::sync::Mutex::new(view3d_gl::Renderer3D::new())),
            tx,
            rx,
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

    fn handle_async_commands(&mut self) {
        while let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                AsyncCommand::LoadWad { name, bytes } => {
                    // Merge a tiny built-in IWAD so PWADs that omit PLAYPAL /
                    // PNAMES / TEXTURE1 / F_/S_ markers still load. Primary
                    // (the user's WAD) wins on name collisions.
                    let merged = crate::wad::merge_wads(&bytes, &crate::minimal_iwad::bytes())
                        .unwrap_or(bytes);
                    if let Ok(wad) = crate::wad::Wad::from_bytes(merged) {
                        let maps = wad.map_names();
                        let is_demo = name.contains("(demo)");
                        self.state.wad_path = Some(std::path::PathBuf::from(name));
                        self.state.wad = Some(wad);
                        self.state.map = None;
                        self.state.selection.clear();
                        self.state.status_message = None;
                        self.state.is_dirty = false;
                        
                        if maps.len() == 1 {
                            commands::load_map_from_wad(&mut self.state, &maps[0]);
                        } else if maps.len() > 1 {
                            let mut map_to_load = None;
                            if is_demo {
                                if maps.contains(&"E1M1".to_string()) {
                                    map_to_load = Some("E1M1");
                                } else if maps.contains(&"MAP01".to_string()) {
                                    map_to_load = Some("MAP01");
                                }
                            }

                            if let Some(map_name) = map_to_load {
                                commands::load_map_from_wad(&mut self.state, map_name);
                            } else {
                                self.state.dialog = Some(crate::app::state::Dialog::OpenMapPicker {
                                    maps,
                                    selected: 0,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            if let Some(bytes) = file.bytes {
                // We treat a drop as a LoadWad command.
                let name = file.name.clone();
                let _ = self.tx.send(AsyncCommand::LoadWad { 
                    name, 
                    bytes: bytes.to_vec() 
                });
            }
        }
    }
}

impl eframe::App for EdMapApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_async_commands();
        self.handle_dropped_files(ctx);
        keybindings::dispatch(ctx, &mut self.state, &self.tx);
        self.refresh_bank_if_needed();
        self.mem_probe.refresh_if_due();
        title_bar(ctx);

        let mem_free_kb = self.mem_probe.free_kb();
        let mem_total_kb = self.mem_probe.total_kb();
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(160.0)
            .frame(egui::Frame::none().fill(theme::SIDEBAR_BG))
            .show(ctx, |ui| {
                sidebar::draw(ui, &mut self.state, &mut self.bank, mem_free_kb, mem_total_kb);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::VIEWPORT_BG))
            .show(ctx, |ui| {
                if self.state.view3d_open {
                    view3d::draw(ui, &mut self.state, &mut self.bank, self.view3d_gl.clone());
                } else {
                    viewport::draw(ui, &mut self.state, &mut self.bank);
                }
            });

        // Cascading menu bar lives across the top of the viewport row, sourced from
        // sidebar state so we can keep menu open across frames.
        menu::draw_open_menu(ctx, &mut self.state, &self.tx);
        // Texture viewer (F10) — drawn before modals so dialogs from it overlay correctly.
        viewer::draw(ctx, &mut self.state, &mut self.bank);
        // Modals draw last so they're above everything else.
        dialog::draw(ctx, &mut self.state);
        // Calculator window — non-modal, drawn on top.
        calculator::draw(ctx, &mut self.state);
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
                    ui.colored_label(theme::MENU_FG, "v2.0.0");
                });
            });
        });
}
