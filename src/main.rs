#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod catrinth_api;
mod config;
mod icons;
mod java_manager;
mod launcher;
mod mod_loaders;
mod mojang_api;
mod theme;

use app::CatrinthApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Catrinth App — Minecraft")
            .with_inner_size([1080.0, 700.0])
            .with_min_inner_size([880.0, 560.0])
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "Catrinth App",
        native_options,
        Box::new(|cc| {
            theme::apply_catrinth_theme(&cc.egui_ctx);
            Ok(Box::new(CatrinthApp::new(cc)))
        }),
    )
}
