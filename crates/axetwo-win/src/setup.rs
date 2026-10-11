//! Device set-up: put Windows' own WinUSB driver on a device node.
//!
//! No driver package of our own is installed. The driver is Microsoft's
//! `winusb.inf`, which ships with Windows and is signed by Microsoft; this
//! module only selects it for one device node, the way "Update driver > Let me
//! pick from a list" does in Device Manager. Needs administrator rights.

use crate::{wide, WinError};
use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
    DiInstallDevice, SetupDiBuildDriverInfoList, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
    SetupDiEnumDriverInfoW, SetupDiGetClassDevsW, SetupDiGetDeviceInstallParamsW, SetupDiGetDeviceInstanceIdW,
    SetupDiOpenDevRegKey, SetupDiSetDeviceInstallParamsW, SetupDiSetDeviceRegistryPropertyW, SetupDiSetSelectedDriverW,
    DICS_FLAG_GLOBAL, DIGCF_ALLCLASSES, DIGCF_PRESENT, DIREG_DEV, DI_ENUMSINGLEINF, DI_FLAGSEX_ALLOWEXCLUDEDDRVS,
    HDEVINFO, SPDIT_CLASSDRIVER, SPDRP_CLASSGUID, SP_DEVINFO_DATA, SP_DEVINSTALL_PARAMS_W, SP_DRVINFO_DATA_V2_W,
};
use windows_sys::Win32::Foundation::{GetLastError, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Registry::{RegCloseKey, RegSetValueExW, HKEY, KEY_SET_VALUE, REG_MULTI_SZ};

/// The "Universal Serial Bus devices" setup class that `winusb.inf` installs into.
const USB_DEVICE_CLASS: &str = "{88BAE032-5A81-49F0-BC3D-A4FF138216D6}";
const USB_DEVICE_CLASS_GUID: windows_sys::core::GUID = windows_sys::core::GUID {
    data1: 0x88ba_e032,
    data2: 0x5a81,
    data3: 0x49f0,
    data4: [0xbc, 0x3d, 0xa4, 0xff, 0x13, 0x82, 0x16, 0xd6],
};

/// What [`bind_winusb`] did.
#[derive(Debug, Clone)]
pub struct BindReport {
    pub instance_id: String,
    /// Description of the driver that was selected.
    pub driver: String,
    /// Every driver the INF offered, for diagnostics.
    pub offered: Vec<String>,
    pub reboot_required: bool,
}

struct InfoSet(HDEVINFO);

impl Drop for InfoSet {
    fn drop(&mut self) {
        // SAFETY: the set came from a successful SetupDiGetClassDevsW and is destroyed once.
        unsafe { SetupDiDestroyDeviceInfoList(self.0) };
    }
}

fn last_error(what: &'static str) -> WinError {
    // SAFETY: GetLastError has no preconditions.
    let code = unsafe { GetLastError() };
    WinError { what, code }
}

fn text(units: &[u16]) -> String {
    let end = units.iter().position(|&c| c == 0).unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

/// Finds the present device whose instance id starts with `prefix`
/// (compared without regard to case).
fn find(prefix: &str, present_only: bool) -> Result<Option<(InfoSet, SP_DEVINFO_DATA, String)>, WinError> {
    let flags = if present_only { DIGCF_ALLCLASSES | DIGCF_PRESENT } else { DIGCF_ALLCLASSES };
    // SAFETY: null class and enumerator are allowed with DIGCF_ALLCLASSES.
    let raw = unsafe { SetupDiGetClassDevsW(std::ptr::null(), std::ptr::null(), std::ptr::null_mut(), flags) };
    if raw == INVALID_HANDLE_VALUE as HDEVINFO {
        return Err(last_error("listing devices"));
    }
    let set = InfoSet(raw);
    let wanted = prefix.to_ascii_uppercase();
    for index in 0.. {
        // SAFETY: an all-zero SP_DEVINFO_DATA is a valid value; cbSize is set below.
        let mut info: SP_DEVINFO_DATA = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
        // SAFETY: `set` is a live device information set and `info` is a valid out pointer.
        if unsafe { SetupDiEnumDeviceInfo(set.0, index, &mut info) } == 0 {
            break;
        }
        let mut id = [0u16; 512];
        // SAFETY: `id` is valid for `id.len()` UTF-16 units; the required-size pointer may be null.
        let ok = unsafe {
            SetupDiGetDeviceInstanceIdW(set.0, &info, id.as_mut_ptr(), id.len() as u32, std::ptr::null_mut())
        };
        if ok == 0 {
            continue;
        }
        let id = text(&id);
        if id.to_ascii_uppercase().starts_with(&wanted) {
            return Ok(Some((set, info, id)));
        }
    }
    Ok(None)
}

/// Instance id of the present device whose id starts with `prefix`, if any.
pub fn find_instance(prefix: &str) -> Result<Option<String>, WinError> {
    Ok(find(prefix, true)?.map(|(_, _, id)| id))
}

/// Like [`find_instance`], but also finds a device Windows remembers that is
/// not attached right now.
pub fn find_known_instance(prefix: &str) -> Result<Option<String>, WinError> {
    Ok(find(prefix, false)?.map(|(_, _, id)| id))
}

/// Writes `DeviceInterfaceGUIDs` into the device's hardware key so WinUSB
/// exposes the device under `interface_guid` (in `{...}` form).
fn set_interface_guid(set: &InfoSet, info: &SP_DEVINFO_DATA, interface_guid: &str) -> Result<(), WinError> {
    // SAFETY: `set` and `info` describe a live device node.
    let key: HKEY = unsafe { SetupDiOpenDevRegKey(set.0, info, DICS_FLAG_GLOBAL, 0, DIREG_DEV, KEY_SET_VALUE) };
    if std::ptr::eq(key, INVALID_HANDLE_VALUE) {
        return Err(last_error("opening the device's registry key"));
    }
    // REG_MULTI_SZ: one string, then an empty string.
    let mut value = wide(interface_guid);
    value.push(0);
    let name = wide("DeviceInterfaceGUIDs");
    // SAFETY: `name` is NUL-terminated; `value` is valid for the byte count given.
    let status =
        unsafe { RegSetValueExW(key, name.as_ptr(), 0, REG_MULTI_SZ, value.as_ptr().cast(), (value.len() * 2) as u32) };
    // SAFETY: `key` was opened above and is closed once.
    unsafe { RegCloseKey(key) };
    if status != 0 {
        return Err(WinError { what: "writing DeviceInterfaceGUIDs", code: status });
    }
    Ok(())
}

/// Puts Microsoft's WinUSB driver on the present device whose instance id
/// starts with `instance_prefix`, exposed under `interface_guid`.
pub fn bind_winusb(instance_prefix: &str, interface_guid: &str) -> Result<BindReport, WinError> {
    bind(instance_prefix, interface_guid, true)
}

/// Like [`bind_winusb`], for a device Windows remembers whether or not it is
/// attached right now. Used for the unit's short-lived power-on identity.
pub fn bind_winusb_known(instance_prefix: &str, interface_guid: &str) -> Result<BindReport, WinError> {
    bind(instance_prefix, interface_guid, false)
}

fn bind(instance_prefix: &str, interface_guid: &str, present_only: bool) -> Result<BindReport, WinError> {
    let Some((set, mut info, instance_id)) = find(instance_prefix, present_only)? else {
        return Err(WinError { what: "finding the device (is it connected and switched on?)", code: 0 });
    };
    set_interface_guid(&set, &info, interface_guid)?;

    // A class driver list only offers drivers of the device's own setup class,
    // so the device is moved to the class winusb.inf installs into first.
    let class = wide(USB_DEVICE_CLASS);
    // SAFETY: `set`/`info` are live; `class` is valid for the byte count given.
    let ok = unsafe {
        SetupDiSetDeviceRegistryPropertyW(
            set.0,
            &mut info,
            SPDRP_CLASSGUID,
            class.as_ptr().cast(),
            (class.len() * 2) as u32,
        )
    };
    if ok == 0 {
        return Err(last_error("setting the device class (administrator rights are needed)"));
    }
    info.ClassGuid = USB_DEVICE_CLASS_GUID;

    // SAFETY: an all-zero SP_DEVINSTALL_PARAMS_W is a valid value; cbSize is set below.
    let mut params: SP_DEVINSTALL_PARAMS_W = unsafe { std::mem::zeroed() };
    params.cbSize = std::mem::size_of::<SP_DEVINSTALL_PARAMS_W>() as u32;
    // SAFETY: `set`/`info` are live and `params` is a correctly sized out structure.
    if unsafe { SetupDiGetDeviceInstallParamsW(set.0, &info, &mut params) } == 0 {
        return Err(last_error("reading install parameters"));
    }
    let windir = std::env::var("SystemRoot").unwrap_or_else(|_| String::from("C:\\Windows"));
    let inf = wide(&format!("{windir}\\INF\\winusb.inf"));
    if inf.len() > params.DriverPath.len() {
        return Err(WinError { what: "building the path of winusb.inf", code: 0 });
    }
    params.DriverPath[..inf.len()].copy_from_slice(&inf);
    params.Flags |= DI_ENUMSINGLEINF;
    params.FlagsEx |= DI_FLAGSEX_ALLOWEXCLUDEDDRVS;
    // SAFETY: as above; `params` is fully initialised.
    if unsafe { SetupDiSetDeviceInstallParamsW(set.0, &info, &params) } == 0 {
        return Err(last_error("setting install parameters"));
    }
    // SAFETY: `set`/`info` are live.
    if unsafe { SetupDiBuildDriverInfoList(set.0, &mut info, SPDIT_CLASSDRIVER) } == 0 {
        return Err(last_error("reading winusb.inf"));
    }

    let mut offered = Vec::new();
    let mut chosen: Option<SP_DRVINFO_DATA_V2_W> = None;
    for index in 0.. {
        // SAFETY: an all-zero SP_DRVINFO_DATA_V2_W is a valid value; cbSize is set below.
        let mut driver: SP_DRVINFO_DATA_V2_W = unsafe { std::mem::zeroed() };
        driver.cbSize = std::mem::size_of::<SP_DRVINFO_DATA_V2_W>() as u32;
        // SAFETY: `set`/`info` are live and `driver` is a correctly sized out structure.
        if unsafe { SetupDiEnumDriverInfoW(set.0, &info, SPDIT_CLASSDRIVER, index, &mut driver) } == 0 {
            break;
        }
        let description = text(&driver.Description);
        if chosen.is_none() && description.to_ascii_lowercase().contains("winusb") {
            chosen = Some(driver);
        }
        offered.push(description);
    }
    let Some(mut driver) = chosen else {
        return Err(WinError { what: "finding the WinUSB driver in winusb.inf", code: 0 });
    };
    // SAFETY: `driver` was returned by SetupDiEnumDriverInfoW for this set and device.
    if unsafe { SetupDiSetSelectedDriverW(set.0, &mut info, &mut driver) } == 0 {
        return Err(last_error("selecting the WinUSB driver"));
    }
    let mut reboot = 0;
    // SAFETY: all pointers refer to live values owned by this function; a null
    // parent window is allowed.
    let ok = unsafe { DiInstallDevice(std::ptr::null_mut(), set.0, &info, &driver, 0, &mut reboot) };
    if ok == 0 {
        return Err(last_error("installing the WinUSB driver (administrator rights are needed)"));
    }
    Ok(BindReport { instance_id, driver: text(&driver.Description), offered, reboot_required: reboot != 0 })
}
