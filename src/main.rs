mod app;
mod ptz;
mod stream;

use app::EagleCastApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "EagleCast",
        options,
        Box::new(|cc| Ok(Box::new(EagleCastApp::new(cc)))),
    )
}
