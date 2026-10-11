//! Loading the unit's USB firmware at power-on.
//!
//! After power-on the unit's USB controller waits for its firmware. This
//! module sends it, from the script the original driver package installed on
//! this computer; AxeTwoDriver ships no firmware. Every block is read back
//! and compared before the controller is allowed to run, and only RAM is
//! written: switching the unit off undoes everything.

use axetwo_usb::loader::{self, ScriptError, Step, CPU_CONTROL, REQUEST_FIRMWARE_LOAD};
use axetwo_win::winusb::{self, Device};
use axetwo_win::WinError;
use std::fmt;
use std::path::PathBuf;

/// Device interface class under which WinUSB exposes a unit that is waiting
/// for its firmware, once the installer has set that up.
pub const LOADER_INTERFACE_GUID: &str = "{3D0C5A6E-71B4-4F0B-8E53-5B9C2A6D1F47}";
/// Device instance-id prefix of a unit waiting for its firmware.
pub const LOADER_DEVICE_ID: &str = "USB\\VID_2466&PID_0003\\";

/// Vendor request to the device, host to device.
const VENDOR_OUT: u8 = 0x40;
/// Vendor request to the device, device to host.
const VENDOR_IN: u8 = 0xC0;

#[derive(Debug)]
pub enum FirmwareError {
    /// The original package's script is not on this computer.
    NoScript(PathBuf),
    Script(ScriptError),
    Windows(WinError),
    /// A block read back differently from what was written. The controller
    /// was left held in reset; switching the unit off and on starts over.
    Mismatch {
        address: u16,
    },
}

impl fmt::Display for FirmwareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoScript(path) => write!(
                f,
                "the firmware script was not found at {}; install Fractal Audio's driver package",
                path.display()
            ),
            Self::Script(e) => write!(f, "{e}"),
            Self::Windows(e) => write!(f, "{e}"),
            Self::Mismatch { address } => write!(
                f,
                "the unit did not store the firmware block at {address:#06x} correctly; nothing was started. Switch the unit off and on to try again"
            ),
        }
    }
}

impl std::error::Error for FirmwareError {}

impl From<WinError> for FirmwareError {
    fn from(e: WinError) -> Self {
        Self::Windows(e)
    }
}

/// Where the original package keeps the script its own loader uses.
pub fn script_path() -> PathBuf {
    let windows = std::env::var("SystemRoot").unwrap_or_else(|_| String::from("C:\\Windows"));
    PathBuf::from(windows).join("System32").join("axefx2load").join("axefx2load.spt")
}

/// Reads and fully checks the script without touching any device.
pub fn read_script() -> Result<Vec<Step>, FirmwareError> {
    let path = script_path();
    let bytes = std::fs::read(&path).map_err(|_| FirmwareError::NoScript(path))?;
    loader::parse_script(&bytes).map_err(FirmwareError::Script)
}

/// What a successful load did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loaded {
    pub blocks: usize,
    pub bytes: usize,
}

/// Whether a unit is attached and waiting for its firmware.
pub fn unit_is_waiting() -> bool {
    winusb::interface_paths(LOADER_INTERFACE_GUID).is_ok_and(|p| !p.is_empty())
}

/// Loads the firmware into a waiting unit. Returns `None` when no unit is
/// waiting. The script is checked in full before the unit is touched.
pub fn load_if_waiting() -> Result<Option<Loaded>, FirmwareError> {
    let paths = winusb::interface_paths(LOADER_INTERFACE_GUID)?;
    let Some(path) = paths.first() else { return Ok(None) };
    let steps = read_script()?;
    let device = Device::open(path)?;
    let mut done = Loaded { blocks: 0, bytes: 0 };
    for step in &steps {
        match step {
            Step::Hold => device.control_out(VENDOR_OUT, REQUEST_FIRMWARE_LOAD, CPU_CONTROL, 0, &[1])?,
            Step::Run => device.control_out(VENDOR_OUT, REQUEST_FIRMWARE_LOAD, CPU_CONTROL, 0, &[0])?,
            Step::Write { address, data } => {
                device.control_out(VENDOR_OUT, REQUEST_FIRMWARE_LOAD, *address, 0, data)?;
                let stored = device.control_in(VENDOR_IN, REQUEST_FIRMWARE_LOAD, *address, 0, data.len() as u16)?;
                if stored != *data {
                    // The controller is held in reset here: the parser only
                    // allows writes while held, so nothing runs.
                    return Err(FirmwareError::Mismatch { address: *address });
                }
                done.blocks += 1;
                done.bytes += data.len();
            }
        }
    }
    Ok(Some(done))
}
