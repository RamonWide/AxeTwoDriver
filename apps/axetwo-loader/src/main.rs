//! Loads the unit's USB firmware whenever it is switched on.
//!
//! Started by Windows at boot (the installer registers a scheduled task) and
//! does nothing but wait for a unit that needs its firmware. What it did is
//! appended to `C:\ProgramData\AxeTwoDriver\loader.log`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {}

#[cfg(windows)]
fn main() {
    use axetwo_stream::firmware::{self, LOADER_DEVICE_ID, LOADER_INTERFACE_GUID};
    use axetwo_stream::status::{self, UnitState};
    use axetwo_win::setup;
    use std::time::Duration;

    const POLL: Duration = Duration::from_secs(1);
    /// Time the unit needs to come back under its running identity.
    const SETTLE: Duration = Duration::from_secs(5);

    log("started");
    // After a failure, wait for the unit to be switched off before trying
    // again, so a unit that refuses the firmware is not sent it in a loop.
    let mut failed = false;
    // Whether this arrival of the unit was already handed to WinUSB.
    let mut claimed = false;
    loop {
        let attached_without_firmware = status::read().is_ok_and(|s| s.unit == UnitState::NeedsFirmware);
        if !attached_without_firmware {
            failed = false;
            claimed = false;
        } else if !firmware::unit_is_waiting() {
            // The unit is there but Windows has it on another driver, or on
            // none: a new USB port, or the first power-on after installing.
            // Windows only keeps a driver choice made while the device is
            // attached, so it is made here.
            if !claimed {
                claimed = true;
                match setup::bind_winusb(LOADER_DEVICE_ID, LOADER_INTERFACE_GUID) {
                    Ok(report) => log(&format!("took over {}", report.instance_id)),
                    Err(e) => log(&format!("could not take over the waiting unit: {e}")),
                }
            }
        } else if !failed {
            match firmware::load_if_waiting() {
                Ok(Some(done)) => {
                    log(&format!("firmware loaded: {} blocks, {} bytes, all verified", done.blocks, done.bytes));
                    std::thread::sleep(SETTLE);
                }
                Ok(None) => {}
                Err(e) => {
                    log(&format!("not loaded: {e}"));
                    failed = true;
                }
            }
        }
        std::thread::sleep(POLL);
    }
}

#[cfg(windows)]
fn log(text: &str) {
    use std::io::Write;
    let dir =
        std::path::PathBuf::from(std::env::var("ProgramData").unwrap_or_else(|_| String::from("C:\\ProgramData")))
            .join("AxeTwoDriver");
    let _ = std::fs::create_dir_all(&dir);
    let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("loader.log")) else {
        return;
    };
    let seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let _ = writeln!(file, "{seconds} {text}");
}
