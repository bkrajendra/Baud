mod format;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Baud",
        options,
        Box::new(|_cc| Ok(Box::new(EmptyApp))),
    )
}

struct EmptyApp;

impl eframe::App for EmptyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.label("Baud starting up...");
    }
}
