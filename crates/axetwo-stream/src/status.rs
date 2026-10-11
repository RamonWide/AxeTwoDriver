//! A read-only look at the unit: what is attached and whether audio is set up.
//! Uses only hub queries, so it works while another program is streaming.

use crate::{StreamError, INTERFACE_GUID};
use axetwo_usb::layout::{self, StreamLayout};
use axetwo_win::{hub, winusb};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitState {
    NotConnected,
    /// Attached, but its USB firmware has not been loaded.
    NeedsFirmware,
    Running,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub unit: UnitState,
    /// Whether the audio function is on WinUSB, as `axetwo setup` leaves it.
    pub audio_set_up: bool,
    /// The unit's streams, when it is running and its layout is supported.
    pub layout: Option<StreamLayout>,
}

pub fn read() -> Result<Status, StreamError> {
    let attached = hub::attached_devices()?;
    let find = |pid| attached.iter().find(move |d| d.vendor_id() == layout::VENDOR_ID && d.product_id() == pid);
    let audio_set_up = !winusb::interface_paths(INTERFACE_GUID)?.is_empty();
    let Some(unit) = find(layout::PRODUCT_ID_RUNNING) else {
        let unit = if find(layout::PRODUCT_ID_BOOTLOADER).is_some() {
            UnitState::NeedsFirmware
        } else {
            UnitState::NotConnected
        };
        return Ok(Status { unit, audio_set_up, layout: None });
    };
    let layout = hub::configuration_descriptor(&unit.hub_path, unit.port, 0)
        .ok()
        .and_then(|raw| axetwo_usb::parse_configuration(&raw).ok())
        .and_then(|config| layout::stream_layout(&config).ok());
    Ok(Status { unit: UnitState::Running, audio_set_up, layout })
}
