//! Read-only queries through the USB hub driver.
//!
//! The hub answers these for any attached device, whether or not a driver is
//! bound to it, which makes them usable on a device Windows refuses to start.
//! Only standard `GET_DESCRIPTOR` requests are issued; nothing is written to
//! the device.

use crate::{wide, WinError};
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_Device_Interface_ListW, CM_Get_Device_Interface_List_SizeW, CM_GET_DEVICE_INTERFACE_LIST_PRESENT, CR_SUCCESS,
};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
use windows_sys::Win32::System::IO::DeviceIoControl;

/// `GUID_DEVINTERFACE_USB_HUB`
const HUB_INTERFACE: windows_sys::core::GUID = windows_sys::core::GUID {
    data1: 0xf18a_0e88,
    data2: 0xc30c,
    data3: 0x11d0,
    data4: [0x88, 0x15, 0x00, 0xa0, 0xc9, 0x06, 0xbe, 0xd8],
};

const IOCTL_USB_GET_DESCRIPTOR_FROM_NODE_CONNECTION: u32 = 0x0022_0410;
const IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX: u32 = 0x0022_0448;

/// Size of `USB_NODE_CONNECTION_INFORMATION_EX` without its pipe list.
const CONNECTION_INFO_LEN: usize = 35;
/// Size of `USB_DESCRIPTOR_REQUEST` without its data.
const DESCRIPTOR_REQUEST_LEN: usize = 12;
const MAX_PORTS: u32 = 64;

/// A device attached to a hub port.
#[derive(Debug, Clone)]
pub struct PortDevice {
    /// Device interface path of the hub.
    pub hub_path: String,
    /// One-based port number on that hub.
    pub port: u32,
    /// The 18-byte device descriptor.
    pub device_descriptor: [u8; 18],
    /// `USB_DEVICE_SPEED`: 0 low, 1 full, 2 high, 3 super.
    pub speed: u8,
    pub current_configuration: u8,
}

impl PortDevice {
    pub fn vendor_id(&self) -> u16 {
        u16::from_le_bytes([self.device_descriptor[8], self.device_descriptor[9]])
    }

    pub fn product_id(&self) -> u16 {
        u16::from_le_bytes([self.device_descriptor[10], self.device_descriptor[11]])
    }
}

struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful CreateFileW and is closed once.
        unsafe { CloseHandle(self.0) };
    }
}

fn last_error(what: &'static str) -> WinError {
    // SAFETY: GetLastError has no preconditions.
    let code = unsafe { GetLastError() };
    WinError { what, code }
}

/// Device interface paths of every USB hub present.
pub fn hub_paths() -> Result<Vec<String>, WinError> {
    let mut len: u32 = 0;
    // SAFETY: `len` is a valid out pointer and the GUID outlives the call.
    let cr = unsafe {
        CM_Get_Device_Interface_List_SizeW(
            &mut len,
            &HUB_INTERFACE,
            std::ptr::null(),
            CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
        )
    };
    if cr != CR_SUCCESS {
        return Err(WinError { what: "listing USB hubs", code: cr });
    }
    let mut buf = vec![0u16; len as usize];
    // SAFETY: `buf` holds exactly `len` UTF-16 units, as the size call asked for.
    let cr = unsafe {
        CM_Get_Device_Interface_ListW(
            &HUB_INTERFACE,
            std::ptr::null(),
            buf.as_mut_ptr(),
            len,
            CM_GET_DEVICE_INTERFACE_LIST_PRESENT,
        )
    };
    if cr != CR_SUCCESS {
        return Err(WinError { what: "listing USB hubs", code: cr });
    }
    Ok(buf.split(|&c| c == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy).collect())
}

