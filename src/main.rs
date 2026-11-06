#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod catrinth_api;
mod config;
mod java_manager;
mod launcher;
mod mojang_api;
mod theme;

use app::CatrinthApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Catrinth Launcher — Minecraft")
            .with_inner_size([1040.0, 680.0])
            .with_min_inner_size([840.0, 540.0])
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "Catrinth Launcher",
        native_options,
        Box::new(|cc| {
            theme::apply_catrinth_theme(&cc.egui_ctx);
            Ok(Box::new(CatrinthApp::new(cc)))
        }),
    )
}
