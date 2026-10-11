// Hide the console window on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use eq_box::{icon, ui};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("EQ Box")
            .with_app_id("eq-box")
            .with_inner_size(ui::app::DESIGN)
            .with_min_inner_size(ui::app::DESIGN * 0.5)
            .with_icon(icon::icon()),
        ..Default::default()
    };
    eframe::run_native(
        "EQ Box",
        options,
        Box::new(|cc| Ok(Box::new(ui::app::EqApp::new(cc)))),
    )
}