fn open_hub(path: &str) -> Result<Handle, WinError> {
    let wpath = wide(path);
    // SAFETY: `wpath` is NUL-terminated and outlives the call; the other
    // arguments are plain values or null where the API allows null.
    let h = unsafe {
        CreateFileW(
            wpath.as_ptr(),
            GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return Err(last_error("opening a USB hub"));
    }
    Ok(Handle(h))
}

/// Sends `buf` as both input and output of a buffered IOCTL; returns the
/// number of bytes the driver wrote back.
fn ioctl(hub: &Handle, code: u32, buf: &mut [u8], what: &'static str) -> Result<usize, WinError> {
    let mut returned: u32 = 0;
    let len = buf.len() as u32;
    // SAFETY: `buf` is valid for reads and writes of `len` bytes for the
    // duration of the call, and `returned` is a valid out pointer.
    let ok = unsafe {
        DeviceIoControl(
            hub.0,
            code,
            buf.as_ptr().cast(),
            len,
            buf.as_mut_ptr().cast(),
            len,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(last_error(what));
    }
    Ok((returned as usize).min(buf.len()))
}

/// Every device attached to any hub port.
pub fn attached_devices() -> Result<Vec<PortDevice>, WinError> {
    let mut found = Vec::new();
    for hub_path in hub_paths()? {
        // A hub that cannot be opened is skipped so one bad hub does not hide the rest.
        let Ok(hub) = open_hub(&hub_path) else {
            continue;
        };
        for port in 1..=MAX_PORTS {
            let mut buf = [0u8; CONNECTION_INFO_LEN + 32 * 30];
            buf[..4].copy_from_slice(&port.to_le_bytes());
            let Ok(n) = ioctl(&hub, IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX, &mut buf, "reading a hub port")
            else {
                break;
            };
            if n < CONNECTION_INFO_LEN {
                continue;
            }
            let status = u32::from_le_bytes([buf[31], buf[32], buf[33], buf[34]]);
            // 1 is DeviceConnected.
            if status != 1 {
                continue;
            }
            let mut device_descriptor = [0u8; 18];
            device_descriptor.copy_from_slice(&buf[4..22]);
            found.push(PortDevice {
                hub_path: hub_path.clone(),
                port,
                device_descriptor,
                current_configuration: buf[22],
                speed: buf[23],
            });
        }
    }
    Ok(found)
}

/// Issues a standard `GET_DESCRIPTOR` to the device on `port` and returns the
/// bytes it answered, at most `max_len`.
pub fn get_descriptor(
    hub_path: &str,
    port: u32,
    descriptor_type: u8,
    index: u8,
    language: u16,
    max_len: u16,
) -> Result<Vec<u8>, WinError> {
    let hub = open_hub(hub_path)?;
    let mut buf = vec![0u8; DESCRIPTOR_REQUEST_LEN + max_len as usize];
    buf[..4].copy_from_slice(&port.to_le_bytes());
    // Setup packet: the hub driver fills in bmRequestType and bRequest.
    let value = (u16::from(descriptor_type) << 8) | u16::from(index);
    buf[6..8].copy_from_slice(&value.to_le_bytes());
    buf[8..10].copy_from_slice(&language.to_le_bytes());
    buf[10..12].copy_from_slice(&max_len.to_le_bytes());
    let n = ioctl(&hub, IOCTL_USB_GET_DESCRIPTOR_FROM_NODE_CONNECTION, &mut buf, "reading a descriptor")?;
    Ok(buf.get(DESCRIPTOR_REQUEST_LEN..n).unwrap_or(&[]).to_vec())
}

/// The complete configuration descriptor (all interfaces and endpoints).
pub fn configuration_descriptor(hub_path: &str, port: u32, index: u8) -> Result<Vec<u8>, WinError> {
    let head = get_descriptor(hub_path, port, 2, index, 0, 9)?;
    let total = match head.get(2..4) {
        Some(b) => u16::from_le_bytes([b[0], b[1]]),
        None => return Ok(head),
    };
    get_descriptor(hub_path, port, 2, index, 0, total.max(9))
}

/// A string descriptor decoded to text, in US English.
pub fn string_descriptor(hub_path: &str, port: u32, index: u8) -> Result<String, WinError> {
    let raw = get_descriptor(hub_path, port, 3, index, 0x0409, 255)?;
    let units: Vec<u16> =
        raw.get(2..).unwrap_or(&[]).as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
    Ok(String::from_utf16_lossy(&units))
}
