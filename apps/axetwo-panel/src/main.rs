//! The AxeTwoDriver control panel: what is connected, and the buffer settings.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod panel;

#[cfg(windows)]
fn main() -> eframe::Result {
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 400.0])
            .with_min_inner_size([420.0, 360.0])
            .with_title("AxeTwoDriver Control Panel"),
        ..Default::default()
    };
    eframe::run_native("AxeTwoDriver Control Panel", native, Box::new(|_cc| Ok(Box::new(panel::Panel::new()))))
}

#[cfg(not(windows))]
fn main() {
    eprintln!("The AxeTwoDriver control panel runs on Windows only.");
}
