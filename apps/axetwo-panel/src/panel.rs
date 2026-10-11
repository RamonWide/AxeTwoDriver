//! The window. It reads the unit's state through hub queries only, so it can
//! stay open while an audio program is streaming.

use axetwo_stream::settings::{Settings, BUFFER_SIZES, STREAMING_MODES};
use axetwo_stream::status::{self, Status, UnitState};
use axetwo_stream::Duplex;
use std::time::{Duration, Instant};

const REFRESH: Duration = Duration::from_secs(2);
const SAMPLE_RATE: u32 = 48_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Status,
    Buffers,
    Info,
}

/// What a one-off attempt to open the unit said about its clock.
enum Rate {
    Unknown,
    Hz(u32),
    /// The unit could not be opened, most likely because a program is using it.
    Busy,
}

pub struct Panel {
    page: Page,
    status: Option<Result<Status, String>>,
    checked: Instant,
    rate: Rate,
    settings: Settings,
    save_error: Option<String>,
}

impl Panel {
    pub fn new() -> Self {
        let mut panel = Self {
            page: Page::Status,
            status: None,
            checked: Instant::now(),
            rate: Rate::Unknown,
            settings: Settings::load(),
            save_error: None,
        };
        panel.refresh();
        panel.read_rate();
        panel
    }

    fn refresh(&mut self) {
        self.status = Some(status::read().map_err(|e| e.to_string()));
        self.checked = Instant::now();
    }

    /// Opens the unit for a moment to ask its clock. Done on request only: an
    /// audio program starting at the same instant would find the unit taken.
    fn read_rate(&mut self) {
        self.rate = match Duplex::open().and_then(|d| d.sample_rate()) {
            Ok(hz) => Rate::Hz(hz),
            Err(_) => Rate::Busy,
        };
    }

    fn save(&mut self) {
        self.save_error = self.settings.save().err().map(|e| e.to_string());
    }

    fn status_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("USB audio device");
        let status = match &self.status {
            Some(Ok(status)) => status.clone(),
            Some(Err(e)) => {
                ui.colored_label(ui.visuals().error_fg_color, e.as_str());
                return;
            }
            None => return,
        };
        let unit = match status.unit {
            UnitState::Running => "Axe-Fx II connected",
            UnitState::NeedsFirmware => "Axe-Fx II connected, waiting for its USB firmware",
            UnitState::NotConnected => "No device connected",
        };
        ui.label(unit);
        if status.unit == UnitState::Running && !status.audio_set_up {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "Audio is not set up for this unit. Run the AxeTwoDriver installer again with the unit switched on.",
            );
        }
        ui.add_space(12.0);

        ui.heading("Current sample rate");
        ui.horizontal(|ui| {
            match self.rate {
                Rate::Hz(hz) => ui.label(format!("{hz} Hz")),
                Rate::Busy if status.unit == UnitState::Running && status.audio_set_up => {
                    ui.label(format!("{SAMPLE_RATE} Hz (in use by an audio program)"))
                }
                _ => ui.label("-"),
            };
            if ui.button("Check").clicked() {
                self.read_rate();
            }
        });
        ui.add_space(12.0);

        ui.heading("Format");
        match status.layout {
            Some(layout) => {
                ui.label(format!("Input: {} channels, {} bits", layout.capture.channels, layout.capture.bits));
                ui.label(format!("Output: {} channels, {} bits", layout.playback.channels, layout.playback.bits));
            }
            None => {
                ui.label("-");
            }
        }
    }

    fn buffers_page(&mut self, ui: &mut egui::Ui) {
        let before = self.settings;
        ui.heading("USB streaming mode");
        let mode_name = |ms: u32| {
            STREAMING_MODES
                .iter()
                .find(|m| m.0 == ms)
                .map_or_else(|| format!("{ms} ms"), |m| format!("{} ({} ms)", m.1, m.0))
        };
        egui::ComboBox::from_id_salt("mode").width(240.0).selected_text(mode_name(self.settings.streaming_ms)).show_ui(
            ui,
            |ui| {
                for (ms, _) in STREAMING_MODES {
                    ui.selectable_value(&mut self.settings.streaming_ms, ms, mode_name(ms));
                }
            },
        );
        ui.label("How much playback is kept queued for the unit. More is safer against dropouts and adds latency.");
        ui.add_space(12.0);

        ui.heading("Buffer size");
        egui::ComboBox::from_id_salt("buffer")
            .width(240.0)
            .selected_text(format!("{} samples", self.settings.buffer_frames))
            .show_ui(ui, |ui| {
                for frames in BUFFER_SIZES {
                    ui.selectable_value(&mut self.settings.buffer_frames, frames, format!("{frames} samples"));
                }
            });
        ui.label("The size AxeTwoDriver suggests to audio programs. Many programs let you choose it there as well.");
        if self.settings.buffer_too_small(SAMPLE_RATE) {
            ui.add_space(6.0);
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "The buffer size is small for this streaming mode. If you hear crackles, choose a larger buffer or a lower-latency mode.",
            );
        }
        if self.settings != before {
            self.save();
        }
        if let Some(e) = &self.save_error {
            ui.colored_label(ui.visuals().error_fg_color, format!("Could not save: {e}"));
        }
        ui.add_space(12.0);
        ui.weak("A running audio program is asked to restart its audio engine when these change.");
    }

    fn info_page(ui: &mut egui::Ui) {
        ui.heading("AxeTwoDriver");
        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
        ui.label("An open-source USB driver for the Axe-Fx II, MIT licensed.");
        ui.hyperlink("https://github.com/RamonWide/AxeTwoDriver");
        ui.add_space(12.0);
        ui.weak(
            "Independent project, not affiliated with or endorsed by Fractal Audio Systems. \
             Axe-Fx is a trademark of Fractal Audio Systems, LLC.",
        );
    }
}

impl eframe::App for Panel {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.checked.elapsed() >= REFRESH {
            self.refresh();
        }
        ui.ctx().request_repaint_after(REFRESH);
        egui::Frame::new().inner_margin(14.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                for (page, name) in [(Page::Status, "Status"), (Page::Buffers, "Buffer Settings"), (Page::Info, "Info")]
                {
                    ui.selectable_value(&mut self.page, page, name);
                }
            });
            ui.separator();
            ui.add_space(6.0);
            match self.page {
                Page::Status => self.status_page(ui),
                Page::Buffers => self.buffers_page(ui),
                Page::Info => Self::info_page(ui),
            }
        });
    }
}
