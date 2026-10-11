//! SP-10 Spectrum Analyser: the spectrum analyser on its own (see
//! `ui/analyser_app.rs`).

// Hide the console window on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use eq_box::{icon, ui};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("SP-10 Spectrum Analyser")
            .with_app_id("sp-10")
            .with_inner_size(ui::analyser_app::DEFAULT_SIZE)
            .with_min_inner_size(egui::Vec2::new(320.0, 200.0))
            .with_icon(icon::icon()),
        ..Default::default()
    };
    eframe::run_native(
        "SP-10 Spectrum Analyser",
        options,
        Box::new(|cc| Ok(Box::new(ui::analyser_app::AnalyserApp::new(cc)))),
    )
}
