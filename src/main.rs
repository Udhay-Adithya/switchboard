mod app;
mod demo;
mod model;
mod ops;
mod scan;
mod theme;

use app::AccountManagerApp;

fn main() -> eframe::Result<()> {
    let use_demo = std::env::args().any(|a| a == "--demo");
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([900.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "switchboard",
        options,
        Box::new(move |cc| Ok(Box::new(AccountManagerApp::new(cc, use_demo)))),
    )
}
