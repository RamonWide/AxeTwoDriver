//! Windows system calls used by AxeTwoDriver.
//!
//! This is the only crate in the workspace that contains `unsafe`. Everything
//! it returns is plain owned data, so the crates above it stay safe.

#![cfg(windows)]

pub mod elevate;
pub mod hub;
pub mod registry;
pub mod setup;
pub mod winusb;

use std::fmt;

/// A failed system call: what was being attempted and the Win32 error code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinError {
    pub what: &'static str,
    pub code: u32,
}

impl fmt::Display for WinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} failed (Windows error {})", self.what, self.code)
    }
}

impl std::error::Error for WinError {}

pub(crate) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
