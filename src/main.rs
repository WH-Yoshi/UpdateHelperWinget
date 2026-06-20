#![windows_subsystem = "windows"]

mod config;
mod gui;
mod winget_manager;

use egui::ViewportBuilder;
use gui::PackageApp;
use config::{WINDOW_WIDTH, WINDOW_HEIGHT, MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT};

fn main() -> Result<(), eframe::Error> {
    let viewport = ViewportBuilder::default()
        .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
        .with_min_inner_size([MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT])
        .with_maximize_button(true)
        .with_minimize_button(true)
        .with_close_button(true)
        .with_decorations(true)
        .with_resizable(true)
        .with_transparent(false)
        .with_title("Gestionnaire de Mises à Jour (winget)");

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Gestionnaire de mises à jour",
        native_options,
        Box::new(|cc| Ok(Box::new(PackageApp::new(cc))))
    )
}