mod app;
mod format;
mod linebuf;
mod serial;

use app::BaudApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Baud",
        options,
        Box::new(|_cc| Ok(Box::new(BaudApp::default()))),
    )
}
