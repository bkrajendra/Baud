#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod format;
mod linebuf;
mod serial;

use app::BaudApp;

fn load_icon() -> egui::IconData {
    let bytes = include_bytes!("../docs/app-icon.png");
    let image = image::load_from_memory(bytes)
        .expect("failed to decode docs/app-icon.png")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 500.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Baud",
        options,
        Box::new(|_cc| Ok(Box::new(BaudApp::default()))),
    )
}
