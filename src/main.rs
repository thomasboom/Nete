mod app;
mod config;
mod db;
mod error;
mod models;
mod plugins;
mod services;
mod ui;

use app::NeteApp;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 860.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Nete",
        native_options,
        Box::new(|cc| {
            ui::theme::install_dark_theme(&cc.egui_ctx);
            Box::new(NeteApp::boot())
        }),
    )
}
