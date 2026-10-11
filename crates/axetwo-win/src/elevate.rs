//! Administrator rights: checking for them and asking for them.

use crate::{wide, WinError};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, OpenProcessToken, WaitForSingleObject, INFINITE,
};
use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};

fn last_error(what: &'static str) -> WinError {
    // SAFETY: GetLastError has no preconditions.
    let code = unsafe { GetLastError() };
    WinError { what, code }
}

/// Whether this process runs with administrator rights.
pub fn is_elevated() -> bool {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: the pseudo-handle of the current process is always valid and
    // `token` is a valid out pointer.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return false;
    }
    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut size = 0u32;
    // SAFETY: `token` is open; `elevation` is valid for the size given and
    // `size` is a valid out pointer.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            std::ptr::from_mut(&mut elevation).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        )
    };
    // SAFETY: `token` was opened above and is closed once.
    unsafe { CloseHandle(token) };
    ok != 0 && elevation.TokenIsElevated != 0
}

/// Starts `program` with `arguments` as administrator, showing Windows'
/// consent prompt, waits for it and returns its exit code.
pub fn run_elevated(program: &str, arguments: &str) -> Result<u32, WinError> {
    let (verb, file, params) = (wide("runas"), wide(program), wide(arguments));
    // SAFETY: an all-zero SHELLEXECUTEINFOW is a valid value; the fields used
    // are set below.
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = params.as_ptr();
    info.nShow = 1;
    // SAFETY: `info` is fully initialised and the strings it points to
    // outlive the call.
    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        return Err(last_error("asking for administrator rights"));
    }
    if info.hProcess.is_null() {
        return Ok(0);
    }
    let mut code = 0u32;
    // SAFETY: `hProcess` is a process handle owned by this function; it is
    // waited on, queried and closed once.
    unsafe {
        WaitForSingleObject(info.hProcess, INFINITE);
        GetExitCodeProcess(info.hProcess, &mut code);
        CloseHandle(info.hProcess);
    }
    Ok(code)
}
