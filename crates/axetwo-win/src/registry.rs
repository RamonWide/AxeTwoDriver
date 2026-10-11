//! Small typed access to the Windows registry.

use crate::{wide, WinError};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegGetValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_WRITE, REG_DWORD, REG_SZ, RRF_RT_REG_DWORD,
};

/// Which part of the registry a path is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Root {
    /// Settings of the signed-in user. Writable without administrator rights.
    CurrentUser,
    /// Settings of the machine. Writing needs administrator rights.
    LocalMachine,
}

impl Root {
    fn key(self) -> HKEY {
        match self {
            Self::CurrentUser => HKEY_CURRENT_USER,
            Self::LocalMachine => HKEY_LOCAL_MACHINE,
        }
    }
}

const ERROR_FILE_NOT_FOUND: u32 = 2;

/// Reads a 32-bit value, or `None` when it is missing or of another type.
pub fn get_dword(root: Root, path: &str, name: &str) -> Option<u32> {
    let (wpath, wname) = (wide(path), wide(name));
    let mut value: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: both strings are NUL-terminated; `value` is valid for `size`
    // bytes and `size` is a valid in/out pointer.
    let status = unsafe {
        RegGetValueW(
            root.key(),
            wpath.as_ptr(),
            wname.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            std::ptr::from_mut(&mut value).cast(),
            &mut size,
        )
    };
    (status == 0).then_some(value)
}

/// Creates `path` if needed and runs `set` with the open key.
fn with_key(root: Root, path: &str, set: impl FnOnce(HKEY) -> u32) -> Result<(), WinError> {
    let wpath = wide(path);
    let mut key: HKEY = std::ptr::null_mut();
    // SAFETY: `wpath` is NUL-terminated and `key` is a valid out pointer; the
    // other arguments are plain values or null where the API allows null.
    let status = unsafe {
        RegCreateKeyExW(
            root.key(),
            wpath.as_ptr(),
            0,
            std::ptr::null(),
            0,
            KEY_WRITE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if status != 0 {
        return Err(WinError { what: "opening a registry key for writing", code: status });
    }
    let status = set(key);
    // SAFETY: `key` was opened above and is closed once.
    unsafe { RegCloseKey(key) };
    if status != 0 {
        return Err(WinError { what: "writing a registry value", code: status });
    }
    Ok(())
}

/// Writes a 32-bit value, creating the key if needed.
pub fn set_dword(root: Root, path: &str, name: &str, value: u32) -> Result<(), WinError> {
    let wname = wide(name);
    with_key(root, path, |key| {
        // SAFETY: `key` is open, `wname` is NUL-terminated and `value` is
        // valid for the four bytes given.
        unsafe { RegSetValueExW(key, wname.as_ptr(), 0, REG_DWORD, std::ptr::from_ref(&value).cast(), 4) }
    })
}

/// Writes a string value, creating the key if needed. An empty `name` sets
/// the key's default value.
pub fn set_string(root: Root, path: &str, name: &str, value: &str) -> Result<(), WinError> {
    let (wname, wvalue) = (wide(name), wide(value));
    with_key(root, path, |key| {
        // SAFETY: `key` is open, both strings are NUL-terminated and the byte
        // count covers `wvalue` including its terminator.
        unsafe { RegSetValueExW(key, wname.as_ptr(), 0, REG_SZ, wvalue.as_ptr().cast(), (wvalue.len() * 2) as u32) }
    })
}

/// Deletes a key and everything under it. A key that is not there is fine.
pub fn delete_tree(root: Root, path: &str) -> Result<(), WinError> {
    let wpath = wide(path);
    // SAFETY: `wpath` is NUL-terminated.
    let status = unsafe { RegDeleteTreeW(root.key(), wpath.as_ptr()) };
    if status != 0 && status != ERROR_FILE_NOT_FOUND {
        return Err(WinError { what: "deleting a registry key", code: status });
    }
    Ok(())
}
