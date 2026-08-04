#![windows_subsystem = "windows"]

mod config;
mod gui;
mod winget_manager;

use config::{MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH, WINDOW_HEIGHT, WINDOW_WIDTH};
use egui::ViewportBuilder;
use gui::PackageApp;

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
        .with_title("WinGet Interfacer");

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "WinGet Interfacer",
        native_options,
        Box::new(|cc| Ok(Box::new(PackageApp::new(cc)))),
    )
}
